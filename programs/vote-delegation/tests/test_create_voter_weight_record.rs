use program_test::delegation_test::DelegationTest;
use solana_program::program_pack::IsInitialized;
use solana_program_test::tokio;
use solana_sdk::{signature::Keypair, signer::Signer, transport::TransportError};
use vote_delegation::state::voter_weight_record::VoterWeightRecord;

mod program_test;

type TestOutcome = Result<(), TransportError>;

#[tokio::test]
async fn test_create_voter_weight_record() -> TestOutcome {
    // Arrange
    let mut vote_delegation_test = DelegationTest::start_new().await;
    let realm_cookie = vote_delegation_test.governance.with_realm().await?;
    let wallet = vote_delegation_test.bench.with_wallet().await;
    let token_owner_record = vote_delegation_test
        .governance
        .with_token_owner_record(&realm_cookie, &wallet)
        .await?;
    let fake_proposal = Keypair::new();

    // Act
    let vwr_cookie = vote_delegation_test
        .with_vwr(&realm_cookie, &wallet, fake_proposal.pubkey())
        .await?;

    // Assert
    let vwr_record = vote_delegation_test
        .bench
        .get_anchor_account::<VoterWeightRecord>(vwr_cookie.address)
        .await;

    assert!(vwr_record.is_initialized());

    Ok(())
}
