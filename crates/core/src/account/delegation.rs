//! EIP-7702 delegation state and lifecycle (ADR-001 §3.1, invariants §5.2).
//!
//! The chain is the only source of truth (§5.2.3): [`Delegation::state`]
//! reads `eth_getCode` on every call — there is deliberately no local cache
//! in circle 1 PR-2.
//!
//! Authorization transactions are NOT journaled (spec deviation 4, approved):
//! idempotency comes from the chain — re-authorizing to the same pinned
//! delegate is a state-level no-op. The double-broadcast window of
//! `Executor::execute` (circle 1 PR-1 review finding 1) applies here in
//! full: two concurrent `authorize` calls (e.g. a UI double-tap) broadcast
//! two type-4 transactions and the gas is paid TWICE. No user funds are at
//! risk (the authorization target is hardcoded, the second tx changes no
//! state), but the window is only closed by the `send.rs`/`sign.rs`
//! absorption (PR-4) or by call-site serialization (PR-3).
//!
//! Success of an authorization is NEVER inferred from the transaction
//! receipt (ADR-001 §10): after confirmation, re-read [`Delegation::state`]
//! and require `0xef0100‖DELEGATE_ADDRESS`.

use alloy_eips::eip7702::{Authorization, constants::EIP7702_DELEGATION_DESIGNATOR};
use alloy_primitives::{Address, B256, U256, keccak256};
use alloy_signer::SignerSync;
use alloy_signer_local::PrivateKeySigner;
use thiserror::Error;

use super::delegate;
use crate::provider::{MultiProvider, ProviderError};
use crate::send::SendError;

/// Gas limit for the self-sponsored type-4 authorization transaction.
///
/// Our `MultiProvider::estimate_gas` does not account for the authorization
/// list (21000 + `PER_AUTH_BASE_COST` 12500, +`PER_EMPTY_ACCOUNT_COST` 25000
/// for a previously-empty authority). A fixed limit is used instead; unused
/// gas is refunded by the protocol, so over-reserving costs nothing.
const AUTHORIZATION_TX_GAS: u64 = 100_000;

/// Delegation state of an account on one chain, read from `eth_getCode`.
///
/// Note: ADR-001 §3.1 sketches `Ours { version }`; the version field is
/// dropped deliberately — a future delegate upgrade changes the pinned
/// address, and the old delegate is then classified as [`Self::Foreign`],
/// which already routes to the "show, never silently overwrite, offer
/// revoke" branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelegationState {
    /// No code at the address — a plain EOA (also the state after a revoke:
    /// delegating to `0x0` clears the code entirely).
    None,
    /// Delegated to our pinned delegate (`0xef0100‖DELEGATE_ADDRESS`).
    Ours,
    /// Delegated to some other address — e.g. an imported seed already
    /// delegated in another wallet (§5.1 "Чужая делегация").
    Foreign(Address),
    /// The address holds code that is not a 7702 delegation (it is a
    /// contract). Not delegatable, not revocable via 7702.
    Other,
}

/// Parse the `eth_getCode` result into a [`DelegationState`].
///
/// Pure function — all guard logic is unit-tested here without RPC.
///
/// A 23-byte `0xef0100‖address` payload is a 7702 delegation; the cleared
/// form `0xef0100‖0x0000…0000` (which well-behaved chains never store —
/// clearing removes the code) is treated as [`DelegationState::None`].
#[must_use]
pub fn parse_delegation_code(code: &[u8]) -> DelegationState {
    if code.is_empty() {
        return DelegationState::None;
    }
    let designator_len = EIP7702_DELEGATION_DESIGNATOR.len();
    if code.len() == designator_len + 20 && code[..designator_len] == EIP7702_DELEGATION_DESIGNATOR
    {
        let target = Address::from_slice(&code[designator_len..]);
        if target == Address::ZERO {
            return DelegationState::None;
        }
        if target == delegate::DELEGATE_ADDRESS {
            return DelegationState::Ours;
        }
        return DelegationState::Foreign(target);
    }
    DelegationState::Other
}

