use anchor_lang::prelude::*;

#[error_code]
pub enum DelegationError {
    #[msg("Non-matching delegated voter weight record governing token mint.")]
    InvalidGoverningTokenMint,

    #[msg("Non-matching delegated voter weight record realm.")]
    InvalidRealm,

    #[msg("Mismatched delegation record address provided.")]
    IncorrectDelegationAddress,
}
