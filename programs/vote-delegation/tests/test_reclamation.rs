use crate::program_test::tools::assert_anchor_err;
use crate::program_test::tools::assert_ix_err;
use crate::program_test::tools::assert_vote_delegation_err;
use anchor_lang::prelude::{AnchorError, ErrorCode};
use program_test::delegation_test::DelegationTest;
use solana_program::instruction::Instruction;
use solana_program_test::tokio;
use solana_sdk::{signature::Keypair, signer::Signer, transport::TransportError};
use spl_governance::state::{proposal::ProposalV2, vote_record::get_vote_record_address};
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
async fn test_reclaim_voter_weight_record() -> TestOutcome {
    // Arrange
    let mut vote_delegation_test = DelegationTest::start_new().await;
    let realm_cookie = vote_delegation_test.governance.with_realm().await?;
    let wallet = vote_delegation_test.bench.with_wallet().await;
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
        .reclaim_voter_weight_record(&wallet, &proposal, &vwr_cookie)
        .await?;

    // Assert
    let voter_weight_record = vote_delegation_test
        .bench
        .get_account(&vwr_cookie.address)
        .await;

    assert!(voter_weight_record.is_none());

    Ok(())
}

#[tokio::test]
async fn test_reclaim_voter_weight_record_early_err() -> TestOutcome {
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
    let err = vote_delegation_test
        .reclaim_voter_weight_record(&wallet, &proposal, &vwr_cookie)
        .await
        .err()
        .unwrap();

    // Assert
    assert_vote_delegation_err(err, DelegationError::ReclaimTargetWrongState);

    Ok(())
}

#[tokio::test]
async fn test_reclaim_delegation_record() -> TestOutcome {
    // Arrange
    let mut vote_delegation_test = DelegationTest::start_new().await;
    let realm_cookie = vote_delegation_test.governance.with_realm().await?;
    let wallet = vote_delegation_test.bench.with_wallet().await;
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
        .reclaim_voter_weight_record(&wallet, &proposal, &vwr_cookie)
        .await?;

    // Act
    vote_delegation_test
        .reclaim_delegation_record(&realm_cookie, &wallet, &vwr_cookie, &delegator)
        .await?;

    // Assert
    let delegation_record = vote_delegation_test
        .bench
        .get_account(&Delegation::get_pda_address(
            &realm_cookie.address,
            &realm_cookie.community_mint_cookie.address,
            &delegator.wallet.address,
            &delegator.source_vwr.target,
            Some(delegator.source_vwr.action),
        ))
        .await;

    assert!(delegation_record.is_none());

    Ok(())
}

#[tokio::test]
async fn test_reclaim_delegation_record_before_vwr_err() -> TestOutcome {
    // Arrange
    let mut vote_delegation_test = DelegationTest::start_new().await;
    let realm_cookie = vote_delegation_test.governance.with_realm().await?;
    let wallet = vote_delegation_test.bench.with_wallet().await;
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
    let err = vote_delegation_test
        .reclaim_delegation_record(&realm_cookie, &wallet, &vwr_cookie, &delegator)
        .await
        .err()
        .unwrap();

    // Assert
    assert_vote_delegation_err(err, DelegationError::CannotReclaimDelegationRecordYet);

    Ok(())
}
