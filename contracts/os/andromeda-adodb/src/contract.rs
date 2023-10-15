use crate::{execute, query};
use andromeda_std::ado_contract::ADOContract;
use andromeda_std::common::context::ExecuteContext;
use andromeda_std::common::encode_binary;
use andromeda_std::error::{from_semver, ContractError};
use andromeda_std::os::adodb::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use cosmwasm_std::{
    ensure, entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError,
};
use cw2::{get_contract_version, set_contract_version};
use semver::Version;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:andromeda-adodb";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        info,
        andromeda_std::ado_base::InstantiateMsg {
            ado_type: "adodb".to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            operators: None,
            kernel_address: msg.kernel_address,
            owner: msg.owner,
        },
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.result.is_err() {
        return Err(ContractError::Std(StdError::generic_err(
            msg.result.unwrap_err(),
        )));
    }

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let mut execute_env = ExecuteContext::new(deps, info, env);
    match msg {
        ExecuteMsg::UpdateCodeId {
            code_id_key,
            code_id,
        } => execute::update_code_id(&mut execute_env, &code_id_key, code_id),
        ExecuteMsg::Publish {
            code_id,
            ado_type,
            action_fees,
            version,
            publisher,
        } => execute::publish(
            &mut execute_env,
            code_id,
            ado_type,
            version,
            action_fees,
            publisher,
        ),
        ExecuteMsg::UpdateActionFees {
            action_fees,
            ado_type,
        } => execute::update_action_fees(&mut execute_env, ado_type.as_str(), action_fees),
        ExecuteMsg::RemoveActionFees { ado_type, actions } => {
            execute::remove_action_fees(&mut execute_env, ado_type.as_str(), actions)
        }
        ExecuteMsg::UpdatePublisher {
            ado_version,
            publisher,
        } => execute::update_publisher(&mut execute_env, &ado_version, Some(publisher)),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    let storage = deps.storage;
    match msg {
        QueryMsg::CodeId { key } => encode_binary(&query::code_id_from_ado_version(storage, &key)?),
        QueryMsg::ADOType { code_id } => {
            encode_binary(&query::ado_version_from_code_id(storage, code_id)?)
        }
        QueryMsg::AllADOTypes { start_after, limit } => {
            encode_binary(&query::all_ado_types(storage, start_after, limit)?)
        }
        QueryMsg::ADOVersions {
            ado_type,
            start_after,
            limit,
        } => encode_binary(&query::ado_versions(
            storage,
            &ado_type,
            start_after,
            limit,
        )?),
        QueryMsg::ADOPublisher { ado_version } => {
            encode_binary(&query::publisher(storage, &ado_version)?)
        }
        QueryMsg::ADOMetadata { ado_version } => {
            encode_binary(&query::ado_metadata(storage, &ado_version)?)
        }
        QueryMsg::ActionFee { ado_type, action } => {
            encode_binary(&query::action_fee(storage, &ado_type, &action)?)
        }
        QueryMsg::ActionFeeByCodeId { code_id, action } => {
            encode_binary(&query::action_fee_by_code_id(storage, code_id, &action)?)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // New version
    let version: Version = CONTRACT_VERSION.parse().map_err(from_semver)?;

    // Old version
    let stored = get_contract_version(deps.storage)?;
    let storage_version: Version = stored.version.parse().map_err(from_semver)?;

    let contract = ADOContract::default();

    ensure!(
        stored.contract == CONTRACT_NAME,
        ContractError::CannotMigrate {
            previous_contract: stored.contract,
        }
    );

    // New version has to be newer/greater than the old version
    ensure!(
        storage_version < version,
        ContractError::CannotMigrate {
            previous_contract: stored.version,
        }
    );

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Update the ADOContract's version
    contract.execute_update_version(deps)?;

    Ok(Response::default())
}