/// Errors from delegation operations.
#[derive(Debug, Error)]
pub enum DelegationError {
    /// Provider/RPC failure.
    #[error(transparent)]
    Provider(#[from] ProviderError),

    /// Send-domain failure (txguard block, signing, broadcast).
    #[error(transparent)]
    Send(#[from] SendError),

    /// Authorization signature failure.
    #[error("sign authorization: {0}")]
    Sign(#[from] alloy_signer::Error),

    /// The chain has no EIP-7702 support or no pinned delegate deployment.
    #[error("chain {0} does not support EIP-7702 (not in the delegate matrix)")]
    UnsupportedChain(u64),

    /// Invariant §5.2.2: `chain_id = 0` authorizations are valid on all
    /// chains at once — never sign them.
    #[error("chain_id 0 authorizations are forbidden (valid on every chain at once)")]
    ChainIdZero,

    /// Invariant §5.2.5: the account is delegated to a foreign address;
    /// never overwrite it silently. Show it, offer a revoke.
    #[error("account is delegated to foreign address {0}; refusing to overwrite silently")]
    ForeignDelegation(Address),

    /// The address holds non-7702 contract code — it is not an EOA we can
    /// delegate.
    #[error("account address holds non-7702 contract code")]
    NotDelegatable,

    /// The pinned delegate address holds unexpected code on this chain
    /// (compromised lookalike deployment) — refuse to authorize.
    #[error("pinned delegate code-hash mismatch on chain {0}")]
    DelegateCodeMismatch(u64),

    /// Invariant §5.2.6 counterpart: revoke requires an existing delegation.
    #[error("nothing to revoke: account is not delegated")]
    NothingToRevoke,
}

/// What [`decide_authorize`] decided for the current state (pure policy).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuthDecision {
    /// No delegation yet — sign and broadcast the authorization.
    Authorize,
    /// Invariant §5.2.4: authorize once. Already delegated to our delegate —
    /// no new signature, no transaction.
    AlreadyDelegated,
}

/// Pure guard for `authorize` (invariants §5.2.4 and §5.2.5).
const fn decide_authorize(state: DelegationState) -> Result<AuthDecision, DelegationError> {
    match state {
        DelegationState::None => Ok(AuthDecision::Authorize),
        DelegationState::Ours => Ok(AuthDecision::AlreadyDelegated),
        DelegationState::Foreign(addr) => Err(DelegationError::ForeignDelegation(addr)),
        DelegationState::Other => Err(DelegationError::NotDelegatable),
    }
}

/// Pure guard for `revoke` (invariant §5.2.6). Revoking a foreign
/// delegation is allowed — it is an explicit user action on their own key.
const fn decide_revoke(state: DelegationState) -> Result<(), DelegationError> {
    match state {
        DelegationState::Ours | DelegationState::Foreign(_) => Ok(()),
        DelegationState::None | DelegationState::Other => Err(DelegationError::NothingToRevoke),
    }
}

/// Build the unsigned authorization for [`Delegation::authorize`].
///
/// The target is ALWAYS the pinned delegate — the address is not a
/// parameter anywhere in the delegation API, so signing for an arbitrary
/// (dApp-supplied) address is structurally impossible (invariant §5.2.1).
///
/// `tx_nonce` is the nonce of the carrying type-4 transaction; a
/// self-sponsored authorization must use `tx_nonce + 1` (ADR-001 §10: the
/// sender's nonce is already incremented when the authorization list is
/// processed).
fn authorize_request(chain_id: u64, tx_nonce: u64) -> Authorization {
    Authorization {
        chain_id: U256::from(chain_id),
        address: delegate::DELEGATE_ADDRESS,
        nonce: tx_nonce + 1,
    }
}

/// Build the unsigned authorization for [`Delegation::revoke`] — delegation
/// to `0x0` clears the code (invariant §5.2.6).
fn revoke_request(chain_id: u64, tx_nonce: u64) -> Authorization {
    Authorization {
        address: Address::ZERO,
        ..authorize_request(chain_id, tx_nonce)
    }
}

/// Delegation lifecycle against a [`MultiProvider`]. Thin coordinator —
/// borrow per call site, like [`super::Executor`].
pub struct Delegation<'a> {
    provider: &'a MultiProvider,
}

impl<'a> Delegation<'a> {
    /// Create a delegation handle over `provider`.
    pub const fn new(provider: &'a MultiProvider) -> Self {
        Self { provider }
    }

