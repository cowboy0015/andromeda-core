#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_automation::execute::{ExecuteMsg, InstantiateMsg};
use common::app::AndrAddress;
use cosmwasm_std::{Binary, Empty};
use cw_multi_test::{Contract, ContractWrapper};

pub fn mock_andromeda_execute() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_execute_instantiate_msg(
    target_address: AndrAddress,
    condition_address: AndrAddress,
    task_balancer: String,
    target_message: Binary,
) -> InstantiateMsg {
    InstantiateMsg {
        target_address,
        condition_address,
        task_balancer,
        target_message,
    }
}

pub fn mock_execute_msg() -> ExecuteMsg {
    ExecuteMsg::Execute {}
}
