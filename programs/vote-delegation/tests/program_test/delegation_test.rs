use std::sync::Arc;

use anchor_lang::prelude::{AccountMeta, Pubkey};
use solana_program::instruction::Instruction;
use solana_program_test::{processor, ProgramTest};
use solana_sdk::{signature::Keypair, signer::Signer, transport::TransportError};
use spl_governance::state::{
    realm_config::get_realm_config_address, vote_record::get_vote_record_address,
};
use vote_delegation::state::{
    delegation::Delegation,
    settings::Settings,
    voter_weight_record::{VoterWeightAction, VoterWeightRecord},
};

use super::{
    governance_test::{GovernanceTest, ProposalCookie, RealmCookie, TokenOwnerRecordCookie},
    program_test_bench::{ProgramTestBench, WalletCookie},
    tools::NopOverride,
};

pub struct DelegationTest {
    pub program_id: Pubkey,
    pub bench: Arc<ProgramTestBench>,
    pub governance: GovernanceTest,
}

pub struct VoterWeightRecordCookie {
    pub address: Pubkey,
    pub owner: Pubkey,
    pub action: VoterWeightAction,
    pub target: Pubkey,
}

pub struct PrecursorProgramCookie {
    pub address: Pubkey,
}

pub struct DelegatorCookie {
    pub wallet: WalletCookie,
    pub token_owner_record: TokenOwnerRecordCookie,
    pub source_vwr: VoterWeightRecordCookie,
}

impl DelegationTest {
    pub fn add_program(program_test: &mut ProgramTest) {
        program_test.add_program(
            "vote_delegation",
            vote_delegation::id(),
            processor!(vote_delegation::entry),
        )
    }

    pub async fn start_new() -> Self {
        let mut program_test = ProgramTest::default();

        DelegationTest::add_program(&mut program_test);
        GovernanceTest::add_program(&mut program_test);

        let program_id = vote_delegation::id();
        let bench_rc = Arc::new(ProgramTestBench::start_new(program_test).await);

        Self {
            program_id,
            bench: bench_rc.clone(),
            governance: GovernanceTest::new(bench_rc, Some(program_id), None),
        }
    }

    pub async fn with_unassigned_tokens(
        &mut self,
        realm: &RealmCookie,
    ) -> Result<(), TransportError> {
        let mint = &realm.community_mint_cookie;

        self.bench
            .with_tokens(mint, &Pubkey::new_unique(), 100)
            .await?;

        Ok(())
    }

    pub async fn with_delegator(
        &mut self,
        realm: &RealmCookie,
        predecessor: &PrecursorProgramCookie,
        delegate: Pubkey,
        weight: u64,
        expiry: Option<u64>,
        action: VoterWeightAction,
        target: Pubkey,
    ) -> Result<DelegatorCookie, TransportError> {
        let wallet = self.bench.with_wallet().await;
        let token_owner_record = self
            .governance
            .with_token_owner_record(realm, &wallet)
            .await?;
        self.governance
            .set_delegate(&wallet, &token_owner_record, Some(delegate))
            .await?;

        let vwr_cookie = VoterWeightRecordCookie {
            address: Keypair::new().pubkey(),
            owner: wallet.address,
            action,
            target,
        };
        self.bench
            .set_anchor_account(
                &VoterWeightRecord {
                    realm: realm.address,
                    governing_token_mint: realm.account.community_mint,
                    governing_token_owner: wallet.address,
                    voter_weight: weight,
                    voter_weight_expiry: expiry,
                    weight_action: Some(action),
                    weight_action_target: Some(target),
                    reserved: Default::default(),
                },
                vwr_cookie.address,
                predecessor.address,
            )
            .await?;

        Ok(DelegatorCookie {
            wallet,
            token_owner_record,
            source_vwr: vwr_cookie,
        })
    }

