#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_automation::evaluation::{ExecuteMsg, InstantiateMsg, Operators, QueryMsg};
use common::app::AndrAddress;
use cosmwasm_std::{Empty, Uint128};
use cw_multi_test::{Contract, ContractWrapper};

pub fn mock_andromeda_evaluation() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_evaluation_instantiate_msg(
    condition_address: AndrAddressm,
    oracle_address: AndrAddress,
    task_balancer: AndrAddress,
    user_value: Option<Uint128>,
    operation: Operators,
) -> InstantiateMsg {
    InstantiateMsg {
        condition_address,
        oracle_address,
        task_balancer,
        user_value,
        operation,
    }
}

pub fn mock_evaluation_msg() -> QueryMsg {
    QueryMsg::Evaluation {}
}
