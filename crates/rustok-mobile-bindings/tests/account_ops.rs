//! Integration tests — account operations / delegation surface (PR-3).
//!
//! Covers the no-RPC slice: DTO parsing at the FFI boundary, error
//! mapping of the new `Account` kind, and the journal-backed
//! `get_operation_status` lookup. Broadcast paths (authorize/revoke/
//! execute against a live chain) are covered by core's own tests and
//! the ignored anvil suites — no network here.

use rustok_mobile_bindings::{
    AccountErrorKind, BindingsError, CallDto, EncodingErrorKind, OperationDto, WalletErrorKind,
    WalletHandle,
};

const PASSWORD: &str = "test-password-123";

fn handle() -> (tempfile::TempDir, std::sync::Arc<WalletHandle>) {
    let dir = tempfile::tempdir().expect("tempdir");
    let h = WalletHandle::new(dir.path().to_string_lossy().into_owned()).expect("constructor");
    (dir, h)
}

async fn unlocked_handle() -> (tempfile::TempDir, std::sync::Arc<WalletHandle>) {
    let (dir, h) = handle();
    h.create_wallet(PASSWORD.into()).await.expect("create");
    (dir, h)
}

fn valid_call() -> CallDto {
    CallDto {
        to: "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045".into(),
        value_wei: "1000000000000000".into(),
        data: "0x".into(),
    }
}

// ─── CallDto parsing ───────────────────────────────────────────

#[test]
fn call_dto_empty_data_variants_parse_to_empty_calldata() {
    for data in ["", "0x"] {
        let call = CallDto {
            data: data.into(),
            ..valid_call()
        };
        let core = call.into_core().expect("empty data must parse");
        assert!(core.data.is_empty(), "data {data:?} must be empty Bytes");
    }
}

#[test]
fn call_dto_calldata_roundtrip() {
    let call = CallDto {
        data: "0xa9059cbb".into(),
        ..valid_call()
    };
    let core = call.into_core().expect("calldata must parse");
    assert_eq!(core.data.as_ref(), &[0xa9, 0x05, 0x9c, 0xbb]);
}

#[test]
fn call_dto_bad_address_maps_to_encoding() {
    let call = CallDto {
        to: "not-an-address".into(),
        ..valid_call()
    };
    assert!(matches!(
        call.into_core(),
        Err(BindingsError::Encoding {
            kind: EncodingErrorKind::Address
        })
    ));
}

#[test]
fn call_dto_bad_value_maps_to_encoding() {
    let call = CallDto {
        value_wei: "1.5".into(),
        ..valid_call()
    };
    assert!(matches!(
        call.into_core(),
        Err(BindingsError::Encoding {
            kind: EncodingErrorKind::Amount
        })
    ));
}

#[test]
fn call_dto_bad_data_maps_to_encoding() {
    let call = CallDto {
        data: "0xzz".into(),
        ..valid_call()
    };
    assert!(matches!(
        call.into_core(),
        Err(BindingsError::Encoding {
            kind: EncodingErrorKind::Calldata
        })
    ));
}

// ─── OperationDto mapping ──────────────────────────────────────

#[test]
fn operation_dto_uses_stable_status_and_path_strings() {
    use rustok_core::account::{Operation, OperationStatus, SubmissionPath};

    let op = Operation {
        id: "0xoperationid".into(),
        chain_id: 11155111,
        from: "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
            .parse()
            .expect("address"),
        calls: vec![],
        path: SubmissionPath::DirectSelfCall,
        status: OperationStatus::Broadcast,
        tx_hash: Some(
            "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .parse()
                .expect("hash"),
        ),
        block_number: None,
        error: None,
        created_at: 1000,
        updated_at: 1001,
    };
    let dto = OperationDto::from(op);
    assert_eq!(dto.id, "0xoperationid");
    assert_eq!(dto.chain_id, 11155111);
    assert_eq!(dto.status, "broadcast");
    assert_eq!(dto.path, "direct_self_call");
    assert_eq!(
        dto.tx_hash.as_deref(),
        Some("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
    );
    assert_eq!(dto.error, None);
}

// ─── Delegation status / lifecycle (no-RPC branches) ───────────

#[tokio::test]
async fn delegation_status_unsupported_chain_does_not_need_wallet() {
    let (_dir, h) = handle();
    // zkSync Era (324) is excluded from the 7702 matrix; the unsupported
    // check runs before any wallet-state or RPC access.
    let err = h.get_delegation_status(324).await.unwrap_err();
    assert!(matches!(
        err,
        BindingsError::Account {
            kind: AccountErrorKind::UnsupportedChain
        }
    ));
}

#[tokio::test]
async fn delegation_status_requires_unlock() {
    let (_dir, h) = handle();
    let err = h.get_delegation_status(11155111).await.unwrap_err();
    assert!(matches!(
        err,
        BindingsError::Wallet {
            kind: WalletErrorKind::NotUnlocked
        }
    ));
}

#[tokio::test]
async fn authorize_delegation_requires_unlock() {
    let (_dir, h) = handle();
    let err = h.authorize_delegation(11155111).await.unwrap_err();
    assert!(matches!(
        err,
        BindingsError::Wallet {
            kind: WalletErrorKind::NotUnlocked
        }
    ));
}

#[tokio::test]
async fn authorize_delegation_unsupported_chain() {
    let (_dir, h) = unlocked_handle().await;
    let err = h.authorize_delegation(324).await.unwrap_err();
    assert!(matches!(
        err,
        BindingsError::Account {
            kind: AccountErrorKind::UnsupportedChain
        }
    ));
}

#[tokio::test]
async fn revoke_delegation_unsupported_chain() {
    let (_dir, h) = unlocked_handle().await;
    let err = h.revoke_delegation(324).await.unwrap_err();
    assert!(matches!(
        err,
        BindingsError::Account {
            kind: AccountErrorKind::UnsupportedChain
        }
    ));
}

// ─── Operation execution (no-RPC branches) ─────────────────────

#[tokio::test]
async fn execute_operation_requires_unlock() {
    let (_dir, h) = handle();
    let err = h
        .execute_operation(11155111, vec![valid_call()])
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        BindingsError::Wallet {
            kind: WalletErrorKind::NotUnlocked
        }
    ));
}

#[tokio::test]
async fn execute_operation_empty_calls_maps_to_account_kind() {
    let (_dir, h) = unlocked_handle().await;
    let err = h.execute_operation(11155111, vec![]).await.unwrap_err();
    assert!(matches!(
        err,
        BindingsError::Account {
            kind: AccountErrorKind::EmptyCalls
        }
    ));
}

#[tokio::test]
async fn execute_operation_validates_calls_before_policy() {
    let (_dir, h) = unlocked_handle().await;
    let call = CallDto {
        to: "bad".into(),
        ..valid_call()
    };
    let err = h.execute_operation(1, vec![call]).await.unwrap_err();
    assert!(matches!(
        err,
        BindingsError::Encoding {
            kind: EncodingErrorKind::Address
        }
    ));
}

#[tokio::test]
async fn operation_status_unknown_id_is_none() {
    let (_dir, h) = handle();
    let status = h
        .get_operation_status("0xnonexistent".into())
        .await
        .expect("status lookup");
    assert!(status.is_none());
}
