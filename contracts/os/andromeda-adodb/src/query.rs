use andromeda_std::{
    error::ContractError,
    os::adodb::{ADOMetadata, ADOVersion, ActionFee},
};
use cosmwasm_std::{ensure, Addr, Order, StdResult, Storage};
use cw_storage_plus::Bound;
use semver::Version;

use crate::state::{ACTION_FEES, ADO_TYPE, CODE_ID, LATEST_VERSION, METADATA, PUBLISHER};
const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 100;

pub fn code_id_from_ado_version(
    storage: &dyn Storage,
    ado_version: &str,
) -> Result<u64, ContractError> {
    let ado_version = get_strict_ado(storage, ado_version)?;
    let code_id = CODE_ID.load(storage, ado_version.as_str())?;
    Ok(code_id)
}

pub fn latest_version(storage: &dyn Storage, ado_type: &str) -> Result<String, ContractError> {
    let ado_type = ADOVersion::from_string(ado_type).get_type();
    let latest = LATEST_VERSION.load(storage, &ado_type)?;
    Ok(latest)
}

pub fn publisher(storage: &dyn Storage, ado_version: &str) -> Result<Addr, ContractError> {
    let ado_version = get_strict_ado(storage, ado_version)?;
    let publisher = PUBLISHER.load(storage, ado_version.as_str())?;
    Ok(publisher)
}

pub fn ado_version_from_code_id(
    storage: &dyn Storage,
    code_id: u64,
) -> Result<Option<String>, ContractError> {
    let ado_version = ADO_TYPE.may_load(storage, code_id)?;
    Ok(ado_version)
}

pub fn all_ado_types(
    storage: &dyn Storage,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<String>, ContractError> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(|s| Bound::ExclusiveRaw(s.into()));

    let ado_types: StdResult<Vec<String>> = CODE_ID
        .keys(storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| item.map(|k| k))
        .collect();
    Ok(ado_types?)
}

pub fn ado_versions(
    storage: &dyn Storage,
    ado_type: &str,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<String>, ContractError> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start_after = start_after.unwrap_or(ado_type.to_string());
    let start = Some(Bound::exclusive(start_after.as_str()));

    // All versions have @ as starting point, we can add A which has higher ascii than @ to get the
    let end_ado_type = format!("{ado_type}A");
    let end = Some(Bound::exclusive(end_ado_type.as_str()));

    let mut versions: Vec<String> = CODE_ID
        .keys(storage, start, end, Order::Ascending)
        .take(limit)
        .map(|item| item.unwrap())
        .collect();
    versions.sort_by(|a, b| {
        let version_a: Version = ADOVersion::from_string(a).get_version().parse().unwrap();
        let version_b: Version = ADOVersion::from_string(b).get_version().parse().unwrap();
        version_b.cmp(&version_a)
    });
    Ok(versions)
}

pub fn ado_metadata(
    storage: &dyn Storage,
    ado_version: &str,
) -> Result<ADOMetadata, ContractError> {
    let ado_version = ADOVersion::from_string(ado_version);
    let meta = METADATA.load(storage, ado_version.as_str())?;
    Ok(meta)
}

pub fn action_fee(
    storage: &dyn Storage,
    ado_version: &str,
    action: &str,
) -> Result<Option<ActionFee>, ContractError> {
    let ado_version = ADOVersion::from_string(ado_version);
    Ok(ACTION_FEES.may_load(storage, &(ado_version.into_string(), action.to_string()))?)
}

pub fn action_fee_by_code_id(
    storage: &dyn Storage,
    code_id: u64,
    action: &str,
) -> Result<Option<ActionFee>, ContractError> {
    let ado_version = ADO_TYPE.load(storage, code_id)?;
    Ok(ACTION_FEES.may_load(storage, &(ado_version, action.to_string()))?)
}

// Check if ado exists with proper versioning in the storage
pub fn ado_exists(storage: &dyn Storage, ado_version: &ADOVersion) -> Result<(), ContractError> {
    ensure!(
        ado_version.validate_strict()
            && code_id_from_ado_version(storage, ado_version.as_str()).is_ok(),
        ContractError::InvalidADOVersion {
            msg: Some("ADO type does not exist".to_string())
        }
    );
    Ok(())
}

fn get_strict_ado(storage: &dyn Storage, ado_version: &str) -> Result<ADOVersion, ContractError> {
    let mut ado_version = ADOVersion::from_string(ado_version);
    if ado_version.get_version() == ADOVersion::LATEST {
        let latest = latest_version(storage, &ado_version.get_type())?;
        ado_version = ado_version.with_version(latest);
    }
    Ok(ado_version)
}
