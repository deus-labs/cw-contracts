use crate::error::ContractError;
use cosmwasm_std::Coin;

// extract budget coin validate against sent_funds.denom
pub fn extract_budget_coin(sent_funds: &[Coin], denom: &str) -> Result<Coin, ContractError> {
    if sent_funds.len() != 1 {
        return Err(ContractError::WrongCoinSent {});
    }
    if sent_funds[0].denom != *denom {
        return Err(ContractError::WrongFundCoin {
            expected: denom.to_string(),
            got: sent_funds[0].denom.clone(),
        });
    }
    Ok(sent_funds[0].clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helper::extract_budget_coin;
    use cosmwasm_std::coin;
    use cosmwasm_std::testing::mock_info;

    #[test]
    fn test_extract_funding_coin() {
        let denom = "denom";
        let c = &[coin(4, denom)];
        let info = mock_info("creator", c);

        let res = extract_budget_coin(&info.funds, denom);
        match res {
            Ok(cc) => assert_eq!(c, &[cc]),
            Err(err) => println!("{:?}", err),
        }
        let info = mock_info("creator", &[coin(4, denom), coin(4, "test")]);

        match extract_budget_coin(&info.funds, denom) {
            Ok(_) => panic!("expected error"),
            Err(ContractError::WrongCoinSent { .. }) => {}
            Err(err) => println!("{:?}", err),
        }
    }
}
