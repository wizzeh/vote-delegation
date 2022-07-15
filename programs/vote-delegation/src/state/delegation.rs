use std::cell::RefMut;

use anchor_lang::{
    prelude::*,
    system_program::{self, Transfer},
};

use crate::error::DelegationError;

use super::voter_weight_record::VoterWeightRecord;

#[account(zero_copy)]
pub struct Delegation {
    pub delegate: Pubkey,
    pub voter_weight: u64,
}

impl Delegation {
    pub fn get_pda_address(
        realm: &Pubkey,
        governing_token_mint: &Pubkey,
        governing_token_owner: &Pubkey,
        target: &Pubkey,
    ) -> Pubkey {
        Pubkey::try_find_program_address(
            &Delegation::get_pda_seeds(realm, governing_token_mint, governing_token_owner, target),
            &crate::id(),
        )
        .unwrap()
        .0
    }

    pub fn get_pda_seeds<'a>(
        realm: &'a Pubkey,
        governing_token_mint: &'a Pubkey,
        governing_token_owner: &'a Pubkey,
        target: &'a Pubkey,
    ) -> [&'a [u8]; 5] {
        [
            b"voter-weight-record-delegation".as_ref(),
            realm.as_ref(),
            governing_token_mint.as_ref(),
            governing_token_owner.as_ref(),
            target.as_ref(),
        ]
    }

    pub fn try_init<'a, 'b>(
        loader: &'b AccountLoader<'a, Delegation>,
        record_for: &VoterWeightRecord,
        payer: AccountInfo<'a>,
        system_program: AccountInfo<'a>,
    ) -> Result<RefMut<'b, Self>> {
        require!(
            loader.to_account_info().data_is_empty(),
            DelegationError::VoterWeightAlreadyDelegated
        );

        require_keys_eq!(
            loader.to_account_info().key(),
            Delegation::get_pda_address(
                &record_for.realm,
                &record_for.governing_token_mint,
                &record_for.governing_token_owner,
                &record_for.weight_action_target.unwrap()
            ),
            DelegationError::IncorrectDelegationAddress,
        );

        let delegation_rent = Rent::get()?.minimum_balance(8 + 32 + 8);
        let delegate_record_data = loader.load_init()?;
        let info = loader.to_account_info();
        let delegate_record_lamports = info.try_borrow_mut_lamports()?;
        let needed_lamports = (**delegate_record_lamports)
            .checked_sub(delegation_rent)
            .unwrap_or_default();
        if needed_lamports > 0 {
            system_program::transfer(
                CpiContext::new(
                    system_program.to_account_info(),
                    Transfer {
                        from: payer.to_account_info(),
                        to: loader.to_account_info(),
                    },
                ),
                needed_lamports,
            )?;
        }

        Ok(delegate_record_data)
    }
}
