use anchor_lang::prelude::*;

use crate::{error::DelegationError, state::delegation::Delegation};

#[derive(Accounts)]
pub struct ReclaimDelegation<'info> {
    #[account(mut)]
    payer: Signer<'info>,

    #[account(mut)]
    delegate: AccountInfo<'info>,

    #[account(zero)]
    voter_weight_record: AccountInfo<'info>,

    #[account(mut, close = delegate)]
    delegation: Account<'info, Delegation>,
}

pub fn reclaim_delegation(ctx: Context<ReclaimDelegation>) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.delegation.voter_weight_record,
        ctx.accounts.voter_weight_record.key(),
        DelegationError::IncorrectDelegationAddress
    );
    require_keys_eq!(
        ctx.accounts.delegation.delegate,
        ctx.accounts.delegate.key(),
        DelegationError::NonMatchingDelegationRecordProvided,
    );

    Ok(())
}