    /// Current delegation state of `account` on `chain_id`, fresh from
    /// `eth_getCode` (invariant §5.2.3 — the chain is the truth).
    ///
    /// # Errors
    ///
    /// [`DelegationError::Provider`] if every RPC endpoint of the chain
    /// fails or the chain is not configured.
    pub async fn state(
        &self,
        chain_id: u64,
        account: Address,
    ) -> Result<DelegationState, DelegationError> {
        let code = self.provider.code_at(chain_id, account).await?;
        Ok(parse_delegation_code(&code))
    }

    /// Authorize the pinned delegate for `signer`'s account on `chain_id`.
    ///
    /// Returns `Ok(None)` when the account is already delegated to our
    /// delegate (invariant §5.2.4 — no new transaction); `Ok(Some(hash))`
    /// after the type-4 transaction is broadcast. The receipt is NOT
    /// awaited and is NOT proof of delegation — re-read [`Self::state`]
    /// after confirmation (ADR-001 §10).
    ///
    /// Guards (invariants §5.2): `chain_id != 0`; chain must be in the
    /// delegate matrix; a foreign delegation or contract code at the
    /// account is an error, never a silent overwrite; the pinned delegate's
    /// on-chain code-hash is verified before signing.
    ///
    /// NOT journaled — see the module docstring for the accepted
    /// double-broadcast window.
    ///
    /// # Errors
    ///
    /// See [`DelegationError`].
    pub async fn authorize(
        &self,
        signer: &PrivateKeySigner,
        chain_id: u64,
    ) -> Result<Option<B256>, DelegationError> {
        self.check_chain(chain_id)?;
        let from = signer.address();

        match decide_authorize(self.state(chain_id, from).await?)? {
            AuthDecision::AlreadyDelegated => return Ok(None),
            AuthDecision::Authorize => {}
        }

        // The address alone is not enough: a lookalike deployment at the
        // pinned address with different code must stop the authorization.
        let delegate_code = self
            .provider
            .code_at(chain_id, delegate::DELEGATE_ADDRESS)
            .await?;
        if keccak256(&delegate_code) != delegate::DELEGATE_CODE_HASH {
            return Err(DelegationError::DelegateCodeMismatch(chain_id));
        }

        self.send_authorization(signer, chain_id, from, false)
            .await
            .map(Some)
    }

    /// Revoke the delegation of `signer`'s account on `chain_id`
    /// (authorization to `0x0`, invariant §5.2.6). Works from both `Ours`
    /// and `Foreign` states — revoking is always the user's own key acting
    /// on their own account.
    ///
    /// Same receipt/await semantics as [`Self::authorize`].
    ///
    /// # Errors
    ///
    /// [`DelegationError::NothingToRevoke`] when the account is not
    /// delegated; see [`DelegationError`] for the rest.
    pub async fn revoke(
        &self,
        signer: &PrivateKeySigner,
        chain_id: u64,
    ) -> Result<B256, DelegationError> {
        self.check_chain(chain_id)?;
        let from = signer.address();

        decide_revoke(self.state(chain_id, from).await?)?;

        self.send_authorization(signer, chain_id, from, true).await
    }

    /// Chain guards shared by `authorize`/`revoke`: never `chain_id = 0`
    /// (invariant §5.2.2), only chains from the delegate matrix.
    const fn check_chain(&self, chain_id: u64) -> Result<(), DelegationError> {
        if chain_id == 0 {
            return Err(DelegationError::ChainIdZero);
        }
        if !delegate::supports_eip7702(chain_id) {
            return Err(DelegationError::UnsupportedChain(chain_id));
        }
        Ok(())
    }

