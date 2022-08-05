use anchor_lang::{
    accounts::orphan::Orphan, prelude::*, solana_program::program_pack::IsInitialized,
};
use num_derive::FromPrimitive;
use spl_governance::state::{
    enums::ProposalState, governance::get_governance_data, proposal::get_proposal_data,
};

use crate::error::DelegationError;

/// VoterWeightAction enum as defined in spl-governance-addin-api
/// It's redefined here for Anchor to export it to IDL
#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, Copy, PartialEq, FromPrimitive)]
pub enum VoterWeightAction {
    /// Cast vote for a proposal. Target: Proposal
    CastVote,

    /// Comment a proposal. Target: Proposal
    CommentProposal,

    /// Create Governance within a realm. Target: Realm
    CreateGovernance,

    /// Create a proposal for a governance. Target: Governance
    CreateProposal,

    /// Signs off a proposal for a governance. Target: Proposal
    /// Note: SignOffProposal is not supported in the current version
    SignOffProposal,

    /// Revoke vote. Target: VoteRecord
    RevokeVote,
}

/// VoterWeightRecord account as defined in spl-governance-addin-api
/// It's redefined here without account_discriminator for Anchor to treat it as native account
///
/// The account is used as an api interface to provide voting power to the governance program from external addin contracts
#[account(account)]
#[derive(Debug, PartialEq)]
pub struct VoterWeightRecord {
    /// The Realm the VoterWeightRecord belongs to
    pub realm: Pubkey,

    /// Governing Token Mint the VoterWeightRecord is associated with
    /// Note: The addin can take deposits of any tokens and is not restricted to the community or council tokens only
    // The mint here is to link the record to either community or council mint of the realm
    pub governing_token_mint: Pubkey,

    /// The owner of the governing token and voter
    /// This is the actual owner (voter) and corresponds to TokenOwnerRecord.governing_token_owner
    pub governing_token_owner: Pubkey,

    /// Voter's weight
    /// The weight of the voter provided by the addin for the given realm, governing_token_mint and governing_token_owner (voter)
    pub voter_weight: u64,

    /// The slot when the voting weight expires
    /// It should be set to None if the weight never expires
    /// If the voter weight decays with time, for example for time locked based weights, then the expiry must be set
    /// As a common pattern Revise instruction to update the weight should be invoked before governance instruction within the same transaction
    /// and the expiry set to the current slot to provide up to date weight
    pub voter_weight_expiry: Option<u64>,

    /// The governance action the voter's weight pertains to
    /// It allows to provided voter's weight specific to the particular action the weight is evaluated for
    /// When the action is provided then the governance program asserts the executing action is the same as specified by the addin
    pub weight_action: Option<VoterWeightAction>,

    /// The target the voter's weight action pertains to
    /// It allows to provided voter's weight specific to the target the weight is evaluated for
    /// For example when addin supplies weight to vote on a particular proposal then it must specify the proposal as the action target
    /// When the target is provided then the governance program asserts the target is the same as specified by the addin
    pub weight_action_target: Option<Pubkey>,

    /// Reserved space for future versions
    pub reserved: [u8; 8],
}

impl Orphan for VoterWeightRecord {}

impl VoterWeightRecord {
    pub fn try_aggregate(&self, other: &VoterWeightRecord) -> Result<u64> {
        require!(
            self.governing_token_mint == other.governing_token_mint,
            DelegationError::InvalidGoverningTokenMint
        );

        require!(self.realm == other.realm, DelegationError::InvalidRealm);

        Ok(self.voter_weight.checked_add(other.voter_weight).unwrap())
    }

    pub fn get_revocation_address(
        realm: &Pubkey,
        governing_token_mint: &Pubkey,
        governing_token_owner: &Pubkey,
        target: &Pubkey,
        action: Option<VoterWeightAction>,
    ) -> Pubkey {
        Pubkey::try_find_program_address(
            &[
                b"revocation".as_ref(),
                realm.as_ref(),
                governing_token_mint.as_ref(),
                governing_token_owner.as_ref(),
                target.as_ref(),
                &borsh::to_vec(&action).unwrap(),
            ],
            &crate::id(),
        )
        .unwrap()
        .0
    }

    pub fn get_pda_address(
        realm: &Pubkey,
        governing_token_mint: &Pubkey,
        governing_token_owner: &Pubkey,
        target: &Pubkey,
        action: Option<VoterWeightAction>,
    ) -> Pubkey {
        Pubkey::try_find_program_address(
            &VoterWeightRecord::get_pda_seeds(
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
            b"voter-weight-record".as_ref(),
            realm.as_ref(),
            governing_token_mint.as_ref(),
            governing_token_owner.as_ref(),
            target.as_ref(),
            action,
        ]
    }

    pub fn assert_can_reclaim(
        &self,
        target: &AccountInfo,
        governance_program_id: &Pubkey,
    ) -> Result<()> {
        match self.weight_action.unwrap() {
            VoterWeightAction::CastVote | VoterWeightAction::CommentProposal => {
                if let Ok(proposal) = get_proposal_data(governance_program_id, target) {
                    require!(
                        proposal.state == ProposalState::Completed,
                        DelegationError::ReclaimTargetWrongState
                    );
                }
            }
            VoterWeightAction::SignOffProposal => {
                if let Ok(proposal) = get_proposal_data(governance_program_id, target) {
                    require!(
                        proposal.assert_can_sign_off().is_err(),
                        DelegationError::ReclaimTargetWrongState
                    );
                }
            }
            VoterWeightAction::CreateGovernance => {
                let governance = get_governance_data(governance_program_id, target);
                require!(governance.is_ok(), DelegationError::ReclaimTargetWrongState);
            }
            VoterWeightAction::CreateProposal => {}
            VoterWeightAction::RevokeVote => {}
        };

        Ok(())
    }
}

impl Default for VoterWeightRecord {
    fn default() -> Self {
        Self {
            realm: Default::default(),
            governing_token_mint: Default::default(),
            governing_token_owner: Default::default(),
            voter_weight: Default::default(),
            voter_weight_expiry: Some(0),
            weight_action: Some(VoterWeightAction::CastVote),
            weight_action_target: Some(Default::default()),
            reserved: Default::default(),
        }
    }
}

impl IsInitialized for VoterWeightRecord {
    fn is_initialized(&self) -> bool {
        self.realm != Default::default()
            && self.governing_token_mint != Default::default()
            && self.governing_token_owner != Default::default()
    }
}