    pub async fn with_precursor_program(
        &mut self,
        realm: &RealmCookie,
    ) -> Result<PrecursorProgramCookie, TransportError> {
        let cookie = PrecursorProgramCookie {
            address: Keypair::new().pubkey(),
        };

        self.bench
            .set_executable_account(vec![0u8], cookie.address, Keypair::new().pubkey())
            .await?;

        let data =
            anchor_lang::InstructionData::data(&vote_delegation::instruction::SetPrecursor {
                mint: realm.community_mint_cookie.address,
                voter_weight_source: cookie.address,
            });

        let accounts = anchor_lang::ToAccountMetas::to_account_metas(
            &vote_delegation::accounts::SetPrecursor {
                signer: realm.realm_authority.pubkey(),
                payer: self.bench.payer.pubkey(),
                settings: Settings::get_pda_address(&realm.address, &realm.account.community_mint),
                governance_program_id: self.governance.program_id,
                realm_info: realm.address,
                system_program: solana_sdk::system_program::id(),
            },
            None,
        );

        let set_precursor_ix = Instruction {
            program_id: vote_delegation::id(),
            accounts,
            data,
        };

        self.bench
            .process_transaction(&[set_precursor_ix], Some(&[&realm.realm_authority]))
            .await?;

        Ok(cookie)
    }

    pub async fn with_vwr(
        &mut self,
        realm: &RealmCookie,
        owner: &WalletCookie,
        target: Pubkey,
        action: VoterWeightAction,
    ) -> Result<VoterWeightRecordCookie, TransportError> {
        let data = anchor_lang::InstructionData::data(
            &vote_delegation::instruction::CreateVoterWeightRecord {
                governing_token_owner: owner.address,
                target,
                action,
            },
        );

        let address = VoterWeightRecord::get_pda_address(
            &realm.address,
            &realm.community_mint_cookie.address,
            &owner.address,
            &target,
            Some(action),
        );
        let accounts = anchor_lang::ToAccountMetas::to_account_metas(
            &vote_delegation::accounts::CreateVoterWeightRecord {
                payer: self.bench.payer.pubkey(),
                voter_weight_record: address,
                governance_program_id: self.governance.program_id,
                realm: realm.address,
                realm_governing_token_mint: realm.community_mint_cookie.address,
                system_program: solana_sdk::system_program::id(),
            },
            None,
        );

        let create_voter_weight_record_ix = Instruction {
            program_id: vote_delegation::id(),
            accounts,
            data,
        };

        let signers = &[&self.bench.payer];

        self.bench
            .process_transaction(&[create_voter_weight_record_ix], Some(signers))
            .await?;

        Ok(VoterWeightRecordCookie {
            address,
            action,
            target,
            owner: owner.address,
        })
    }

    pub async fn revoke_vote(
        &mut self,
        realm: &RealmCookie,
        delegator: &DelegatorCookie,
        to_revoke: &VoterWeightRecordCookie,
        proposal: &ProposalCookie,
        to_revoke_token_owner_record: &TokenOwnerRecordCookie,
    ) -> Result<(), TransportError> {
        self.revoke_vote_using_ix(
            realm,
            delegator,
            to_revoke,
            proposal,
            to_revoke_token_owner_record,
            NopOverride,
            None,
        )
        .await
    }

