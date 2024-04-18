use crate::error::ContractError;
use cosmwasm_std::Reply;
use cw_utils::parse_instantiate_response_data;

pub fn get_reply_address(msg: Reply) -> Result<String, ContractError> {
    let res = parse_instantiate_response_data(&msg.payload)?;
    Ok(res.contract_address)
}
