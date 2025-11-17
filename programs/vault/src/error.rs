//! Vault Program Error Definitions

use solana_program::program_error::ProgramError;
use thiserror::Error;

#[derive(Error, Debug, Copy, Clone)]
pub enum VaultError {
    #[error("Invalid Admin Authority")]
    InvalidAuthority,
    
    #[error("Invalid Global Config Account")]
    InvalidGlobalConfig,
    
    #[error("Invalid Vault Account")]
    InvalidVaultAccount,
    
    #[error("Invalid Delegate Account")]
    InvalidDelegateAccount,
    
    #[error("Invalid Owner")]
    InvalidOwner,
    
    #[error("Invalid Delegate")]
    InvalidDelegate,
    
    #[error("Account Already Exists")]
    AccountAlreadyExists,
    
    #[error("Account Already Initialized")]
    AlreadyInitialized,
    
    #[error("Serialization Error")]
    SerializationError,
    
    #[error("Deserialization Error")]
    DeserializationError,
    
    #[error("Insufficient Collateral")]
    InsufficientCollateral,
    
    #[error("Insufficient Free Collateral")]
    InsufficientFreeCollateral,
    
    #[error("Delegate Not Active")]
    DelegateNotActive,
    
    #[error("Delegate Expired")]
    DelegateExpired,
    
    #[error("Permission Denied")]
    PermissionDenied,
    
    #[error("Notional Limit Exceeded")]
    NotionalLimitExceeded,
    
    #[error("Vault Is Frozen")]
    VaultFrozen,
    
    #[error("Invalid Amount")]
    InvalidAmount,
    
    #[error("Invalid Token Mint")]
    InvalidTokenMint,
    
    #[error("Invalid Token Account")]
    InvalidTokenAccount,
    
    #[error("Invalid Token Transfer")]
    InvalidTokenTransfer,
    
    #[error("Arithmetic Overflow")]
    ArithmeticOverflow,
    
    #[error("Arithmetic Underflow")]
    ArithmeticUnderflow,
    
    #[error("Invalid Expiry Slot")]
    InvalidExpirySlot,
    
    #[error("Invalid Permissions")]
    InvalidPermissions,
    
    #[error("Invalid Max Notional")]
    InvalidMaxNotional,
    
    #[error("Numerical Overflow")]
    NumericalOverflow,
}

impl From<VaultError> for ProgramError {
    fn from(e: VaultError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

