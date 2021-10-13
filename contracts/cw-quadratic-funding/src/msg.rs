use crate::error::ContractError;
use crate::matching::QuadraticFundingAlgorithm;
use crate::state::Proposal;
use cosmwasm_std::{Binary, Env};
use cw0::Expiration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InitMsg {
    pub admin: String,
    pub leftover_addr: String,
    pub create_proposal_whitelist: Option<Vec<String>>,
    pub vote_proposal_whitelist: Option<Vec<String>>,
    pub voting_period: Expiration,
    pub proposal_period: Expiration,
    pub budget_denom: String,
    pub algorithm: QuadraticFundingAlgorithm,
}

impl InitMsg {
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    ProposalByID { id: u64 },
    AllProposals {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
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
        let msg = InitMsg {
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
