#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_automation::task_balancer::{ExecuteMsg, InstantiateMsg};
use cosmwasm_std::Empty;
use cw_multi_test::{Contract, ContractWrapper};

pub fn mock_andromeda_task_balancer() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_task_balancer_instantiate_msg(max: u64, storage_code_id: u64) -> InstantiateMsg {
    InstantiateMsg {
        max,
        storage_code_id,
    }
}

pub fn mock_task_balancer_store_msg(process: String) -> ExecuteMsg {
    ExecuteMsg::Add { process }
}

pub fn mock_task_balancer_remove_msg(process: String) -> ExecuteMsg {
    ExecuteMsg::Remove { process }
}
