use anchor_lang::prelude::Pubkey;
use program_test::{delegation_test::DelegationTest, tools::assert_vote_delegation_err};
use solana_program::{instruction::Instruction, program_pack::IsInitialized, vote};
use solana_program_test::tokio;
use solana_sdk::{signature::Keypair, signer::Signer, transport::TransportError};
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
    let token_owner_record = vote_delegation_test
        .governance
        .with_token_owner_record(&realm_cookie, &wallet)
        .await?;
    let fake_proposal = Keypair::new();
    let vwr_cookie = vote_delegation_test
        .with_vwr(
            &realm_cookie,
            &wallet,
            fake_proposal.pubkey(),
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
            fake_proposal.pubkey(),
        )
        .await?;

    // Act
    vote_delegation_test
        .aggregate_delegation(&realm_cookie, &wallet, &vwr_cookie, &[&delegator])
        .await?;

    // Assert
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
    assert_eq!(
        vwr_record.weight_action_target,
        Some(fake_proposal.pubkey())
    );
    assert_ne!(vwr_record.voter_weight_expiry, Some(0));
    assert_eq!(vwr_record.voter_weight, 10);

    Ok(())
}

#[tokio::test]
async fn test_update_voter_weight_record_with_multiple_delegators() -> TestOutcome {
    // Arrange
    let mut vote_delegation_test = DelegationTest::start_new().await;
    let realm_cookie = vote_delegation_test.governance.with_realm().await?;
    let wallet = vote_delegation_test.bench.with_wallet().await;
    let token_owner_record = vote_delegation_test
        .governance
        .with_token_owner_record(&realm_cookie, &wallet)
        .await?;
    let fake_proposal = Keypair::new();
    let vwr_cookie = vote_delegation_test
        .with_vwr(
            &realm_cookie,
            &wallet,
            fake_proposal.pubkey(),
            VoterWeightAction::CastVote,
        )
        .await?;
    let precursor_cookie = vote_delegation_test
        .with_precursor_program(&realm_cookie)
        .await?;

    let delegator1 = vote_delegation_test
        .with_delegator(
            &realm_cookie,
            &precursor_cookie,
            wallet.address,
            10,
            Some(u64::max_value()),
            VoterWeightAction::CastVote,
            fake_proposal.pubkey(),
        )
        .await?;

    let delegator2 = vote_delegation_test
        .with_delegator(
            &realm_cookie,
            &precursor_cookie,
            wallet.address,
            10,
            Some(u64::max_value()),
            VoterWeightAction::CastVote,
            fake_proposal.pubkey(),
        )
        .await?;

    // Act
    vote_delegation_test
        .aggregate_delegation(
            &realm_cookie,
            &wallet,
            &vwr_cookie,
            &[&delegator1, &delegator2],
        )
        .await?;

    // Assert
    let vwr_record = vote_delegation_test
        .bench
        .get_anchor_account::<VoterWeightRecord>(vwr_cookie.address)
        .await;

    for delegator in &[delegator1, delegator2] {
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
    }

    assert_eq!(vwr_record.weight_action, Some(VoterWeightAction::CastVote));
    assert_eq!(
        vwr_record.weight_action_target,
        Some(fake_proposal.pubkey())
    );
    assert_ne!(vwr_record.voter_weight_expiry, Some(0));
    assert_eq!(vwr_record.voter_weight, 20);

    Ok(())
}

