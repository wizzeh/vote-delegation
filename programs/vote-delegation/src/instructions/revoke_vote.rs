use anchor_lang::{
    accounts::orphan::OrphanAccount,
    prelude::*,
    solana_program::{instruction::Instruction, program::invoke},
};
use anchor_spl::token::Mint;
use spl_governance::state::{
    governance::get_governance_data_for_realm,
    proposal::get_proposal_data_for_governance_and_governing_mint,
    realm::get_realm_data_for_governing_token_mint,
    token_owner_record::get_token_owner_record_data_for_realm_and_governing_mint,
    vote_record::get_vote_record_data_for_proposal_and_token_owner_record,
};

use crate::{
    error::DelegationError,
    state::{
        delegation::Delegation,
        voter_weight_record::{VoterWeightAction, VoterWeightRecord},
    },
    tools::dispose_account,
};

#[derive(Accounts)]
pub struct RevokeVote<'info> {
    /// CHECK: Payer
    #[account(mut)]
    payer: UncheckedAccount<'info>,

    #[account(
        init,
        seeds = [
            b"revocation".as_ref(),
            realm_info.key().as_ref(),
            realm_governing_token_mint.key().as_ref(),
            governing_token_owner.key().as_ref(),
            delegated_voter_weight_record.weight_action_target.unwrap().key().as_ref(),
            &borsh::to_vec(&delegated_voter_weight_record.weight_action).unwrap()
        ],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<VoterWeightRecord>(),
        owner = crate::ID
    )]
    revoke_weight_record: OrphanAccount<'info, VoterWeightRecord>,

    /// CHECK: Delegate
    delegate: UncheckedAccount<'info>,

    #[account(
        mut,
        close = delegate,
        seeds = [
            b"voter-weight-record-delegation".as_ref(),
            realm_info.key().as_ref(),
            realm_governing_token_mint.key().as_ref(),
            governing_token_owner.key().as_ref(),
            delegated_voter_weight_record.weight_action_target.unwrap().key().as_ref(),
            &borsh::to_vec(&delegated_voter_weight_record.weight_action).unwrap()
        ],
        bump
    )]
    delegation_record: AccountLoader<'info, Delegation>,

    #[account(
        seeds = [
            b"voter-weight-record".as_ref(),
            realm_info.key().as_ref(),
            realm_governing_token_mint.key().as_ref(),
            delegate.key().as_ref(),
            delegated_voter_weight_record.weight_action_target.unwrap().key().as_ref(),
            &borsh::to_vec(&delegated_voter_weight_record.weight_action).unwrap()
        ],
        bump,
        owner = crate::ID
    )]
    delegated_voter_weight_record: OrphanAccount<'info, VoterWeightRecord>,

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

    /// CHECK: Owned by spl-governance instance specified in governance_program_id
    #[account(owner = governance_program_id.key())]
    delegate_token_owner_record_info: UncheckedAccount<'info>,

    /// Either the realm community mint or the council mint.
    realm_governing_token_mint: Account<'info, Mint>,

    governing_token_owner: Signer<'info>,

    system_program: Program<'info, System>,
}

pub fn revoke_vote<'a, 'b, 'c, 'd, 'e>(ctx: Context<'a, 'b, 'c, 'd, RevokeVote<'e>>) -> Result<()> {
    let delegation_record_data = ctx.accounts.delegation_record.load()?;

    require_keys_eq!(
        delegation_record_data.delegate,
        ctx.accounts.delegate.key(),
        DelegationError::NonMatchingDelegationRecordProvided
    );

    let realm_data = get_realm_data_for_governing_token_mint(
        ctx.accounts.governance_program_id.key,
        &ctx.accounts.realm_info,
        &ctx.accounts.realm_governing_token_mint.key(),
    )?;
    get_governance_data_for_realm(
        ctx.accounts.governance_program_id.key,
        &ctx.accounts.governance_info.to_account_info(),
        ctx.accounts.realm_info.key,
    )?;
    let proposal_data = get_proposal_data_for_governance_and_governing_mint(
        ctx.accounts.governance_program_id.key,
        &ctx.accounts.proposal_info.to_account_info(),
        ctx.accounts.governance_info.key,
        &ctx.accounts.realm_governing_token_mint.key(),
    )?;
    let delegate_token_owner_record_data =
        get_token_owner_record_data_for_realm_and_governing_mint(
            ctx.accounts.governance_program_id.key,
            &ctx.accounts.delegate_token_owner_record_info,
            ctx.accounts.realm_info.key,
            &ctx.accounts.realm_governing_token_mint.key(),
        )?;

    ctx.accounts.delegated_voter_weight_record.voter_weight = ctx
        .accounts
        .delegated_voter_weight_record
        .voter_weight
        .checked_sub(delegation_record_data.voter_weight)
        .unwrap();

    // This is needed to prevent double-voting when stacking voter weight plugins
    // Without this assertion the following attack vector exists:
    // 1) vote-delegation.update_voter_weight_record
    // 2) other-stacked-plugin.update_voter_weight_record
    // 3) voter-delegation.revoke_vote
    // 4) spl-gov.cast_vote
    if ctx
        .accounts
        .delegated_voter_weight_record
        .voter_weight_expiry
        >= Some(Clock::get()?.slot)
    {
        return Err(DelegationError::VoterWeightRecordMustBeExpired.into());
    }

    // We only need to unvote if a vote has actually been cast.
    if !ctx.accounts.vote_record_info.data_is_empty() {
        get_vote_record_data_for_proposal_and_token_owner_record(
            ctx.accounts.governance_program_id.key,
            &ctx.accounts.vote_record_info,
            &realm_data,
            &ctx.accounts.proposal_info.key(),
            &proposal_data,
            &delegate_token_owner_record_data,
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
                weight_action_target: Some(ctx.accounts.vote_record_info.key()),
                reserved: Default::default(),
            });

        invoke(
            &ctx.accounts.get_relinquish_instruction(),
            &get_relinquish_accounts(&ctx)[..],
        )?;

        // This account is disposed here to prevent double-relinquishment.
        dispose_account(
            &ctx.accounts.revoke_weight_record.to_account_info(),
            &ctx.accounts.payer,
        );
    }

    // The delegate paid for this account so they get the lamports back.
    dispose_account(
        &ctx.accounts.delegation_record.to_account_info(),
        &ctx.accounts.delegate,
    );

    Ok(())
}

fn get_relinquish_accounts<'e, 'f>(ctx: &'f Context<RevokeVote<'e>>) -> [AccountInfo<'e>; 9] {
    [
        ctx.accounts.realm_info.to_account_info(),
        ctx.accounts.governance_info.to_account_info(),
        ctx.accounts.proposal_info.to_account_info(),
        ctx.accounts
            .delegate_token_owner_record_info
            .to_account_info(),
        ctx.accounts.vote_record_info.to_account_info(),
        ctx.accounts.realm_governing_token_mint.to_account_info(),
        ctx.accounts.delegate.to_account_info(),
        ctx.accounts.delegate.to_account_info(),
        ctx.accounts.revoke_weight_record.to_account_info(),
    ]
}

impl<'a> RevokeVote<'a> {
    pub fn get_relinquish_instruction(&self) -> Instruction {
        spl_governance::instruction::relinquish_vote(
            &self.governance_program_id.key(),
            &self.realm_info.key(),
            &self.governance_info.key(),
            &self.proposal_info.key(),
            &self.delegate_token_owner_record_info.key(),
            &self.realm_governing_token_mint.key(),
            Some(self.delegate.key()),
            Some(self.delegate.key()),
            Some(self.revoke_weight_record.key()),
        )
    }
}
