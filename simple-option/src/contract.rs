use cosmwasm_std::{
    to_binary, Api, BankMsg, Binary, Context, Env, Extern, HandleResponse, HumanAddr, InitResponse,
    Querier, StdError, StdResult, Storage,
};

use crate::msg::{ConfigResponse, HandleMsg, InitMsg, QueryMsg};
use crate::state::{config, config_read, State};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    if msg.expires <= env.block.height {
        return Err(StdError::generic_err("Cannot create expired option"));
    }

    let state = State {
        creator: env.message.sender.clone(),
        owner: env.message.sender.clone(),
        collateral: env.message.sent_funds,
        counter_offer: msg.counter_offer,
        expires: msg.expires,
    };

    config(&mut deps.storage).save(&state)?;

    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::Transfer { recipient } => handle_transfer(deps, env, recipient),
        HandleMsg::Execute {} => handle_execute(deps, env),
        HandleMsg::Burn {} => handle_burn(deps, env),
    }
}

pub fn handle_transfer<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    recipient: HumanAddr,
) -> StdResult<HandleResponse> {
    // ensure msg sender is the owner
    let mut state = config(&mut deps.storage).load()?;
    if env.message.sender != state.owner {
        return Err(StdError::unauthorized());
    }

    // set new owner on state
    state.owner = recipient.clone();
    config(&mut deps.storage).save(&state)?;

    let mut res = Context::new();
    res.add_log("action", "transfer");
    res.add_log("owner", recipient);
    Ok(res.into())
}

pub fn handle_execute<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> StdResult<HandleResponse> {
    // ensure msg sender is the owner
    let state = config(&mut deps.storage).load()?;
    if env.message.sender != state.owner {
        return Err(StdError::unauthorized());
    }

    // ensure not expired
    if env.block.height >= state.expires {
        return Err(StdError::generic_err("option expired"));
    }

    // ensure sending proper counter_offer
    if env.message.sent_funds != state.counter_offer {
        return Err(StdError::generic_err(format!(
            "must send exact counter offer: {:?}",
            state.counter_offer
        )));
    }

    // release counter_offer to creator
    let mut res = Context::new();
    res.add_message(BankMsg::Send {
        from_address: env.contract.address.clone(),
        to_address: state.creator,
        amount: state.counter_offer,
    });

    // release collateral to sender
    res.add_message(BankMsg::Send {
        from_address: env.contract.address,
        to_address: state.owner,
        amount: state.collateral,
    });

    // delete the option
    config(&mut deps.storage).remove();

    res.add_log("action", "execute");
    Ok(res.into())
}

pub fn handle_burn<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> StdResult<HandleResponse> {
    // ensure is expired
    let state = config(&mut deps.storage).load()?;
    if env.block.height < state.expires {
        return Err(StdError::generic_err("option not yet expired"));
    }

    // ensure sending proper counter_offer
    if !env.message.sent_funds.is_empty() {
        return Err(StdError::generic_err("don't send funds with burn"));
    }

    // release collateral to creator
    let mut res = Context::new();
    res.add_message(BankMsg::Send {
        from_address: env.contract.address,
        to_address: state.creator,
        amount: state.collateral,
    });

    // delete the option
    config(&mut deps.storage).remove();

    res.add_log("action", "burn");
    Ok(res.into())
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
    }
}