    /// Sign and broadcast the self-sponsored type-4 authorization
    /// transaction. The nonce is pinned explicitly: the authorization nonce
    /// must be `tx_nonce + 1`, so the carrier transaction nonce must be
    /// known before signing and is passed to the send path as-is.
    async fn send_authorization(
        &self,
        signer: &PrivateKeySigner,
        chain_id: u64,
        from: Address,
        revoke: bool,
    ) -> Result<B256, DelegationError> {
        let tx_nonce = self.provider.nonce(chain_id, from).await?;
        let auth = if revoke {
            revoke_request(chain_id, tx_nonce)
        } else {
            authorize_request(chain_id, tx_nonce)
        };
        let signature = signer.sign_hash_sync(&auth.signature_hash())?;
        let signed = auth.into_signed(signature);

        let tx = alloy_rpc_types_eth::TransactionRequest {
            to: Some(alloy_primitives::TxKind::Call(from)),
            value: Some(U256::ZERO),
            nonce: Some(tx_nonce),
            gas: Some(AUTHORIZATION_TX_GAS),
            authorization_list: Some(vec![signed]),
            ..Default::default()
        };

        Ok(
            crate::sign::sign_and_send_transaction_with_signer(signer, self.provider, tx, chain_id)
                .await?,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{Bytes, address, b256};

    const FOREIGN: Address = address!("1111111111111111111111111111111111111111");

    fn designator_code(target: Address) -> Bytes {
        let mut code = Vec::with_capacity(23);
        code.extend_from_slice(&EIP7702_DELEGATION_DESIGNATOR);
        code.extend_from_slice(target.as_slice());
        code.into()
    }

    // --- parse_delegation_code (invariant §5.2.3 mechanics) ---

    #[test]
    fn parse_empty_code_is_none() {
        assert_eq!(parse_delegation_code(&[]), DelegationState::None);
    }

    #[test]
    fn parse_our_delegate_is_ours() {
        let code = designator_code(delegate::DELEGATE_ADDRESS);
        assert_eq!(parse_delegation_code(&code), DelegationState::Ours);
    }

    #[test]
    fn parse_foreign_delegate_is_foreign() {
        let code = designator_code(FOREIGN);
        assert_eq!(
            parse_delegation_code(&code),
            DelegationState::Foreign(FOREIGN)
        );
    }

    #[test]
    fn parse_cleared_delegation_is_none() {
        // 0xef0100‖0x0000…0000 — the cleared form; chains remove the code
        // entirely on revoke, but treat the explicit form as None too.
        let code = designator_code(Address::ZERO);
        assert_eq!(parse_delegation_code(&code), DelegationState::None);
    }

    #[test]
    fn parse_contract_code_is_other() {
        // Real contract bytecode (not a designator prefix).
        assert_eq!(
            parse_delegation_code(&[0x60, 0x80, 0x60, 0x40, 0x52]),
            DelegationState::Other
        );
        // Designator prefix but wrong length — malformed, not a delegation.
        let mut bad = Vec::new();
        bad.extend_from_slice(&EIP7702_DELEGATION_DESIGNATOR);
        bad.extend_from_slice(&[0u8; 19]);
        assert_eq!(parse_delegation_code(&bad), DelegationState::Other);
        let mut long = designator_code(FOREIGN).to_vec();
        long.push(0x00);
        assert_eq!(parse_delegation_code(&long), DelegationState::Other);
    }

    // --- decide_authorize (invariants §5.2.4, §5.2.5) ---

    #[test]
    fn authorize_from_none_is_allowed() {
        assert_eq!(
            decide_authorize(DelegationState::None).unwrap(),
            AuthDecision::Authorize
        );
    }

    /// Invariant §5.2.4: authorize once — already ours is a no-op.
    #[test]
    fn authorize_from_ours_is_noop() {
        assert_eq!(
            decide_authorize(DelegationState::Ours).unwrap(),
            AuthDecision::AlreadyDelegated
        );
    }

    /// Invariant §5.2.5: a foreign delegation is never silently overwritten.
    #[test]
    fn authorize_from_foreign_is_rejected() {
        assert!(matches!(
            decide_authorize(DelegationState::Foreign(FOREIGN)),
            Err(DelegationError::ForeignDelegation(addr)) if addr == FOREIGN
        ));
    }

    #[test]
    fn authorize_from_other_is_rejected() {
        assert!(matches!(
            decide_authorize(DelegationState::Other),
            Err(DelegationError::NotDelegatable)
        ));
    }

    // --- decide_revoke (invariant §5.2.6) ---

    #[test]
    fn revoke_allowed_from_ours_and_foreign() {
        assert!(decide_revoke(DelegationState::Ours).is_ok());
        assert!(decide_revoke(DelegationState::Foreign(FOREIGN)).is_ok());
    }

    #[test]
    fn revoke_rejected_from_none_and_other() {
        assert!(matches!(
            decide_revoke(DelegationState::None),
            Err(DelegationError::NothingToRevoke)
        ));
        assert!(matches!(
            decide_revoke(DelegationState::Other),
            Err(DelegationError::NothingToRevoke)
        ));
    }

    // --- authorization construction (invariants §5.2.1, §5.2.2, ADR §10) ---

    /// Invariant §5.2.1: the authorization target is always the pinned
    /// delegate — the address is not a parameter of the delegation API.
    #[test]
    fn authorize_request_targets_only_pinned_delegate() {
        let auth = authorize_request(1, 41);
        assert_eq!(auth.address, delegate::DELEGATE_ADDRESS);
        assert_eq!(auth.chain_id, U256::from(1));
    }

    /// ADR-001 §10: self-sponsored type-4 — authorization nonce is the
    /// carrier transaction's nonce + 1.
    #[test]
    fn authorization_nonce_is_tx_nonce_plus_one() {
        assert_eq!(authorize_request(1, 41).nonce, 42);
        assert_eq!(revoke_request(1, 41).nonce, 42);
    }

    /// Invariant §5.2.6: revoke delegates to `0x0`.
    #[test]
    fn revoke_request_targets_zero_address() {
        let auth = revoke_request(8453, 0);
        assert_eq!(auth.address, Address::ZERO);
        assert_eq!(auth.chain_id, U256::from(8453));
    }

    /// Invariant §5.2.2: `chain_id = 0` is rejected before any RPC.
    #[tokio::test]
    async fn authorize_chain_id_zero_rejected() {
        let provider = MultiProvider::default_chains();
        let delegation = Delegation::new(&provider);
        let signer: PrivateKeySigner =
            b256!("0000000000000000000000000000000000000000000000000000000000000001")
                .to_string()
                .parse()
                .unwrap();
        assert!(matches!(
            delegation.authorize(&signer, 0).await,
            Err(DelegationError::ChainIdZero)
        ));
        assert!(matches!(
            delegation.revoke(&signer, 0).await,
            Err(DelegationError::ChainIdZero)
        ));
    }

    /// Chains outside the delegate matrix are rejected before any RPC.
    #[tokio::test]
    async fn authorize_unsupported_chain_rejected() {
        let provider = MultiProvider::default_chains();
        let delegation = Delegation::new(&provider);
        let signer: PrivateKeySigner =
            b256!("0000000000000000000000000000000000000000000000000000000000000001")
                .to_string()
                .parse()
                .unwrap();
        // zkSync Era — excluded from 7702 by the Captain's decision.
        assert!(matches!(
            delegation.authorize(&signer, 324).await,
            Err(DelegationError::UnsupportedChain(324))
        ));
        assert!(matches!(
            delegation.revoke(&signer, 324).await,
            Err(DelegationError::UnsupportedChain(324))
        ));
    }
}
