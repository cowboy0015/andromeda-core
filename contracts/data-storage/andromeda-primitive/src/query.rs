use crate::state::{DATA, DEFAULT_KEY, KEY_OWNER, RESTRICTION};
use andromeda_data_storage::primitive::{GetValueResponse, PrimitiveRestriction};
use andromeda_std::{ado_contract::ADOContract, amp::AndrAddr, error::ContractError};
use cosmwasm_std::{Addr, Deps, StdResult, Storage};
use cw_storage_plus::Bound;

const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 100;

pub fn get_key_or_default(name: &Option<String>) -> &str {
    match name {
        None => DEFAULT_KEY,
        Some(s) => s,
    }
}

pub fn has_key_permission(
    storage: &dyn Storage,
    addr: &Addr,
    key: &str,
) -> Result<bool, ContractError> {
    let is_operator = ADOContract::default().is_owner_or_operator(storage, addr.as_str())?;
    let allowed = match RESTRICTION.load(storage)? {
        PrimitiveRestriction::Private => is_operator,
        PrimitiveRestriction::Public => true,
        PrimitiveRestriction::Restricted => match KEY_OWNER.load(storage, key).ok() {
            Some(owner) => addr == owner,
            None => true,
        },
    };
    Ok(is_operator || allowed)
}

pub fn all_keys(
    storage: &dyn Storage,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<String>, ContractError> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(|s| Bound::ExclusiveRaw(s.into()));

    let keys: StdResult<Vec<String>> = DATA
        .keys(storage, start, None, cosmwasm_std::Order::Ascending)
        .take(limit)
        .collect();
    Ok(keys?)
}

pub fn owner_keys(
    deps: &Deps,
    owner: AndrAddr,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<String>, ContractError> {
    let owner = owner.get_raw_address(deps)?;
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(|s| Bound::ExclusiveRaw(s.into()));

    let keys = KEY_OWNER
        .range(deps.storage, start, None, cosmwasm_std::Order::Ascending)
        .filter(|x| x.as_ref().unwrap().1 == owner)
        .take(limit)
        .map(|key| key.unwrap().0)
        .collect();
    Ok(keys)
}

pub fn get_value(
    storage: &dyn Storage,
    key: Option<String>,
) -> Result<GetValueResponse, ContractError> {
    let key = get_key_or_default(&key);
    let value = DATA.load(storage, key)?;
    Ok(GetValueResponse {
        key: key.to_string(),
        value,
    })
}