fn query_config<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<ConfigResponse> {
    let state = config_read(&deps.storage).load()?;
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{coins, log, CosmosMsg};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg {
            counter_offer: coins(40, "ETH"),
            expires: 100_000,
        };
        let env = mock_env("creator", &coins(1, "BTC"));

        // we can just call .unwrap() to assert this was a success
        let res = init(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query_config(&deps).unwrap();
        assert_eq!(100_000, res.expires);
        assert_eq!("creator", res.owner.as_str());
        assert_eq!("creator", res.creator.as_str());
        assert_eq!(coins(1, "BTC"), res.collateral);
        assert_eq!(coins(40, "ETH"), res.counter_offer);
    }

    #[test]
    fn transfer() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg {
            counter_offer: coins(40, "ETH"),
            expires: 100_000,
        };
        let env = mock_env("creator", &coins(1, "BTC"));

        // we can just call .unwrap() to assert this was a success
        let res = init(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // random cannot transfer
        let env = mock_env("anyone", &[]);
        let err = handle_transfer(&mut deps, env, HumanAddr::from("anyone")).unwrap_err();
        match err {
            StdError::Unauthorized { .. } => {}
            e => panic!("unexpected error: {}", e),
        }

        // owner can transfer
        let env = mock_env("creator", &[]);
        let res = handle_transfer(&mut deps, env, HumanAddr::from("someone")).unwrap();
        assert_eq!(res.log.len(), 2);
        assert_eq!(res.log[0], log("action", "transfer"));

        // check updated properly
        let res = query_config(&deps).unwrap();
        assert_eq!("someone", res.owner.as_str());
        assert_eq!("creator", res.creator.as_str());
    }

    #[test]
    fn execute() {
        let mut deps = mock_dependencies(20, &[]);

        let counter_offer = coins(40, "ETH");
        let collateral = coins(1, "BTC");
        let msg = InitMsg {
            counter_offer: counter_offer.clone(),
            expires: 100_000,
        };
        let env = mock_env("creator", &collateral);

        // we can just call .unwrap() to assert this was a success
        let _ = init(&mut deps, env, msg).unwrap();

        // set new owner
        let env = mock_env("creator", &[]);
        let _ = handle_transfer(&mut deps, env, HumanAddr::from("owner")).unwrap();

        // random cannot execute
        let env = mock_env("creator", &counter_offer);
        let err = handle_execute(&mut deps, env).unwrap_err();
        match err {
            StdError::Unauthorized { .. } => {}
            e => panic!("unexpected error: {}", e),
        }

        // expired cannot execute
        let mut env = mock_env("owner", &counter_offer);
        env.block.height = 200_000;
        let err = handle_execute(&mut deps, env).unwrap_err();
        match err {
            StdError::GenericErr { msg, .. } => assert_eq!("option expired", msg.as_str()),
            e => panic!("unexpected error: {}", e),
        }

        // bad counter_offer cannot execute
        let env = mock_env("owner", &coins(39, "ETH"));
        let err = handle_execute(&mut deps, env).unwrap_err();
        match err {
            StdError::GenericErr { msg, .. } => assert!(msg.contains("counter offer")),
            e => panic!("unexpected error: {}", e),
        }

        // proper execution
        let env = mock_env("owner", &counter_offer);
        let res = handle_execute(&mut deps, env).unwrap();
        assert_eq!(res.messages.len(), 2);
        assert_eq!(
            res.messages[0],
            CosmosMsg::Bank(BankMsg::Send {
                from_address: MOCK_CONTRACT_ADDR.into(),
                to_address: "creator".into(),
                amount: counter_offer,
            })
        );
        assert_eq!(
            res.messages[1],
            CosmosMsg::Bank(BankMsg::Send {
                from_address: MOCK_CONTRACT_ADDR.into(),
                to_address: "owner".into(),
                amount: collateral,
            })
        );

        // check deleted
        let _ = query_config(&deps).unwrap_err();
    }

    #[test]
    fn burn() {
        let mut deps = mock_dependencies(20, &[]);

        let counter_offer = coins(40, "ETH");
        let collateral = coins(1, "BTC");
        let msg = InitMsg {
            counter_offer: counter_offer.clone(),
            expires: 100_000,
        };
        let env = mock_env("creator", &collateral);

        // we can just call .unwrap() to assert this was a success
        let _ = init(&mut deps, env, msg).unwrap();

        // set new owner
        let env = mock_env("creator", &[]);
        let _ = handle_transfer(&mut deps, env, HumanAddr::from("owner")).unwrap();

        // non-expired cannot execute
        let env = mock_env("anyone", &[]);
        let err = handle_burn(&mut deps, env).unwrap_err();
        match err {
            StdError::GenericErr { msg, .. } => assert_eq!("option not yet expired", msg.as_str()),
            e => panic!("unexpected error: {}", e),
        }

        // with funds cannot execute
        let mut env = mock_env("anyone", &counter_offer);
        env.block.height = 200_000;
        let err = handle_burn(&mut deps, env).unwrap_err();
        match err {
            StdError::GenericErr { msg, .. } => {
                assert_eq!("don't send funds with burn", msg.as_str())
            }
            e => panic!("unexpected error: {}", e),
        }

        // expired returns funds
        let mut env = mock_env("anyone", &[]);
        env.block.height = 200_000;
        let res = handle_burn(&mut deps, env).unwrap();
        assert_eq!(res.messages.len(), 1);
        assert_eq!(
            res.messages[0],
            CosmosMsg::Bank(BankMsg::Send {
                from_address: MOCK_CONTRACT_ADDR.into(),
                to_address: "creator".into(),
                amount: collateral,
            })
        );

        // check deleted
        let _ = query_config(&deps).unwrap_err();
    }
}
