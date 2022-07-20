use std::sync::Arc;

use anchor_lang::prelude::Pubkey;
use solana_program_test::{processor, ProgramTest};

use super::{governance_test::GovernanceTest, program_test_bench::ProgramTestBench};

pub struct DelegationTest {
    pub program_id: Pubkey,
    pub bench: Arc<ProgramTestBench>,
    pub governance: GovernanceTest,
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
}