    pub async fn revoke_vote_using_ix<F: Fn(&mut Instruction)>(
        &mut self,
        realm: &RealmCookie,
        delegator: &DelegatorCookie,
        to_revoke: &VoterWeightRecordCookie,
        proposal: &ProposalCookie,
        to_revoke_token_owner_record: &TokenOwnerRecordCookie,
        instruction_override: F,
        signers: Option<&[&Keypair]>,
    ) -> Result<(), TransportError> {
        let data = anchor_lang::InstructionData::data(&vote_delegation::instruction::RevokeVote {});

        let accounts = anchor_lang::ToAccountMetas::to_account_metas(
            &vote_delegation::accounts::RevokeVote {
                payer: self.bench.payer.pubkey(),
                revoke_weight_record: VoterWeightRecord::get_revocation_address(
                    &realm.address,
                    &realm.community_mint_cookie.address,
                    &delegator.wallet.address,
                    &to_revoke.target,
                    Some(to_revoke.action),
                ),
                delegate: to_revoke.owner,
                delegation_record: Delegation::get_pda_address(
                    &realm.address,
                    &realm.community_mint_cookie.address,
                    &delegator.wallet.address,
                    &to_revoke.target,
                    Some(to_revoke.action),
                ),
                delegated_voter_weight_record: to_revoke.address,
                governance_program_id: self.governance.program_id,
                vote_record_info: get_vote_record_address(
                    &self.governance.program_id,
                    &to_revoke.target,
                    &to_revoke_token_owner_record.address,
                ),
                realm_info: realm.address,
                realm_config_info: get_realm_config_address(
                    &self.governance.program_id,
                    &realm.address,
                ),
                governance_info: proposal.account.governance,
                proposal_info: proposal.address,
                delegate_token_owner_record_info: to_revoke_token_owner_record.address,
                realm_governing_token_mint: realm.community_mint_cookie.address,
                governing_token_owner: delegator.wallet.address,
                system_program: solana_sdk::system_program::id(),
            },
            None,
        );

        let mut revoke_ix = Instruction {
            program_id: vote_delegation::id(),
            accounts,
            data,
        };

        instruction_override(&mut revoke_ix);

        let default_signers = &[&self.bench.payer, &delegator.wallet.signer];
        let signers = signers.unwrap_or(default_signers);

        self.bench
            .process_transaction(&[revoke_ix], Some(signers))
            .await?;

        Ok(())
    }

    pub async fn aggregate_delegation(
        &mut self,
        realm: &RealmCookie,
        owner: &WalletCookie,
        vwr: &VoterWeightRecordCookie,
        delegator_accounts: &[&DelegatorCookie],
    ) -> Result<(), TransportError> {
        self.aggregate_delegation_using_ix(realm, owner, vwr, delegator_accounts, NopOverride)
            .await
    }

    pub async fn aggregate_delegation_using_ix<F: Fn(&mut Instruction)>(
        &mut self,
        realm: &RealmCookie,
        owner: &WalletCookie,
        vwr: &VoterWeightRecordCookie,
        delegator_accounts: &[&DelegatorCookie],
        instruction_override: F,
    ) -> Result<(), TransportError> {
        let data = anchor_lang::InstructionData::data(
            &vote_delegation::instruction::UpdateVoterWeightRecord {
                voter_weight_action: vwr.action,
                target: Some(vwr.target),
            },
        );

        let mut accounts = anchor_lang::ToAccountMetas::to_account_metas(
            &vote_delegation::accounts::UpdateVoterWeightRecord {
                delegate: vwr.owner,
                payer: self.bench.payer.pubkey(),
                settings: Settings::get_pda_address(
                    &realm.address,
                    &realm.community_mint_cookie.address,
                ),
                voter_weight_record: vwr.address,
                system_program: solana_sdk::system_program::id(),
                governance_program_id: self.governance.program_id,
                realm: realm.address,
            },
            None,
        );

        for delegator in delegator_accounts {
            accounts.push(AccountMeta {
                pubkey: delegator.source_vwr.address,
                is_signer: false,
                is_writable: false,
            });
            accounts.push(AccountMeta {
                pubkey: delegator.token_owner_record.address,
                is_signer: false,
                is_writable: false,
            });
            accounts.push(AccountMeta {
                pubkey: Delegation::get_pda_address(
                    &realm.address,
                    &realm.community_mint_cookie.address,
                    &delegator.wallet.address,
                    &delegator.source_vwr.target,
                    Some(delegator.source_vwr.action),
                ),
                is_signer: false,
                is_writable: true,
            });
        }

        let mut update_voter_weight_record_ix = Instruction {
            program_id: vote_delegation::id(),
            accounts,
            data,
        };

        instruction_override(&mut update_voter_weight_record_ix);

        self.bench
            .process_transaction(
                &[update_voter_weight_record_ix],
                Some(&[&self.bench.payer, &owner.signer]),
            )
            .await?;

        Ok(())
    }
}
