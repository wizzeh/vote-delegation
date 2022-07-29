use program_test::{delegation_test::DelegationTest, tools::assert_vote_delegation_err};
use solana_program::{program_pack::IsInitialized, vote};
use solana_program_test::tokio;
use solana_sdk::{signature::Keypair, signer::Signer, transport::TransportError};
use spl_governance::state::{
    proposal::ProposalV2,
    realm::{RealmConfig, RealmV2},
    realm_config::{get_realm_config_address, RealmConfigAccount},
};
use vote_delegation::{
    error::DelegationError,
    state::{
        delegation::Delegation,
        voter_weight_record::{VoterWeightAction, VoterWeightRecord},
    },
};

mod program_test;

type TestOutcome = Result<(), TransportError>;

#[tokio::test]
async fn test_update_voter_weight_record() -> TestOutcome {
    // Arrange
    let mut vote_delegation_test = DelegationTest::start_new().await;
    let realm_cookie = vote_delegation_test.governance.with_realm().await?;
    let wallet = vote_delegation_test.bench.with_wallet().await;
    vote_delegation_test
        .with_unassigned_tokens(&realm_cookie)
        .await?;
    let token_owner_record = vote_delegation_test
        .governance
        .with_token_owner_record(&realm_cookie, &wallet)
        .await?;
    let proposal = vote_delegation_test
        .governance
        .with_proposal(&realm_cookie)
        .await?;
    let vwr_cookie = vote_delegation_test
        .with_vwr(
            &realm_cookie,
            &wallet,
            proposal.address,
            VoterWeightAction::CastVote,
        )
        .await?;
    let precursor_cookie = vote_delegation_test
        .with_precursor_program(&realm_cookie)
        .await?;

    let delegator = vote_delegation_test
        .with_delegator(
            &realm_cookie,
            &precursor_cookie,
            wallet.address,
            10,
            Some(u64::max_value()),
            VoterWeightAction::CastVote,
            proposal.address,
        )
        .await?;
    vote_delegation_test
        .aggregate_delegation(&realm_cookie, &wallet, &vwr_cookie, &[&delegator])
        .await?;

    vote_delegation_test
        .governance
        .cast_vote(
            &realm_cookie,
            &proposal,
            &wallet,
            &token_owner_record,
            &vwr_cookie,
        )
        .await?;

    // Act

    // Assert
    assert!(false);
    let vwr_record = vote_delegation_test
        .bench
        .get_anchor_account::<VoterWeightRecord>(vwr_cookie.address)
        .await;

    let delegate_record_addr = Delegation::get_pda_address(
        &realm_cookie.address,
        &realm_cookie.account.community_mint,
        &delegator.wallet.address,
        &delegator.source_vwr.target,
        Some(delegator.source_vwr.action),
    );
    let delegate_record = vote_delegation_test
        .bench
        .get_anchor_account::<Delegation>(delegate_record_addr)
        .await;
    let delegate_record_acct = vote_delegation_test
        .bench
        .get_account(&delegate_record_addr)
        .await
        .unwrap();

    assert_eq!(delegate_record.delegate, wallet.address);
    assert_eq!(delegate_record.voter_weight, 10);
    assert!(vote_delegation_test.bench.get_rent().await.is_exempt(
        delegate_record_acct.lamports,
        delegate_record_acct.data.len()
    ));

    assert_eq!(vwr_record.weight_action, Some(VoterWeightAction::CastVote));
    assert_eq!(vwr_record.weight_action_target, Some(proposal.address));
    assert_ne!(vwr_record.voter_weight_expiry, Some(0));
    assert_eq!(vwr_record.voter_weight, 10);

    Ok(())
}
