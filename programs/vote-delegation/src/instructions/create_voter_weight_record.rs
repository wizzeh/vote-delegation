use anchor_lang::prelude::*;

use crate::{
    error::DelegationError,
    state::voter_weight_record::{VoterWeightAction, VoterWeightRecord},
};
use anchor_spl::token::Mint;

/**
 * Creates a new VoterWeightRecord
 *
 * You must create a VoterWeightRecord for each target and action you wish to use. Created
 * VoterWeightRecords initially have 0 voter weight.
 */
#[derive(Accounts)]
#[instruction(governing_token_owner: Pubkey, target: Pubkey, action: VoterWeightAction)]
pub struct CreateVoterWeightRecord<'info> {
    #[account(mut)]
    payer: Signer<'info>,

    #[account(
        init,
        seeds = [
            b"voter-weight-record".as_ref(),
            realm.key().as_ref(),
            realm_governing_token_mint.key().as_ref(),
            governing_token_owner.as_ref(),
            target.as_ref(),
            &borsh::to_vec(&Some(action)).unwrap()
        ],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<VoterWeightRecord>(),
        owner = crate::ID
    )]
    voter_weight_record: Account<'info, VoterWeightRecord>,

    /// The program id of the spl-governance program the realm belongs to
    /// CHECK: Can be any instance of spl-governance and it's not known at the compilation time
    #[account(executable)]
    pub governance_program_id: UncheckedAccount<'info>,

    /// CHECK: Owned by spl-governance instance specified in governance_program_id
    #[account(owner = governance_program_id.key())]
    pub realm: UncheckedAccount<'info>,

    /// Either the realm community mint or the council mint.
    pub realm_governing_token_mint: Account<'info, Mint>,

    pub system_program: Program<'info, System>,
}

pub fn create_voter_weight_record(
    ctx: Context<CreateVoterWeightRecord>,
    governing_token_owner: Pubkey,
    target: Pubkey,
    action: VoterWeightAction,
) -> Result<()> {
    require!(
        action != VoterWeightAction::RevokeVote,
        DelegationError::InvalidActionType
    );

    spl_governance::state::realm::get_realm_data_for_governing_token_mint(
        &ctx.accounts.governance_program_id.key(),
        &ctx.accounts.realm,
        &ctx.accounts.realm_governing_token_mint.key(),
    )?;

    let voter_weight_record = &mut ctx.accounts.voter_weight_record;

    voter_weight_record.realm = ctx.accounts.realm.key();
    voter_weight_record.governing_token_mint = ctx.accounts.realm_governing_token_mint.key();
    voter_weight_record.governing_token_owner = governing_token_owner;
    voter_weight_record.weight_action_target = Some(target);
    voter_weight_record.weight_action = Some(action);

    // Set expiry to expired
    voter_weight_record.voter_weight_expiry = Some(0);

    Ok(())
}
