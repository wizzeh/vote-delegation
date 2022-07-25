use std::cell::RefMut;

use anchor_lang::Discriminator;
use anchor_lang::{
    prelude::*,
    system_program::{self, Transfer},
};
use solana_program::{
    program::{invoke, invoke_signed},
    system_instruction::{self, create_account},
};

use crate::error::DelegationError;

use super::voter_weight_record::{VoterWeightAction, VoterWeightRecord};

#[account]
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
        action: Option<VoterWeightAction>,
    ) -> Pubkey {
        Pubkey::try_find_program_address(
            &Delegation::get_pda_seeds(
                realm,
                governing_token_mint,
                governing_token_owner,
                target,
                &borsh::to_vec(&action).unwrap(),
            ),
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
        action: &'a [u8],
    ) -> [&'a [u8]; 6] {
        [
            b"voter-weight-record-delegation".as_ref(),
            realm.as_ref(),
            governing_token_mint.as_ref(),
            governing_token_owner.as_ref(),
            target.as_ref(),
            action,
        ]
    }

    pub fn size() -> usize {
        8 + std::mem::size_of::<Self>()
    }

    pub fn try_create<'a>(
        account_info: &AccountInfo<'a>,
        payer_info: &AccountInfo<'a>,
        system_info: &AccountInfo<'a>,
        seeds: &[&[u8]],
        bump: &[u8],
        data: &Self,
    ) -> Result<()> {
        let serialized_data = [&Self::discriminator()[..], &data.try_to_vec()?].concat();

        let rent_exempt_lamports = Rent::get()?.minimum_balance(serialized_data.len()).max(1);

        let signers_seeds: &[&[u8]] = &[seeds, &[bump]].concat();

        // If the account has some lamports already it can't be created using create_account instruction
        // Anybody can send lamports to a PDA and by doing so create the account and perform DoS attack by blocking create_account
        if account_info.lamports() > 0 {
            let top_up_lamports = rent_exempt_lamports.saturating_sub(account_info.lamports());

            if top_up_lamports > 0 {
                invoke(
                    &system_instruction::transfer(
                        payer_info.key,
                        account_info.key,
                        top_up_lamports,
                    ),
                    &[
                        payer_info.clone(),
                        account_info.clone(),
                        system_info.clone(),
                    ],
                )?;
            }

            invoke_signed(
                &system_instruction::allocate(account_info.key, Self::size() as u64),
                &[account_info.clone(), system_info.clone()],
                &[signers_seeds],
            )?;

            invoke_signed(
                &system_instruction::assign(account_info.key, &crate::id()),
                &[account_info.clone(), system_info.clone()],
                &[signers_seeds],
            )?;
        } else {
            // If the PDA doesn't exist use create_account to use lower compute budget
            let create_account_instruction = create_account(
                payer_info.key,
                account_info.key,
                rent_exempt_lamports,
                Self::size() as u64,
                &crate::id(),
            );

            invoke_signed(
                &create_account_instruction,
                &[
                    payer_info.clone(),
                    account_info.clone(),
                    system_info.clone(),
                ],
                &[signers_seeds],
            )?;
        }

        account_info
            .data
            .borrow_mut()
            .copy_from_slice(&serialized_data);

        Ok(())
    }
}
