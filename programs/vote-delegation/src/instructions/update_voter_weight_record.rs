use anchor_lang::{accounts::orphan::OrphanAccount, prelude::*};
use spl_governance::state::token_owner_record::get_token_owner_record_data_for_realm_and_governing_mint;

use crate::{
    error::DelegationError,
    state::{
        delegation::Delegation,
        settings::Settings,
        voter_weight_record::{VoterWeightAction, VoterWeightRecord},
    },
};

#[derive(Accounts)]
#[instruction(voter_weight_action: VoterWeightAction, target: Option<Pubkey>)]
pub struct UpdateVoterWeightRecord<'info> {
    delegate: Signer<'info>,

    #[account(mut)]
    payer: AccountInfo<'info>,

    #[account(
        seeds = [
            b"settings".as_ref(),
            realm.key().as_ref(),
            voter_weight_record.governing_token_mint.key().as_ref(),
        ],
        bump
    )]
    settings: Account<'info, Settings>,

    #[account(
        seeds = [
            b"voter-weight-record".as_ref(),
            voter_weight_record.realm.key().as_ref(),
            voter_weight_record.governing_token_mint.key().as_ref(),
            voter_weight_record.governing_token_owner.as_ref(),
            voter_weight_record.weight_action_target.unwrap().as_ref()
        ],
        bump,
        owner = crate::id()
    )]
    voter_weight_record: OrphanAccount<'info, VoterWeightRecord>,

    system_program: Program<'info, System>,

    /// The program id of the spl-governance program the realm belongs to
    /// CHECK: Can be any instance of spl-governance and it's not known at the compilation time
    #[account(executable)]
    governance_program_id: UncheckedAccount<'info>,

    /// CHECK: Owned by spl-governance instance specified in governance_program_id
    #[account(owner = governance_program_id.key())]
    realm: UncheckedAccount<'info>,
}

pub fn update_voter_weight_record<'info>(
    ctx: Context<'_, '_, '_, 'info, UpdateVoterWeightRecord<'info>>,
    voter_weight_action: VoterWeightAction,
    target: Option<Pubkey>,
) -> Result<()> {
    let voter_weight_record = &mut ctx.accounts.voter_weight_record;

    require_keys_eq!(
        ctx.accounts.realm.key(),
        voter_weight_record.realm,
        DelegationError::InvalidRealm
    );

    for to_aggregate in ctx.remaining_accounts.chunks_exact(3) {
        // Accumulate vote weight
        let mut to_aggregate_iter = to_aggregate.iter();
        let account = to_aggregate_iter.next().unwrap();
        let token_owner_info = to_aggregate_iter.next().unwrap();
        let delegation_info = to_aggregate_iter.next().unwrap();

        let token_owner_record = get_token_owner_record_data_for_realm_and_governing_mint(
            ctx.accounts.governance_program_id.key,
            token_owner_info,
            &voter_weight_record.realm,
            &voter_weight_record.governing_token_mint,
        )?;

        require!(
            token_owner_record.governance_delegate.is_some(),
            DelegationError::VoterWeightNotDelegatedToDelegate
        );
        require_keys_eq!(
            token_owner_record.governance_delegate.unwrap(),
            ctx.accounts.delegate.key(),
            DelegationError::VoterWeightNotDelegatedToDelegate
        );

        require_keys_eq!(
            *account.owner,
            ctx.accounts.settings.voter_weight_source,
            DelegationError::InvalidVoterWeightRecordSource
        );
        let to_agg = OrphanAccount::<VoterWeightRecord>::try_from(account)?;
        voter_weight_record.try_aggregate(&to_agg)?;

        // Create delegation record
        let loader =
            AccountLoader::<Delegation>::try_from_unchecked(&crate::id(), delegation_info)?;
        let mut delegate = Delegation::try_init(
            &loader,
            &to_agg,
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        )?;
        delegate.delegate = ctx.accounts.delegate.key();
        delegate.voter_weight = to_agg.voter_weight;
    }

    voter_weight_record.voter_weight_expiry = Some(Clock::get()?.slot); // TODO: End of proposal?
    voter_weight_record.weight_action = Some(voter_weight_action);
    voter_weight_record.weight_action_target = target;

    Ok(())
}
