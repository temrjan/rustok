//! Pinned EIP-7702 delegate and the 7702 network matrix (ADR-001 §6).
//!
//! The delegate is `Simple7702AccountV09` (Ethereum Foundation Account
//! Abstraction Team; Cantina audit for the v0.9 contracts, Spearbit for the
//! v0.8 line) targeting EntryPoint v0.9. Chosen 2026-08-20 after the
//! landscape re-check ADR-001 §6 requires at circle 1 start: EntryPoint v0.9
//! is ABI-compatible with v0.8, and Candide recommends `Simple7702AccountV09`
//! for new integrations.
//!
//! Verified on-chain 2026-08-20 (`eth_getCode` on every network below,
//! control networks double-checked through independent RPC providers):
//! - the delegate has IDENTICAL 3747-byte runtime code on all six networks —
//!   [`DELEGATE_CODE_HASH`] holds everywhere;
//! - EntryPoint v0.9 runtime code DIFFERS between chains (same length,
//!   different keccak) — it is pinned by address only, never by code hash;
//! - the EP v0.9 address is embedded in the delegate bytecode as a constant.
//!
//! ABI selectors confirmed against the deployed bytecode:
//! `execute(address,uint256,bytes)` = `0xb61d27f6`,
//! `executeBatch((address,uint256,bytes)[])` = `0x34fcd5be`,
//! `entryPoint()` = `0xb0d691fe`.

use alloy_primitives::{Address, B256, address, b256};
use alloy_sol_types::sol;

/// Pinned delegate address — the ONLY address this wallet ever signs an
/// EIP-7702 authorization for (invariant ADR-001 §5.2.1). Deployed at the
/// same address on every supported network.
pub const DELEGATE_ADDRESS: Address = address!("a46cc63eBF4Bd77888AA327837d20b23A63a56B5");

/// keccak256 of the delegate runtime bytecode — identical on all six
/// supported networks. Checked before every authorization: a matching
/// address with different code means a compromised lookalike deployment.
pub const DELEGATE_CODE_HASH: B256 =
    b256!("c443c078781889c491bf2b29230504c029e910018b6be8ee67f4e36422b9d8cb");

/// EntryPoint v0.9. Pinned by address only: the deployed runtime code
/// differs between chains (measured 2026-08-20). The delegate itself
/// references this address as a bytecode constant.
pub const ENTRY_POINT_V09: Address = address!("433709009B8330FDa32311DF1C2AFA402eD8D009");

/// Chains with working EIP-7702 (Pectra-class rules) AND a verified pinned
/// delegate deployment. zkSync Era (324) is excluded by the Captain's
/// 2026-08-19 decision (no 7702); unknown chains default to `false`.
pub const EIP7702_CHAINS: &[u64] = &[
    1,        // Ethereum mainnet (Pectra 2025-05-07)
    11155111, // Sepolia
    42161,    // Arbitrum One (ArbOS 40)
    421614,   // Arbitrum Sepolia
    8453,     // Base (Isthmus)
    10,       // Optimism (Isthmus)
];

/// Whether EIP-7702 delegation is supported and the pinned delegate is
/// deployed on `chain_id`.
#[must_use]
pub const fn supports_eip7702(chain_id: u64) -> bool {
    let mut i = 0;
    while i < EIP7702_CHAINS.len() {
        if EIP7702_CHAINS[i] == chain_id {
            return true;
        }
        i += 1;
    }
    false
}

sol! {
    /// Call shape of `Simple7702Account(V09)::executeBatch`
    /// (`(address,uint256,bytes)[]`, selector `0x34fcd5be`).
    #[derive(Debug, PartialEq, Eq)]
    struct DelegateCall {
        address target;
        uint256 value;
        bytes data;
    }

    /// Atomic batch — `0x34fcd5be`. The only delegate entry point the
    /// wallet's direct path uses (`DirectSelfCall`); `execute` /
    /// `entryPoint` are intentionally not declared until a caller needs
    /// them.
    #[derive(Debug, PartialEq, Eq)]
    function executeBatch(DelegateCall[] calls) external;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::MultiProvider;

    #[test]
    fn matrix_covers_every_default_chain_except_zksync() {
        for chain in MultiProvider::default_chains().chains() {
            if chain.id == 324 {
                // zkSync Era — excluded by the Captain's 2026-08-19 decision.
                assert!(!supports_eip7702(chain.id), "zkSync must stay excluded");
            } else {
                assert!(
                    supports_eip7702(chain.id),
                    "default chain {} ({}) must be in the 7702 matrix",
                    chain.id,
                    chain.name
                );
            }
        }
    }

    #[test]
    fn unknown_chains_are_not_supported() {
        assert!(!supports_eip7702(0));
        assert!(!supports_eip7702(324));
        assert!(!supports_eip7702(999_999_999));
    }

    #[test]
    fn delegate_pin_constants_are_well_formed() {
        assert_ne!(DELEGATE_ADDRESS, Address::ZERO);
        assert_ne!(ENTRY_POINT_V09, Address::ZERO);
        assert_ne!(DELEGATE_ADDRESS, ENTRY_POINT_V09);
        assert_ne!(DELEGATE_CODE_HASH, B256::ZERO);
        // The delegate embeds the EP v0.9 address — sanity-check the pair
        // we verified on-chain stays in sync with the constants.
        assert_eq!(
            ENTRY_POINT_V09,
            address!("433709009B8330FDa32311DF1C2AFA402eD8D009")
        );
    }
}
