use crate::error::ContractError;
use crate::matching::QuadraticFundingAlgorithm;
use crate::state::Proposal;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Binary, Env};
use cw0::Expiration;

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: String,
    pub leftover_addr: String,
    pub create_proposal_whitelist: Option<Vec<String>>,
    pub vote_proposal_whitelist: Option<Vec<String>>,
    pub voting_period: Expiration,
    pub proposal_period: Expiration,
    pub budget_denom: String,
    pub algorithm: QuadraticFundingAlgorithm,
}

impl InstantiateMsg {
    pub fn validate(&self, env: Env) -> Result<(), ContractError> {
        // check if proposal period is expired
        if self.proposal_period.is_expired(&env.block) {
            return Err(ContractError::ProposalPeriodExpired {});
        }
        // check if voting period is expired
        if self.voting_period.is_expired(&env.block) {
            return Err(ContractError::VotingPeriodExpired {});
        }

        Ok(())
    }
}

#[cw_serde]
pub enum ExecuteMsg {
    CreateProposal {
        title: String,
        description: String,
        metadata: Option<Binary>,
        fund_address: String,
    },
    VoteProposal {
        proposal_id: u64,
    },
    TriggerDistribution {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Proposal)]
    ProposalByID { id: u64 },
    #[returns(AllProposalsResponse)]
    AllProposals {},
}

#[cw_serde]
pub struct AllProposalsResponse {
    pub proposals: Vec<Proposal>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::mock_env;

    #[test]
    fn validate_init_msg() {
        let mut env = mock_env();

        env.block.height = 30;
        let msg = InstantiateMsg {
            admin: Default::default(),
            leftover_addr: Default::default(),
            create_proposal_whitelist: None,
            vote_proposal_whitelist: None,
            voting_period: Default::default(),
            proposal_period: Default::default(),
            budget_denom: "".to_string(),
            algorithm: QuadraticFundingAlgorithm::CapitalConstrainedLiberalRadicalism {
                parameter: "".to_string(),
            },
        };

        let mut msg1 = msg.clone();
        msg1.voting_period = Expiration::AtHeight(15);
        match msg1.validate(env.clone()) {
            Ok(_) => panic!("expected error"),
            Err(ContractError::VotingPeriodExpired {}) => {}
            Err(err) => println!("{:?}", err),
        }

        let mut msg2 = msg.clone();
        msg2.proposal_period = Expiration::AtHeight(15);
        match msg2.validate(env.clone()) {
            Ok(_) => panic!("expected error"),
            Err(ContractError::ProposalPeriodExpired {}) => {}
            Err(err) => println!("{:?}", err),
        }

        match msg.validate(env) {
            Ok(_) => {}
            Err(err) => println!("{:?}", err),
        }
    }
}
