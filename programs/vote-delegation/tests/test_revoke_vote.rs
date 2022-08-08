use crate::program_test::tools::assert_anchor_err;

use anchor_lang::prelude::ErrorCode;
use program_test::delegation_test::DelegationTest;

use solana_program_test::tokio;
use solana_sdk::transport::TransportError;
use spl_governance::state::{proposal::ProposalV2, vote_record::get_vote_record_address};
use vote_delegation::state::{
    delegation::Delegation,
    voter_weight_record::{VoterWeightAction, VoterWeightRecord},
};

mod program_test;

type TestOutcome = Result<(), TransportError>;

#[tokio::test]
async fn test_revoke_voter_weight_record() -> TestOutcome {
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

    vote_delegation_test.bench.advance_clock_a_lot().await;

    // Act
    vote_delegation_test
        .revoke_vote(
            &realm_cookie,
            &delegator,
            &vwr_cookie,
            &proposal,
            &token_owner_record,
        )
        .await?;

    // Assert
    let proposal_record = vote_delegation_test
        .bench
        .get_borsh_account::<ProposalV2>(&proposal.address)
        .await;

    assert_eq!(proposal_record.options[0].vote_weight, 0);

    let delegation_record = vote_delegation_test
        .bench
        .get_account(&Delegation::get_pda_address(
            &realm_cookie.address,
            &realm_cookie.community_mint_cookie.address,
            &delegator.wallet.address,
            &proposal.address,
            Some(VoterWeightAction::CastVote),
        ))
        .await;

    assert!(delegation_record.is_none());

    let revoke_record = vote_delegation_test
        .bench
        .get_account(&VoterWeightRecord::get_revocation_address(
            &realm_cookie.address,
            &realm_cookie.community_mint_cookie.address,
            &delegator.wallet.address,
            &proposal.address,
            Some(VoterWeightAction::CastVote),
        ))
        .await;

    assert!(revoke_record.is_none());

    let vote_record = vote_delegation_test
        .bench
        .get_account(&get_vote_record_address(
            &vote_delegation_test.governance.program_id,
            &vwr_cookie.target,
            &token_owner_record.address,
        ))
        .await;

    assert!(vote_record.is_none());

    Ok(())
}

#[tokio::test]
async fn test_revoke_voter_weight_record_before_vote_cast() -> TestOutcome {
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

    vote_delegation_test.bench.advance_clock_a_lot().await;

    // Act
    vote_delegation_test
        .revoke_vote(
            &realm_cookie,
            &delegator,
            &vwr_cookie,
            &proposal,
            &token_owner_record,
        )
        .await?;

    // Assert
    let proposal_record = vote_delegation_test
        .bench
        .get_borsh_account::<ProposalV2>(&proposal.address)
        .await;

    assert_eq!(proposal_record.options[0].vote_weight, 0);

    let delegation_record = vote_delegation_test
        .bench
        .get_account(&Delegation::get_pda_address(
            &realm_cookie.address,
            &realm_cookie.community_mint_cookie.address,
            &delegator.wallet.address,
            &proposal.address,
            Some(VoterWeightAction::CastVote),
        ))
        .await;

    assert!(delegation_record.is_none());

    let revoke_record = vote_delegation_test
        .bench
        .get_account(&VoterWeightRecord::get_revocation_address(
            &realm_cookie.address,
            &realm_cookie.community_mint_cookie.address,
            &delegator.wallet.address,
            &proposal.address,
            Some(VoterWeightAction::CastVote),
        ))
        .await;

    assert!(revoke_record.is_none());

    let vote_record = vote_delegation_test
        .bench
        .get_account(&get_vote_record_address(
            &vote_delegation_test.governance.program_id,
            &vwr_cookie.target,
            &token_owner_record.address,
        ))
        .await;

    assert!(vote_record.is_none());

    Ok(())
}

#[tokio::test]
async fn test_repeat_revoke_voter_weight_record_err() -> TestOutcome {
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

    vote_delegation_test.bench.advance_clock_a_lot().await;

    vote_delegation_test
        .revoke_vote(
            &realm_cookie,
            &delegator,
            &vwr_cookie,
            &proposal,
            &token_owner_record,
        )
        .await?;

    vote_delegation_test.bench.advance_clock().await;

    // Act
    let err = vote_delegation_test
        .revoke_vote(
            &realm_cookie,
            &delegator,
            &vwr_cookie,
            &proposal,
            &token_owner_record,
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_anchor_err(err, ErrorCode::AccountNotInitialized);

    Ok(())
}
