pub mod error;
mod instructions;
pub mod state;
mod tools;

use anchor_lang::prelude::*;
use instructions::*;
use state::voter_weight_record::VoterWeightAction;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod vote_delegation {
    use super::*;

    pub fn create_voter_weight_record(
        ctx: Context<CreateVoterWeightRecord>,
        governing_token_owner: Pubkey,
        target: Pubkey,
        action: VoterWeightAction,
    ) -> Result<()> {
        instructions::create_voter_weight_record(ctx, governing_token_owner, target, action)
    }

    pub fn revoke_vote(ctx: Context<RevokeVote>) -> Result<()> {
        instructions::revoke_vote(ctx)
    }

    pub fn set_precursor(
        ctx: Context<SetPrecursor>,
        mint: Pubkey,
        voter_weight_source: Pubkey,
    ) -> Result<()> {
        instructions::set_precursor(ctx, mint, voter_weight_source)
    }

    pub fn update_voter_weight_record<'info>(
        ctx: Context<'_, '_, '_, 'info, UpdateVoterWeightRecord<'info>>,
        voter_weight_action: VoterWeightAction,
        target: Option<Pubkey>,
    ) -> Result<()> {
        instructions::update_voter_weight_record(ctx, voter_weight_action, target)
    }

    pub fn reclaim_voter_weight_record(ctx: Context<ReclaimVoterWeightRecord>) -> Result<()> {
        instructions::reclaim_voter_weight_record(ctx)
    }

    pub fn reclaim_delegation(ctx: Context<ReclaimDelegation>) -> Result<()> {
        instructions::reclaim_delegation(ctx)
    }
}
