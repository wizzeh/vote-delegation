use anchor_lang::{
    prelude::*,
    system_program::{self, Transfer},
};

use crate::state::{
    delegation::Delegation,
    voter_weight_record::{VoterWeightAction, VoterWeightRecord},
};

#[derive(Accounts)]
#[instruction(voter_weight_action: VoterWeightAction, target: Option<Pubkey>)]
pub struct UpdateVoterWeightRecord<'info> {
    delegate: Signer<'info>,

    #[account(mut)]
    payer: AccountInfo<'info>,

    voter_weight_record: Account<'info, VoterWeightRecord>,

    system_program: Program<'info, System>,
}

pub fn update_voter_weight_record<'info>(
    ctx: Context<'_, '_, '_, 'info, UpdateVoterWeightRecord<'info>>,
    voter_weight_action: VoterWeightAction,
    target: Option<Pubkey>,
) -> Result<()> {
    let voter_weight_record = &mut ctx.accounts.voter_weight_record;

    for chunk in ctx.remaining_accounts.chunks_exact(2) {
        // Accumulate vote weight
        let account = chunk.first().unwrap();
        let to_agg = Account::<VoterWeightRecord>::try_from(account)?;
        voter_weight_record.try_aggregate(&to_agg)?;

        // Create delegation record
        let loader =
            AccountLoader::<Delegation>::try_from_unchecked(&crate::id(), chunk.last().unwrap())?;
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
