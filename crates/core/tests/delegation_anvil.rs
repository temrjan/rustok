//! End-to-end delegation lifecycle on a local Anvil (`--hardfork prague`).
//!
//! Ignored by default — requires the `anvil` binary on PATH. Run with:
//!
//! ```sh
//! cargo test -p rustok-core --test delegation_anvil -- --include-ignored
//! ```
//!
//! The pinned delegate's runtime bytecode is injected into Anvil via
//! `anvil_setCode` from `tests/fixtures/simple7702account_v09_runtime.hex`
//! (fetched from Ethereum mainnet 2026-08-20; the test re-verifies its
//! keccak against `DELEGATE_CODE_HASH`, so the fixture doubles as a pin
//! self-check). Anvil runs with `--chain-id 1` so the delegate matrix guard
//! (`supports_eip7702`) applies exactly as on mainnet.

use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use alloy_primitives::{Address, B256, Bytes, U256, address, keccak256};
use alloy_signer_local::PrivateKeySigner;
use rustok_core::account::delegate::{DELEGATE_ADDRESS, DELEGATE_CODE_HASH};
use rustok_core::account::delegation::{Delegation, DelegationState};
use rustok_core::account::journal::Journal;
use rustok_core::account::{Call, Executor, OperationStatus, SubmissionPath};
use rustok_core::provider::{Chain, MultiProvider};

/// Anvil dev account #0 (public, well-known foundry test key — never holds
/// real funds).
const ANVIL_KEY: &str = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";

/// Runtime bytecode of the pinned `Simple7702AccountV09` delegate.
const DELEGATE_RUNTIME: &str = include_str!("fixtures/simple7702account_v09_runtime.hex");

/// Kill the Anvil process even if the test panics.
struct AnvilGuard(Child);

impl Drop for AnvilGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn find_anvil() -> Option<&'static str> {
    // `which`-style lookup without an extra dependency.
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|dir| dir.join("anvil"))
            .find(|candidate| candidate.is_file())
            .map(|_| "anvil")
    })
}

async fn rpc(
    http: &reqwest::Client,
    url: &str,
    method: &str,
    params: serde_json::Value,
) -> serde_json::Value {
    let response: serde_json::Value = http
        .post(url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        }))
        .send()
        .await
        .expect("rpc send")
        .json()
        .await
        .expect("rpc json");
    if let Some(error) = response.get("error") {
        panic!("{method} failed: {error}");
    }
    response["result"].clone()
}

