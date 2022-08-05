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

    #[msg("Did not provide a full set of delegator accounts.")]
    MissingDelegatorAccounts,

    #[msg("Cannot close an account not owned by caller.")]
    VoterWeightRecordWrongOwner,

    #[msg("Target is in the wrong state to reclaim lamports.")]
    ReclaimTargetWrongState,
}
