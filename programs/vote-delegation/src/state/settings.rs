use anchor_lang::prelude::*;

#[account]
pub struct Settings {
    pub voter_weight_source: Pubkey,
}