/// Fresh `eth_getCode`-backed state with a short retry — Anvil automines,
/// but the code update is only visible after the block is sealed.
async fn wait_state(
    delegation: &Delegation<'_>,
    account: Address,
    expected: DelegationState,
) -> DelegationState {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let state = delegation.state(1, account).await.expect("state");
        if state == expected || Instant::now() > deadline {
            return state;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[tokio::test]
#[ignore = "requires the anvil binary on PATH"]
async fn delegation_lifecycle_on_anvil() {
    if find_anvil().is_none() {
        eprintln!("anvil not found on PATH — skipping (install foundry to run)");
        return;
    }

    // The fixture must be the pinned delegate code — this is the pin
    // self-check, run even if everything downstream failed early.
    let runtime_hex = DELEGATE_RUNTIME.trim();
    let runtime: Bytes = runtime_hex.parse().expect("fixture hex parses");
    assert_eq!(
        keccak256(&runtime),
        DELEGATE_CODE_HASH,
        "fixture bytecode must match DELEGATE_CODE_HASH"
    );

    let port = TcpListener::bind("127.0.0.1:0")
        .expect("bind ephemeral port")
        .local_addr()
        .expect("local addr")
        .port();
    let url = format!("http://127.0.0.1:{port}");

    let child = Command::new("anvil")
        .args([
            "--hardfork",
            "prague",
            "--chain-id",
            "1",
            "--port",
            &port.to_string(),
            "--silent",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn anvil");
    let _guard = AnvilGuard(child);

    let http = reqwest::Client::new();
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if let Ok(resp) = http
            .post(&url)
            .json(&serde_json::json!({"jsonrpc":"2.0","id":1,"method":"eth_chainId","params":[]}))
            .send()
            .await
        {
            if resp.status().is_success() {
                break;
            }
        }
        assert!(Instant::now() < deadline, "anvil did not start in time");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Inject the pinned delegate code at the pinned address.
    rpc(
        &http,
        &url,
        "anvil_setCode",
        serde_json::json!([format!("{DELEGATE_ADDRESS:?}"), runtime_hex]),
    )
    .await;

    let chain = Chain {
        id: 1,
        name: "Anvil (chain-id 1)".into(),
        rpc_urls: vec![url.clone()],
        explorer_url: String::new(),
        native_symbol: "ETH".into(),
        native_decimals: 18,
        testnet: true,
        slug: "anvil",
    };
    let provider = MultiProvider::new(vec![chain]);
    let delegation = Delegation::new(&provider);
    let signer: PrivateKeySigner = ANVIL_KEY.parse().expect("anvil key parses");
    let account = signer.address();
    assert_eq!(
        account,
        address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266"),
        "anvil dev account #0"
    );

    // 1. Fresh account: no delegation.
    assert_eq!(
        delegation.state(1, account).await.expect("state"),
        DelegationState::None
    );

    // 2. Authorize the pinned delegate.
    let hash = delegation
        .authorize(&signer, 1)
        .await
        .expect("authorize")
        .expect("fresh account must broadcast, not no-op");
    assert_ne!(hash, B256::ZERO);
    // Success is proven by eth_getCode, not by the receipt (ADR-001 §10).
    assert_eq!(
        wait_state(&delegation, account, DelegationState::Ours).await,
        DelegationState::Ours,
        "after authorize the code must be 0xef0100‖DELEGATE_ADDRESS"
    );

    // 3. Second authorize is a no-op (invariant §5.2.4).
    assert_eq!(
        delegation
            .authorize(&signer, 1)
            .await
            .expect("re-authorize"),
        None,
        "already delegated must not send another transaction"
    );

    // 4. Multi-call operation goes through DirectSelfCall (executeBatch).
    let dir = tempfile::tempdir().expect("tempdir");
    let journal = Journal::open(dir.path().join("journal.db")).expect("journal");
    let executor = Executor::new(&provider, &journal);
    let recipient = address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045");
    let balance_before: U256 = {
        let hex = rpc(
            &http,
            &url,
            "eth_getBalance",
            serde_json::json!([format!("{recipient:?}"), "latest"]),
        )
        .await;
        U256::from_str_radix(hex.as_str().unwrap().trim_start_matches("0x"), 16).unwrap()
    };
    let calls = vec![
        Call {
            to: recipient,
            value: U256::from(1u64),
            data: Bytes::new(),
        },
        Call {
            to: recipient,
            value: U256::from(2u64),
            data: Bytes::new(),
        },
    ];
    let op = executor
        .execute(&signer, 1, calls)
        .await
        .expect("batch execute");
    assert_eq!(op.path, SubmissionPath::DirectSelfCall);
    assert_eq!(op.status, OperationStatus::Broadcast);

    let confirmed = executor
        .status(&op.id)
        .await
        .expect("status")
        .expect("known id");
    assert_eq!(confirmed.status, OperationStatus::Confirmed);
    let balance_after: U256 = {
        let hex = rpc(
            &http,
            &url,
            "eth_getBalance",
            serde_json::json!([format!("{recipient:?}"), "latest"]),
        )
        .await;
        U256::from_str_radix(hex.as_str().unwrap().trim_start_matches("0x"), 16).unwrap()
    };
    assert_eq!(
        balance_after - balance_before,
        U256::from(3u64),
        "both batched calls must land atomically in one transaction"
    );

    // 5. Revoke — the code is cleared, the EOA path is back.
    delegation.revoke(&signer, 1).await.expect("revoke");
    assert_eq!(
        wait_state(&delegation, account, DelegationState::None).await,
        DelegationState::None,
        "after revoke eth_getCode must be empty"
    );

    // 6. Single call after revoke still works via DirectEoa.
    let op = executor
        .execute(
            &signer,
            1,
            vec![Call {
                to: recipient,
                value: U256::from(5u64),
                data: Bytes::new(),
            }],
        )
        .await
        .expect("single call after revoke");
    assert_eq!(op.path, SubmissionPath::DirectEoa);
}
