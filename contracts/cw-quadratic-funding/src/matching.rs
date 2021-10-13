use crate::error::ContractError;

use integer_sqrt::IntegerSquareRoot;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QuadraticFundingAlgorithm {
    CapitalConstrainedLiberalRadicalism { parameter: String },
}

#[derive(Clone, Debug, PartialEq)]
pub struct RawGrant {
    pub addr: String,
    pub funds: Vec<u128>,
    pub collected_vote_funds: u128,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CalculatedGrant {
    pub addr: String,
    pub grant: u128,
    pub collected_vote_funds: u128,
}

type LeftOver = u128;

pub fn calculate_clr(
    grants: Vec<RawGrant>,
    budget: Option<u128>,
) -> Result<(Vec<CalculatedGrant>, LeftOver), ContractError> {
    // clr algorithm works with budget constrain
    if let Some(budget) = budget {
        // calculate matches sum
        let matched = calculate_matched_sum(grants);

        // constraint the grants by budget
        let constrained = constrain_by_budget(matched, budget);

        let constrained_sum: u128 = constrained.iter().map(|c| c.grant).sum();
        // calculate leftover
        // shouldn't be used with tokens with > 10 decimal points
        // will cause overflow and panic on the during execution.
        let leftover = budget - constrained_sum;

        Ok((constrained, leftover))
    } else {
        Err(ContractError::CLRConstrainRequired {})
    }
}

// takes square root of each fund, sums, then squares and returns u128
fn calculate_matched_sum(grants: Vec<RawGrant>) -> Vec<CalculatedGrant> {
    grants
        .into_iter()
        .map(|g| {
            let sum_sqrts: u128 = g.funds.into_iter().map(|v| v.integer_sqrt()).sum();
            CalculatedGrant {
                addr: g.addr,
                grant: sum_sqrts * sum_sqrts,
                collected_vote_funds: g.collected_vote_funds,
            }
        })
        .collect()
}

// takes square root of each fund, sums, then squares and returns u128
fn constrain_by_budget(grants: Vec<CalculatedGrant>, budget: u128) -> Vec<CalculatedGrant> {
    let raw_total: u128 = grants.iter().map(|g| g.grant).sum();
    grants
        .into_iter()
        .map(|g| CalculatedGrant {
            addr: g.addr,
            grant: (g.grant * budget) / raw_total,
            collected_vote_funds: g.collected_vote_funds,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::matching::{calculate_clr, CalculatedGrant, RawGrant};
    use crate::state::Proposal;

    #[test]
    fn test_clr_1() {
        let proposal1 = Proposal {
            fund_address: "proposal1".to_string(),
            ..Default::default()
        };
        let proposal2 = Proposal {
            fund_address: "proposal2".into(),
            ..Default::default()
        };
        let proposal3 = Proposal {
            fund_address: "proposal3".into(),
            ..Default::default()
        };
        let proposal4 = Proposal {
            fund_address: "proposal4".into(),
            ..Default::default()
        };
        let votes1 = vec![7200u128];
        let votes2 = vec![12345u128];
        let votes3 = vec![4456u128];
        let votes4 = vec![60000u128];

        let grants = vec![
            RawGrant {
                addr: proposal1.fund_address.clone(),
                funds: votes1.clone(),
                collected_vote_funds: votes1.iter().sum(),
            },
            RawGrant {
                addr: proposal2.fund_address.clone(),
                funds: votes2.clone(),
                collected_vote_funds: votes2.iter().sum(),
            },
            RawGrant {
                addr: proposal3.fund_address.clone(),
                funds: votes3.clone(),
                collected_vote_funds: votes3.iter().sum(),
            },
            RawGrant {
                addr: proposal4.fund_address.clone(),
                funds: votes4.clone(),
                collected_vote_funds: votes4.iter().sum(),
            },
        ];
        let expected = vec![
            CalculatedGrant {
                addr: proposal1.fund_address,
                grant: 84737u128,
                collected_vote_funds: 7200u128,
            },
            CalculatedGrant {
                addr: proposal2.fund_address,
                grant: 147966u128,
                collected_vote_funds: 12345u128,
            },
            CalculatedGrant {
                addr: proposal3.fund_address,
                grant: 52312u128,
                collected_vote_funds: 4456u128,
            },
            CalculatedGrant {
                addr: proposal4.fund_address,
                grant: 714983u128,
                collected_vote_funds: 60000u128,
            },
        ];
        let res = calculate_clr(grants, Some(1000000u128));
        match res {
            Ok(o) => {
                assert_eq!(o.0, expected);
                assert_eq!(o.1, 2)
            }
            e => panic!("unexpected error, got {:?}", e),
        }
    }

    // values got from https://wtfisqf.com/?grant=1200,44999,33&grant=30000,58999&grant=230000,100&grant=100000,5&match=550000
    //        expected   got
    // grant1 60673.38   60212
    // grant2 164749.05  164602
    // grant3 228074.05  228537
    // grant4 96503.53   96648
    #[test]
    fn test_clr_2() {
        let proposal1 = Proposal {
            fund_address: "proposal1".to_string(),
            ..Default::default()
        };
        let proposal2 = Proposal {
            fund_address: "proposal2".into(),
            ..Default::default()
        };
        let proposal3 = Proposal {
            fund_address: "proposal3".into(),
            ..Default::default()
        };
        let proposal4 = Proposal {
            fund_address: "proposal4".into(),
            ..Default::default()
        };
        let votes1 = vec![1200u128, 44999u128, 33u128];
        let votes2 = vec![30000u128, 58999u128];
        let votes3 = vec![230000u128, 100u128];
        let votes4 = vec![100000u128, 5u128];

        let grants = vec![
            RawGrant {
                addr: proposal1.fund_address.clone(),
                funds: votes1.clone(),
                collected_vote_funds: votes1.iter().sum(),
            },
            RawGrant {
                addr: proposal2.fund_address.clone(),
                funds: votes2.clone(),
                collected_vote_funds: votes2.iter().sum(),
            },
            RawGrant {
                addr: proposal3.fund_address.clone(),
                funds: votes3.clone(),
                collected_vote_funds: votes3.iter().sum(),
            },
            RawGrant {
                addr: proposal4.fund_address.clone(),
                funds: votes4.clone(),
                collected_vote_funds: votes4.iter().sum(),
            },
        ];
        let expected = vec![
            CalculatedGrant {
                addr: proposal1.fund_address,
                grant: 60212u128,
                collected_vote_funds: votes1.iter().sum(),
            },
            CalculatedGrant {
                addr: proposal2.fund_address,
                grant: 164602u128,
                collected_vote_funds: votes2.iter().sum(),
            },
            CalculatedGrant {
                addr: proposal3.fund_address,
                grant: 228537u128,
                collected_vote_funds: votes3.iter().sum(),
            },
            CalculatedGrant {
                addr: proposal4.fund_address,
                grant: 96648u128,
                collected_vote_funds: votes4.iter().sum(),
            },
        ];
        let res = calculate_clr(grants, Some(550000u128));
        match res {
            Ok(o) => {
                assert_eq!(o.0, expected);
                assert_eq!(o.1, 1)
            }
            e => panic!("unexpected error, got {:?}", e),
        }
    }
}
