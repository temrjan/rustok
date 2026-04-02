//! ABI definitions using alloy `sol!` macro.
//!
//! These generate compile-time Rust bindings for common contract interfaces.
//! Each function gets a corresponding Rust struct with `abi_decode` support.

use alloy_sol_types::sol;

// ERC-20 Token Standard
sol! {
    /// Transfer tokens to a recipient.
    function transfer(address to, uint256 amount) external returns (bool);

    /// Approve a spender to use tokens on behalf of the caller.
    function approve(address spender, uint256 amount) external returns (bool);

    /// Transfer tokens from one address to another (requires prior approval).
    function transferFrom(address from, address to, uint256 amount) external returns (bool);
}

// ERC-721 NFT Standard
sol! {
    /// Transfer an NFT (safe transfer with data).
    function safeTransferFrom(address from, address to, uint256 tokenId, bytes data) external;

    /// Set approval for all tokens to an operator.
    function setApprovalForAll(address operator, bool approved) external;
}

// ERC-20 Permit (EIP-2612)
sol! {
    /// Approve via off-chain signature.
    function permit(
        address owner,
        address spender,
        uint256 value,
        uint256 deadline,
        uint8 v,
        bytes32 r,
        bytes32 s
    ) external;
}

/// Well-known function selectors for quick matching.
pub(crate) mod selectors {
    /// ERC-20 `transfer(address,uint256)` — 0xa9059cbb
    pub(crate) const TRANSFER: [u8; 4] = [0xa9, 0x05, 0x9c, 0xbb];

    /// ERC-20 `approve(address,uint256)` — 0x095ea7b3
    pub(crate) const APPROVE: [u8; 4] = [0x09, 0x5e, 0xa7, 0xb3];

    /// ERC-20 `transferFrom(address,address,uint256)` — 0x23b872dd
    pub(crate) const TRANSFER_FROM: [u8; 4] = [0x23, 0xb8, 0x72, 0xdd];

    /// ERC-721 `setApprovalForAll(address,bool)` — 0xa22cb465
    pub(crate) const SET_APPROVAL_FOR_ALL: [u8; 4] = [0xa2, 0x2c, 0xb4, 0x65];

    /// EIP-2612 `permit(address,address,uint256,uint256,uint8,bytes32,bytes32)` — 0xd505accf
    pub(crate) const PERMIT: [u8; 4] = [0xd5, 0x05, 0xac, 0xcf];
}