#[tokio::test]
async fn test_update_voter_weight_record_with_incomplete_delegate() -> TestOutcome {
    // Arrange
    let mut vote_delegation_test = DelegationTest::start_new().await;
    let realm_cookie = vote_delegation_test.governance.with_realm().await?;
    let wallet = vote_delegation_test.bench.with_wallet().await;
    let token_owner_record = vote_delegation_test
        .governance
        .with_token_owner_record(&realm_cookie, &wallet)
        .await?;
    let fake_proposal = Keypair::new();
    let vwr_cookie = vote_delegation_test
        .with_vwr(
            &realm_cookie,
            &wallet,
            fake_proposal.pubkey(),
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
            fake_proposal.pubkey(),
        )
        .await?;

    let remove_account = |ixn: &mut Instruction| {
        ixn.accounts.pop();
    };

    // Act
    let err = vote_delegation_test
        .aggregate_delegation_using_ix(
            &realm_cookie,
            &wallet,
            &vwr_cookie,
            &[&delegator],
            remove_account,
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_vote_delegation_err(err, DelegationError::MissingDelegatorAccounts);

    Ok(())
}

#[tokio::test]
async fn test_update_voter_weight_record_with_non_delegating_user() -> TestOutcome {
    // Arrange
    let mut vote_delegation_test = DelegationTest::start_new().await;
    let realm_cookie = vote_delegation_test.governance.with_realm().await?;
    let wallet = vote_delegation_test.bench.with_wallet().await;
    let token_owner_record = vote_delegation_test
        .governance
        .with_token_owner_record(&realm_cookie, &wallet)
        .await?;
    let fake_proposal = Keypair::new();
    let vwr_cookie = vote_delegation_test
        .with_vwr(
            &realm_cookie,
            &wallet,
            fake_proposal.pubkey(),
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
            Pubkey::new_unique(),
            10,
            Some(u64::max_value()),
            VoterWeightAction::CastVote,
            fake_proposal.pubkey(),
        )
        .await?;

    // Act
    let err = vote_delegation_test
        .aggregate_delegation(&realm_cookie, &wallet, &vwr_cookie, &[&delegator])
        .await
        .err()
        .unwrap();

    // Assert
    assert_vote_delegation_err(err, DelegationError::VoterWeightNotDelegatedToDelegate);

    Ok(())
}

#[tokio::test]
async fn test_update_voter_weight_record_with_duplicate_delegators_err() -> TestOutcome {
    // Arrange
    let mut vote_delegation_test = DelegationTest::start_new().await;
    let realm_cookie = vote_delegation_test.governance.with_realm().await?;
    let wallet = vote_delegation_test.bench.with_wallet().await;
    let token_owner_record = vote_delegation_test
        .governance
        .with_token_owner_record(&realm_cookie, &wallet)
        .await?;
    let fake_proposal = Keypair::new();
    let vwr_cookie = vote_delegation_test
        .with_vwr(
            &realm_cookie,
            &wallet,
            fake_proposal.pubkey(),
            VoterWeightAction::CastVote,
        )
        .await?;
    let precursor_cookie = vote_delegation_test
        .with_precursor_program(&realm_cookie)
        .await?;

    let delegator1 = vote_delegation_test
        .with_delegator(
            &realm_cookie,
            &precursor_cookie,
            wallet.address,
            10,
            Some(u64::max_value()),
            VoterWeightAction::CastVote,
            fake_proposal.pubkey(),
        )
        .await?;

    // Act
    let err = vote_delegation_test
        .aggregate_delegation(
            &realm_cookie,
            &wallet,
            &vwr_cookie,
            &[&delegator1, &delegator1],
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_vote_delegation_err(err, DelegationError::VoterWeightAlreadyDelegated);

    Ok(())
}

#[tokio::test]
async fn test_repeat_update_voter_weight_record_err() -> TestOutcome {
    // Arrange
    let mut vote_delegation_test = DelegationTest::start_new().await;
    let realm_cookie = vote_delegation_test.governance.with_realm().await?;
    let wallet = vote_delegation_test.bench.with_wallet().await;
    let token_owner_record = vote_delegation_test
        .governance
        .with_token_owner_record(&realm_cookie, &wallet)
        .await?;
    let fake_proposal = Keypair::new();
    let vwr_cookie = vote_delegation_test
        .with_vwr(
            &realm_cookie,
            &wallet,
            fake_proposal.pubkey(),
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
            fake_proposal.pubkey(),
        )
        .await?;

    vote_delegation_test
        .aggregate_delegation(&realm_cookie, &wallet, &vwr_cookie, &[&delegator])
        .await?;
    vote_delegation_test.bench.advance_clock().await;

    // Act
    let err = vote_delegation_test
        .aggregate_delegation(&realm_cookie, &wallet, &vwr_cookie, &[&delegator])
        .await
        .err()
        .unwrap();

    // Assert
    assert_vote_delegation_err(err, DelegationError::VoterWeightAlreadyDelegated);

    Ok(())
}

#[tokio::test]
async fn test_aggregate_vwr_from_wrong_source_err() -> TestOutcome {
    // Arrange
    let mut vote_delegation_test = DelegationTest::start_new().await;
    let realm_cookie = vote_delegation_test.governance.with_realm().await?;
    let wallet = vote_delegation_test.bench.with_wallet().await;
    let token_owner_record = vote_delegation_test
        .governance
        .with_token_owner_record(&realm_cookie, &wallet)
        .await?;
    let fake_proposal = Keypair::new();
    let vwr_cookie = vote_delegation_test
        .with_vwr(
            &realm_cookie,
            &wallet,
            fake_proposal.pubkey(),
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
            fake_proposal.pubkey(),
        )
        .await?;

    let delegator_vwr_account = vote_delegation_test
        .bench
        .get_account(&delegator.source_vwr.address)
        .await
        .unwrap();
    vote_delegation_test
        .bench
        .set_account(
            delegator_vwr_account.data,
            delegator.source_vwr.address,
            Pubkey::new_unique(),
        )
        .await?;

    // Act
    let err = vote_delegation_test
        .aggregate_delegation(&realm_cookie, &wallet, &vwr_cookie, &[&delegator])
        .await
        .err()
        .unwrap();

    // Assert
    assert_vote_delegation_err(err, DelegationError::InvalidVoterWeightRecordSource);

    Ok(())
}
