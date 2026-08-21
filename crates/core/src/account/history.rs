//! Merged operation history (ТЗ §5, circle 1: «слияние истории —
//! обязательная часть журнала, без этого шага история врёт»).
//!
//! One feed from two sources: the local [`Journal`] (knows our operations
//! immediately — pending broadcasts, on-chain reverts, batches whose real
//! effect is invisible in an explorer `txlist`) and the explorer `txlist`
//! (knows everything else, e.g. incoming transfers). Deduplication is by
//! transaction hash; on conflict the journal's status wins for operations
//! it tracks more freshly (`Broadcast` → "pending" even if the explorer
//! already shows the hash, `Failed` → "failed").
//!
//! Known limitation (accepted in the PR-3 spec): an operation stuck in
//! `Broadcast` (transaction dropped from the mempool without a receipt)
//! shows as "pending" indefinitely — the `Dropped` transition and a
//! reconciliation policy are intentionally out of PR-3 scope.

use rustok_types::TransactionDto;

use super::{Operation, OperationStatus};
use crate::explorer::format_time_ago;
use crate::provider::{Chain, format_wei};

/// Merge one chain's journal operations with its explorer transactions
/// into a single feed, newest first, capped at `limit`.
///
/// `journal_ops` and `explorer_txs` must both belong to `chain` (callers
/// use [`super::journal::Journal::list_by_chain`] and filter the explorer
/// result by `chain_id`); entries from other chains are ignored.
///
/// Journal entries are shown for `Broadcast` / `Confirmed` / `Failed`
/// operations; `Draft` (recorded, never broadcast) is hidden — it is a
/// transient state with nothing to show the user.
#[must_use]
pub fn merge_history(
    journal_ops: &[Operation],
    explorer_txs: &[TransactionDto],
    chain: &Chain,
    limit: u32,
) -> Vec<TransactionDto> {
    let mut consumed: Vec<bool> = vec![false; explorer_txs.len()];
    let mut merged: Vec<TransactionDto> = Vec::new();

    for op in journal_ops {
        if op.chain_id != chain.id || op.status == OperationStatus::Draft {
            continue;
        }

        let hash = op.tx_hash.map(|h| h.to_string().to_lowercase());
        let matched = hash.as_deref().and_then(|h| {
            explorer_txs.iter().enumerate().find(|(i, tx)| {
                !consumed[*i] && tx.chain_id == chain.id && tx.tx_hash.to_lowercase() == h
            })
        });

        if let Some((i, tx)) = matched {
            consumed[i] = true;
            let mut entry = tx.clone();
            // The journal is fresher for our own operations: a receipt the
            // poller has not seen yet keeps the entry pending; an on-chain
            // revert is failed even if the explorer has not indexed it.
            match op.status {
                OperationStatus::Broadcast => entry.status = "pending".into(),
                OperationStatus::Failed => entry.status = "failed".into(),
                _ => {}
            }
            merged.push(entry);
        } else {
            merged.push(operation_entry(op, chain));
        }
    }

    for (i, tx) in explorer_txs.iter().enumerate() {
        if !consumed[i] && tx.chain_id == chain.id {
            merged.push(tx.clone());
        }
    }

    merged.sort_by_key(|tx| std::cmp::Reverse(tx.timestamp));
    merged.truncate(limit as usize);
    merged
}

