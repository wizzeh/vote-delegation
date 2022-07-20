use std::sync::Arc;

use anchor_lang::prelude::Pubkey;
use solana_program::instruction::Instruction;
use solana_program_test::{processor, ProgramTest};
use solana_sdk::{signer::Signer, transport::TransportError};
use vote_delegation::state::voter_weight_record::{VoterWeightAction, VoterWeightRecord};

use super::{
    governance_test::{GovernanceTest, RealmCookie},
    program_test_bench::{ProgramTestBench, WalletCookie},
};

pub struct DelegationTest {
    pub program_id: Pubkey,
    pub bench: Arc<ProgramTestBench>,
    pub governance: GovernanceTest,
}

pub struct VoterWeightRecordCookie {
    pub address: Pubkey,
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
            governance: GovernanceTest::new(bench_rc, Some(program_id), Some(program_id)),
        }
    }

    pub async fn with_vwr(
        &mut self,
        realm: &RealmCookie,
        owner: &WalletCookie,
        target: Pubkey,
    ) -> Result<VoterWeightRecordCookie, TransportError> {
        let data = anchor_lang::InstructionData::data(
            &vote_delegation::instruction::CreateVoterWeightRecord {
                governing_token_owner: owner.address,
                target,
                action: VoterWeightAction::CastVote,
            },
        );

        let address = VoterWeightRecord::get_pda_address(
            &realm.address,
            &realm.community_mint_cookie.address,
            &owner.address,
            &target,
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

        Ok(VoterWeightRecordCookie { address })
    }
}
