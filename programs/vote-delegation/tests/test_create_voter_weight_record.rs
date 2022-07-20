use program_test::delegation_test::DelegationTest;
use solana_program_test::tokio;
use solana_sdk::transport::TransportError;

mod program_test;

type TestOutcome = Result<(), TransportError>;

#[tokio::test]
async fn test_create_voter_weight_record() -> TestOutcome {
    // Arrange
    let mut vote_delegation_test = DelegationTest::start_new().await;
    let realm_cookie = vote_delegation_test.governance.with_realm().await?;

    Ok(())
}
