use anchor_lang::prelude::*;

#[error_code]
pub enum DelegationError {
    #[msg("Non-matching delegated voter weight record governing token mint.")]
    InvalidGoverningTokenMint,

    #[msg("Non-matching delegated voter weight record realm.")]
    InvalidRealm,

    #[msg("Mismatched delegation record address provided.")]
    IncorrectDelegationAddress,

    #[msg("Provided delegation record does not match the delegate provided.")]
    NonMatchingDelegationRecordProvided,

    #[msg("Provided voter weight has already been delegated.")]
    VoterWeightAlreadyDelegated,

    #[msg("Provided voter weight has not been delegated to provided delegate.")]
    VoterWeightNotDelegatedToDelegate,

    #[msg("Only the realm authority can change voter weight settings.")]
    NotRealmAuthority,

    #[msg("Invalid voter weight record source.")]
    InvalidVoterWeightRecordSource,

    #[msg("Voter weight record must be expired to revoke.")]
    VoterWeightRecordMustBeExpired,

    #[msg("Cannot manually create a revocation voter weight record.")]
    InvalidActionType,
}
