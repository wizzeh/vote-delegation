use anchor_lang::prelude::*;

use crate::{error::DelegationError, state::voter_weight_record::VoterWeightRecord};

#[derive(Accounts)]
pub struct ReclaimVoterWeightRecord<'info> {
    #[account(mut)]
    payer: Signer<'info>,

    #[account(mut)]
    caller: Signer<'info>,

    #[account(
        mut,
        close = caller,
        owner = crate::ID
    )]
    voter_weight_record: Account<'info, VoterWeightRecord>,

    /// CHECK: Will be deserialized depending on voter_weight_record type.
    #[account(owner = governance_program_id.key())]
    target: AccountInfo<'info>,

    /// The program id of the spl-governance program the realm belongs to
    /// CHECK: Can be any instance of spl-governance and it's not known at the compilation time
    #[account(executable)]
    governance_program_id: UncheckedAccount<'info>,
}

pub fn reclaim_voter_weight_record(ctx: Context<ReclaimVoterWeightRecord>) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.voter_weight_record.governing_token_owner,
        ctx.accounts.caller.key(),
        DelegationError::VoterWeightRecordWrongOwner
    );

    ctx.accounts
        .voter_weight_record
        .assert_can_reclaim(&ctx.accounts.target, ctx.accounts.governance_program_id.key)?;

    Ok(())
}
