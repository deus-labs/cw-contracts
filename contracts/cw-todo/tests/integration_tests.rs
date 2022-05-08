use cosmwasm_std::{Addr, Coin, Empty, Uint128};
use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};
use todo_list::helpers::CwTemplateContract;
use todo_list::msg::ExecuteMsg;
use todo_list::msg::InstantiateMsg;

pub fn contract_template() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        todo_list::contract::execute,
        todo_list::contract::instantiate,
        todo_list::contract::query,
    );
    Box::new(contract)
}

const USER: &str = "USER";
const ADMIN: &str = "ADMIN";
const NATIVE_DENOM: &str = "denom";

fn mock_app() -> App {
    AppBuilder::new().build(|router, _, storage| {
        router
            .bank
            .init_balance(
                storage,
                &Addr::unchecked(USER),
                vec![Coin {
                    denom: NATIVE_DENOM.to_string(),
                    amount: Uint128::new(1),
                }],
            )
            .unwrap();
    })
}

fn proper_instantiate() -> (App, CwTemplateContract) {
    let mut app = mock_app();
    let cw_template_id = app.store_code(contract_template());

    let msg = InstantiateMsg {};
    let cw_template_contract_addr = app
        .instantiate_contract(
            cw_template_id,
            Addr::unchecked(ADMIN),
            &msg,
            &[],
            "test",
            None,
        )
        .unwrap();

    let cw_template_contract = CwTemplateContract(cw_template_contract_addr);

    (app, cw_template_contract)
}

#[test]
fn add_todo() {
    let (mut app, cw_template_contract) = proper_instantiate();

    let msg = ExecuteMsg::AddTodo {
        description: String::from("This is my awesome description"),
    };
    let cosmos_msg = cw_template_contract.call(msg).unwrap();
    app.execute(Addr::unchecked(USER), cosmos_msg).unwrap();
}
