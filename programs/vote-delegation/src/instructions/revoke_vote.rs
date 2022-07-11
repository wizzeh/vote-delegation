use anchor_lang::prelude::*;
use anchor_spl::token::Mint;
use spl_governance::state::{
    governance::get_governance_data_for_realm,
    proposal::get_proposal_data_for_governance_and_governing_mint,
    realm::get_realm_data_for_governing_token_mint,
    vote_record::get_vote_record_data_for_proposal_and_token_owner,
};

use crate::{
    error::DelegationError,
    state::{
        delegation::Delegation,
        voter_weight_record::{VoterWeightAction, VoterWeightRecord},
    },
};

#[derive(Accounts)]
pub struct RevokeVote<'info> {
    #[account(mut)]
    payer: AccountInfo<'info>,

    #[account(
        init,
        seeds = [
            b"voter-weight-record".as_ref(),
            realm_info.key().as_ref(),
            realm_governing_token_mint.key().as_ref(),
            governing_token_owner.key().as_ref()
        ],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<VoterWeightRecord>(),
    )]
    revoke_weight_record: Account<'info, VoterWeightRecord>,

    delegate: AccountInfo<'info>,

    #[account(
        mut,
        close = delegate,
        seeds = [
            b"voter-weight-record-delegation".as_ref(),
            realm_info.key().as_ref(),
            realm_governing_token_mint.key().as_ref(),
            governing_token_owner.key().as_ref() // TODO: Does this need to include the proposal?
        ],
        bump
    )]
    delegation_record: AccountLoader<'info, Delegation>,

    #[account(
        seeds = [
            b"voter-weight-record".as_ref(),
            realm_info.key().as_ref(),
            realm_governing_token_mint.key().as_ref(),
            delegate.key().as_ref()
        ],
        bump,
    )]
    delegated_voter_weight_record: Account<'info, VoterWeightRecord>,

    /// The program id of the spl-governance program the realm belongs to
    /// CHECK: Can be any instance of spl-governance and it's not known at the compilation time
    #[account(executable)]
    governance_program_id: UncheckedAccount<'info>,

    /// CHECK: Owned by spl-governance instance specified in governance_program_id
    #[account(owner = governance_program_id.key())]
    vote_record_info: UncheckedAccount<'info>,

    /// CHECK: Owned by spl-governance instance specified in governance_program_id
    #[account(owner = governance_program_id.key())]
    realm_info: UncheckedAccount<'info>,

    /// CHECK: Owned by spl-governance instance specified in governance_program_id
    #[account(owner = governance_program_id.key())]
    governance_info: UncheckedAccount<'info>,

    /// CHECK: Owned by spl-governance instance specified in governance_program_id
    #[account(owner = governance_program_id.key())]
    proposal_info: UncheckedAccount<'info>,

    /// Either the realm community mint or the council mint.
    realm_governing_token_mint: Account<'info, Mint>,

    governing_token_owner: Signer<'info>,

    system_program: Program<'info, System>,
}

pub fn revoke_vote(ctx: Context<RevokeVote>) -> Result<()> {
    let delegation_record_data = ctx.accounts.delegation_record.load()?;

    require_keys_eq!(
        delegation_record_data.delegate,
        ctx.accounts.delegate.key(),
        DelegationError::NonMatchingDelegationRecordProvided
    );

    get_realm_data_for_governing_token_mint(
        ctx.accounts.governance_program_id.key,
        &ctx.accounts.realm_info,
        &ctx.accounts.realm_governing_token_mint.key(),
    )?;
    get_governance_data_for_realm(
        ctx.accounts.governance_program_id.key,
        &ctx.accounts.governance_info.to_account_info(),
        ctx.accounts.realm_info.key,
    )?;
    get_proposal_data_for_governance_and_governing_mint(
        ctx.accounts.governance_program_id.key,
        &ctx.accounts.proposal_info.to_account_info(),
        ctx.accounts.governance_info.key,
        &ctx.accounts.realm_governing_token_mint.key(),
    )?;

    ctx.accounts.delegated_voter_weight_record.voter_weight = ctx
        .accounts
        .delegated_voter_weight_record
        .voter_weight
        .checked_sub(delegation_record_data.voter_weight)
        .unwrap();

    // We only need to unvote if a vote has actually been cast.
    if !ctx.accounts.vote_record_info.data_is_empty() {
        get_vote_record_data_for_proposal_and_token_owner(
            ctx.accounts.governance_program_id.key,
            &ctx.accounts.vote_record_info,
            &ctx.accounts.proposal_info.key(),
            ctx.accounts.delegate.key,
        )?;

        ctx.accounts
            .revoke_weight_record
            .set_inner(VoterWeightRecord {
                realm: ctx.accounts.realm_info.key(),
                governing_token_mint: ctx.accounts.realm_governing_token_mint.key(),
                governing_token_owner: ctx.accounts.governing_token_owner.key(),
                voter_weight: delegation_record_data.voter_weight,
                voter_weight_expiry: None,
                weight_action: Some(VoterWeightAction::RevokeVote),
                weight_action_target: Some(ctx.accounts.proposal_info.key()),
                reserved: Default::default(),
            });

        // TODO: Send the revoke instruction
    }

    Ok(())
}
