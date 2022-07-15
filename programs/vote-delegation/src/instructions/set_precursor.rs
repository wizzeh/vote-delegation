use anchor_lang::prelude::*;
use spl_governance::state::realm::get_realm_data;

use crate::{error::DelegationError, state::settings::Settings};

#[derive(Accounts)]
#[instruction(mint: Pubkey, voter_weight_source: Pubkey)]
pub struct SetPrecursor<'info> {
    signer: Signer<'info>,

    #[account(mut)]
    payer: AccountInfo<'info>,

    #[account(
        init_if_needed,
        payer = payer,
        space = 8 + std::mem::size_of::<Settings>(),
        seeds = [
            b"settings".as_ref(),
            realm_info.key().as_ref(),
            mint.as_ref(),
        ],
        bump
    )]
    settings: Account<'info, Settings>,

    /// The program id of the spl-governance program the realm belongs to
    /// CHECK: Can be any instance of spl-governance and it's not known at the compilation time
    #[account(executable)]
    governance_program_id: UncheckedAccount<'info>,

    realm_info: UncheckedAccount<'info>,

    system_program: Program<'info, System>,
}

pub fn set_precursor(
    ctx: Context<SetPrecursor>,
    mint: Pubkey,
    voter_weight_source: Pubkey,
) -> Result<()> {
    let realm_data = get_realm_data(
        ctx.accounts.governance_program_id.key,
        &ctx.accounts.realm_info,
    )?;

    require!(
        realm_data.authority.is_some(),
        DelegationError::NotRealmAuthority
    );
    require_keys_eq!(
        realm_data.authority.unwrap(),
        ctx.accounts.signer.key(),
        DelegationError::NotRealmAuthority
    );

    ctx.accounts.settings.voter_weight_source = voter_weight_source;

    Ok(())
}
