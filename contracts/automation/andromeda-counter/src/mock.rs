#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_automation::counter::{ExecuteMsg, InstantiateMsg};
use common::app::AndrAddress;
use cosmwasm_std::Empty;
use cw_multi_test::{Contract, ContractWrapper};

pub fn mock_andromeda_counter() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_counter_instantiate_msg(whitelist: Vec<AndrAddress>) -> InstantiateMsg {
    InstantiateMsg { whitelist }
}

pub fn mock_counter_increment_one_msg() -> ExecuteMsg {
    ExecuteMsg::IncrementOne {}
}

pub fn mock_counter_increment_two_msg() -> ExecuteMsg {
    ExecuteMsg::IncrementTwo {}
}

pub fn mock_counter_reset_msg() -> ExecuteMsg {
    ExecuteMsg::Reset {}
}
