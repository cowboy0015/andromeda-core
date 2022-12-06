#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_automation::oracle::{InstantiateMsg, QueryMsg, RegularTypes, TypeOfResponse};
use cosmwasm_std::{Binary, Empty};
use cw_multi_test::{Contract, ContractWrapper};

pub fn mock_andromeda_oracle() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_oracle_instantiate_msg(
    target_address: String,
    message_binary: Binary,
    return_type: TypeOfResponse,
    response_element: Option<String>,
) -> InstantiateMsg {
    InstantiateMsg {
        target_address,
        message_binary,
        return_type,
        response_element,
    }
}

pub fn mock_oracle_msg() -> QueryMsg {
    QueryMsg::Target {}
}
