use anchor_lang::{prelude::*, solana_program::clock::DEFAULT_S_PER_SLOT};
use spl_governance::state::token_owner_record::get_token_owner_record_data_for_realm_and_governing_mint;
use static_assertions::const_assert;

use crate::{
    error::DelegationError,
    state::{
        delegation::Delegation,
        settings::Settings,
        voter_weight_record::{VoterWeightAction, VoterWeightRecord},
    },
};

/**
 * Aggregates delegated voter weight.
 *
 * This instruction is used to aggregate voter weight which has been delegated to a user
 * into their Voter Weight Record.
 *
 * For each source of voter weight to be aggregated, the following accounts must be
 * provided as additional accounts:
 *  - The VoterWeightRecord account produced by this program's voter weight source (not
 * signer, not writable).
 *  - The Realms TokenOwnerRecord of the delegator (not signer, not writable).
 *  - The Delegation PDA account `Delegation::get_pda_address` (not signer, writable).
 */
#[derive(Accounts)]
#[instruction(voter_weight_action: VoterWeightAction, target: Option<Pubkey>)]
pub struct UpdateVoterWeightRecord<'info> {
    delegate: Signer<'info>,

    /// CHECK: Payer
    #[account(mut)]
    payer: UncheckedAccount<'info>,

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
        mut,
        seeds = [
            b"voter-weight-record".as_ref(),
            voter_weight_record.realm.key().as_ref(),
            voter_weight_record.governing_token_mint.key().as_ref(),
            voter_weight_record.governing_token_owner.as_ref(),
            voter_weight_record.weight_action_target.unwrap().as_ref(),
            &borsh::to_vec(&voter_weight_record.weight_action).unwrap()
        ],
        bump,
        owner = crate::id()
    )]
    voter_weight_record: Account<'info, VoterWeightRecord>,

    system_program: Program<'info, System>,

    /// The program id of the spl-governance program the realm belongs to
    /// CHECK: Can be any instance of spl-governance and it's not known at the compilation time
    #[account(executable)]
    governance_program_id: UncheckedAccount<'info>,

    /// CHECK: Owned by spl-governance instance specified in governance_program_id
    #[account(owner = governance_program_id.key())]
    realm: UncheckedAccount<'info>,
}

const_assert!(APPROX_SLOTS_PER_MINUTE > 0);
const APPROX_SLOTS_PER_MINUTE: u64 = (60.0 / DEFAULT_S_PER_SLOT) as u64;

pub fn update_voter_weight_record<'info>(
    ctx: Context<'_, '_, '_, 'info, UpdateVoterWeightRecord<'info>>,
    voter_weight_action: VoterWeightAction,
    target: Option<Pubkey>,
) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.realm.key(),
        ctx.accounts.voter_weight_record.realm,
        DelegationError::InvalidRealm
    );

    require_eq!(
        ctx.remaining_accounts.len() % 3,
        0,
        DelegationError::MissingDelegatorAccounts
    );

    for to_aggregate in ctx.remaining_accounts.chunks_exact(3) {
        // Accumulate vote weight
        let mut to_aggregate_iter = to_aggregate.iter();
        let vwr_account = to_aggregate_iter.next().unwrap();
        let token_owner_info = to_aggregate_iter.next().unwrap();
        let delegation_info = to_aggregate_iter.next().unwrap();

        let token_owner_record = get_token_owner_record_data_for_realm_and_governing_mint(
            ctx.accounts.governance_program_id.key,
            token_owner_info,
            &ctx.accounts.voter_weight_record.realm,
            &ctx.accounts.voter_weight_record.governing_token_mint,
        )?;

        // You can always aggregate your own voter weight.
        if token_owner_record.governing_token_owner != ctx.accounts.delegate.key() {
            require!(
                token_owner_record.governance_delegate.is_some(),
                DelegationError::VoterWeightNotDelegatedToDelegate
            );
            require_keys_eq!(
                token_owner_record.governance_delegate.unwrap(),
                ctx.accounts.delegate.key(),
                DelegationError::VoterWeightNotDelegatedToDelegate
            );
        }

        require_keys_eq!(
            *vwr_account.owner,
            ctx.accounts.settings.voter_weight_source,
            DelegationError::InvalidVoterWeightRecordSource
        );

        require!(
            delegation_info.data_is_empty(),
            DelegationError::VoterWeightAlreadyDelegated
        );

        let mut data: &[u8] = &vwr_account.try_borrow_data()?;
        let to_agg = VoterWeightRecord::try_deserialize(&mut data)?;
        ctx.accounts.voter_weight_record.voter_weight =
            ctx.accounts.voter_weight_record.try_aggregate(&to_agg)?;

        // Create delegation record
        let encoded_action =
            borsh::to_vec(&ctx.accounts.voter_weight_record.weight_action).unwrap();
        let target = ctx
            .accounts
            .voter_weight_record
            .weight_action_target
            .unwrap();
        let signer_seeds = Delegation::get_pda_seeds(
            ctx.accounts.realm.key,
            &token_owner_record.governing_token_mint,
            &token_owner_record.governing_token_owner,
            &target,
            &encoded_action,
        );
        let (_, bump) = Pubkey::find_program_address(&signer_seeds, &crate::id());
        Delegation::try_create(
            delegation_info,
            &ctx.accounts.payer,
            &ctx.accounts.system_program,
            &Delegation::get_pda_seeds(
                ctx.accounts.realm.key,
                &token_owner_record.governing_token_mint.key(),
                &token_owner_record.governing_token_owner,
                &ctx.accounts
                    .voter_weight_record
                    .weight_action_target
                    .unwrap(),
                &borsh::to_vec(&ctx.accounts.voter_weight_record.weight_action).unwrap(),
            ),
            &[bump],
            &Delegation {
                delegate: ctx.accounts.delegate.key(),
                voter_weight_record: ctx.accounts.voter_weight_record.key(),
                voter_weight: to_agg.voter_weight,
            },
        )?;
    }

    // Give some time to spend multiple transactions aggregating.
    ctx.accounts.voter_weight_record.voter_weight_expiry =
        Some(Clock::get()?.slot + APPROX_SLOTS_PER_MINUTE);
    ctx.accounts.voter_weight_record.weight_action = Some(voter_weight_action);
    ctx.accounts.voter_weight_record.weight_action_target = target;

    Ok(())
}