/// Synthesize a history entry from a journaled operation that has no
/// explorer counterpart (pending broadcast, recent confirmation, batch).
fn operation_entry(op: &Operation, chain: &Chain) -> TransactionDto {
    let tx_hash = op.tx_hash.map(|h| h.to_string()).unwrap_or_default();
    let status = match op.status {
        OperationStatus::Broadcast => "pending",
        OperationStatus::Confirmed => "confirmed",
        OperationStatus::Failed => "failed",
        // Filtered by the caller; unreachable here.
        OperationStatus::Draft | OperationStatus::Dropped => "pending",
    };

    // A single call displays like a plain transfer. A batch's carrier is a
    // zero-value self-call — display the batch shape instead of the
    // misleading carrier.
    let (to, value_formatted, direction) = if op.calls.len() == 1 {
        let call = &op.calls[0];
        (
            call.to.to_string(),
            format!(
                "{} {}",
                format_wei(call.value, chain.native_decimals),
                chain.native_symbol
            ),
            if call.to == op.from { "self" } else { "sent" },
        )
    } else {
        (
            op.from.to_string(),
            format!("Batch of {} calls", op.calls.len()),
            "self",
        )
    };

    TransactionDto {
        explorer_url: if tx_hash.is_empty() {
            String::new()
        } else {
            format!("{}/tx/{tx_hash}", chain.explorer_url)
        },
        tx_hash,
        chain_id: op.chain_id,
        chain_name: chain.name.clone(),
        from: op.from.to_string(),
        to,
        value_formatted,
        timestamp: op.created_at,
        time_ago: format_time_ago(op.created_at),
        direction: direction.into(),
        status: status.into(),
        block_number: op.block_number.unwrap_or(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::journal::Journal;
    use crate::account::{Call, SubmissionPath};
    use alloy_primitives::{Address, Bytes, U256, address, b256};

    fn test_chain() -> Chain {
        Chain {
            id: 11155111,
            name: "Sepolia".into(),
            rpc_urls: vec![],
            explorer_url: "https://sepolia.etherscan.io".into(),
            native_symbol: "ETH".into(),
            native_decimals: 18,
            testnet: true,
            slug: "sepolia",
        }
    }

    fn from_addr() -> Address {
        address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266")
    }

    fn call(value: u64) -> Call {
        Call {
            to: address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045"),
            value: U256::from(value),
            data: Bytes::new(),
        }
    }

    fn explorer_tx(hash: &str, timestamp: u64) -> TransactionDto {
        TransactionDto {
            tx_hash: hash.into(),
            chain_id: 11155111,
            chain_name: "Sepolia".into(),
            from: from_addr().to_string(),
            to: "0xrecipient".into(),
            value_formatted: "0.5 ETH".into(),
            timestamp,
            time_ago: "1h ago".into(),
            direction: "sent".into(),
            status: "confirmed".into(),
            block_number: 100,
            explorer_url: format!("https://sepolia.etherscan.io/tx/{hash}"),
        }
    }

    fn journaled_op(journal: &Journal, value: u64) -> Operation {
        journal
            .insert_draft(
                11155111,
                from_addr(),
                &[call(value)],
                SubmissionPath::DirectEoa,
            )
            .unwrap()
    }

    #[test]
    fn broadcast_operation_appears_pending_without_explorer_row() {
        let journal = Journal::open_in_memory().unwrap();
        let op = journaled_op(&journal, 1);
        let hash = b256!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        let op = journal.mark_broadcast(&op.id, hash).unwrap();

        let merged = merge_history(&[op], &[], &test_chain(), 20);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].status, "pending");
        assert_eq!(merged[0].tx_hash, hash.to_string());
        assert_eq!(merged[0].direction, "sent");
        assert!(merged[0].value_formatted.ends_with("ETH"));
    }

    #[test]
    fn same_hash_dedupes_and_journal_marks_pending() {
        let journal = Journal::open_in_memory().unwrap();
        let op = journaled_op(&journal, 1);
        let hash = b256!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        let op = journal.mark_broadcast(&op.id, hash).unwrap();

        // Explorer already shows the hash as confirmed (it can lag behind
        // finality) — the journal's fresher Broadcast state wins.
        let explorer = vec![explorer_tx(&hash.to_string(), 999)];
        let merged = merge_history(std::slice::from_ref(&op), &explorer, &test_chain(), 20);

        assert_eq!(merged.len(), 1, "no duplicate by hash");
        assert_eq!(merged[0].status, "pending");
        // Display fields come from the explorer row.
        assert_eq!(merged[0].value_formatted, "0.5 ETH");
        assert_eq!(merged[0].timestamp, 999);
    }

    #[test]
    fn confirmed_operation_uses_explorer_status() {
        let journal = Journal::open_in_memory().unwrap();
        let op = journaled_op(&journal, 1);
        let hash = b256!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        let op = journal.mark_broadcast(&op.id, hash).unwrap();
        let op = journal.mark_confirmed(&op.id, 100).unwrap();

        let explorer = vec![explorer_tx(&hash.to_string(), 999)];
        let merged = merge_history(std::slice::from_ref(&op), &explorer, &test_chain(), 20);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].status, "confirmed");
    }

    #[test]
    fn reverted_operation_is_failed_even_if_explorer_says_confirmed() {
        let journal = Journal::open_in_memory().unwrap();
        let op = journaled_op(&journal, 1);
        let hash = b256!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        let op = journal.mark_broadcast(&op.id, hash).unwrap();
        let op = journal
            .mark_failed(&op.id, "transaction reverted on-chain")
            .unwrap();

        let explorer = vec![explorer_tx(&hash.to_string(), 999)];
        let merged = merge_history(std::slice::from_ref(&op), &explorer, &test_chain(), 20);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].status, "failed");
    }

    #[test]
    fn failed_broadcast_without_hash_is_shown_as_failed() {
        let journal = Journal::open_in_memory().unwrap();
        let op = journaled_op(&journal, 1);
        let op = journal.mark_failed(&op.id, "rpc down").unwrap();
        assert!(op.tx_hash.is_none());

        let merged = merge_history(&[op], &[], &test_chain(), 20);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].status, "failed");
        assert!(merged[0].tx_hash.is_empty());
        assert!(merged[0].explorer_url.is_empty());
    }

    #[test]
    fn draft_operations_are_hidden() {
        let journal = Journal::open_in_memory().unwrap();
        let op = journaled_op(&journal, 1);
        assert_eq!(op.status, OperationStatus::Draft);

        let merged = merge_history(&[op], &[], &test_chain(), 20);

        assert!(merged.is_empty());
    }

    #[test]
    fn batch_operation_displays_as_batch() {
        let journal = Journal::open_in_memory().unwrap();
        let op = journal
            .insert_draft(
                11155111,
                from_addr(),
                &[call(1), call(2)],
                SubmissionPath::DirectSelfCall,
            )
            .unwrap();
        let hash = b256!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
        let op = journal.mark_broadcast(&op.id, hash).unwrap();

        let merged = merge_history(&[op], &[], &test_chain(), 20);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].value_formatted, "Batch of 2 calls");
        assert_eq!(merged[0].direction, "self");
        assert_eq!(merged[0].status, "pending");
    }

    #[test]
    fn explorer_only_and_other_chain_entries() {
        let journal = Journal::open_in_memory().unwrap();
        let op = journaled_op(&journal, 1);
        let hash = b256!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        let op = journal.mark_broadcast(&op.id, hash).unwrap();

        let mut other_chain = explorer_tx("0xcccc", 500);
        other_chain.chain_id = 1;
        // Explorer row newer than the pending op (created "now").
        let explorer = vec![explorer_tx("0xdddd", op.created_at + 1000), other_chain];

        let merged = merge_history(std::slice::from_ref(&op), &explorer, &test_chain(), 20);

        assert_eq!(merged.len(), 2, "journal op + own-chain explorer row");
        assert!(merged.iter().all(|tx| tx.chain_id == 11155111));
        // Newest first: explorer row (1000) before the pending op.
        assert_eq!(merged[0].tx_hash, "0xdddd");
        assert_eq!(merged[1].status, "pending");
    }

    #[test]
    fn limit_truncates_after_merge() {
        let journal = Journal::open_in_memory().unwrap();
        let op = journaled_op(&journal, 1);
        let hash = b256!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        let op = journal.mark_broadcast(&op.id, hash).unwrap();

        let explorer = vec![explorer_tx("0xdddd", u64::MAX)];
        let merged = merge_history(std::slice::from_ref(&op), &explorer, &test_chain(), 1);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].tx_hash, "0xdddd");
    }
}
