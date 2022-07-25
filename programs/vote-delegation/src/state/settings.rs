use anchor_lang::prelude::*;

#[account]
pub struct Settings {
    pub voter_weight_source: Pubkey,
}

impl Settings {
    pub fn get_pda_address(realm: &Pubkey, governing_token_mint: &Pubkey) -> Pubkey {
        Pubkey::try_find_program_address(
            &Settings::get_pda_seeds(realm, governing_token_mint),
            &crate::id(),
        )
        .unwrap()
        .0
    }

    pub fn get_pda_seeds<'a>(realm: &'a Pubkey, governing_token_mint: &'a Pubkey) -> [&'a [u8]; 3] {
        [
            b"settings".as_ref(),
            realm.as_ref(),
            governing_token_mint.as_ref(),
        ]
    }
}
