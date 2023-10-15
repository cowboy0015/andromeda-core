use andromeda_std::{
    ado_contract::ADOContract,
    common::context::ExecuteContext,
    error::ContractError,
    os::adodb::{ADOMetadata, ADOVersion, ActionFee},
};
use cosmwasm_std::{ensure, Addr, Attribute, Response, StdError};
use semver::Version;

use crate::{
    attributes::Attributes,
    query,
    state::{ACTION_FEES, ADO_TYPE, CODE_ID, LATEST_VERSION, METADATA, PUBLISHER},
};

pub fn publish(
    execute_env: &mut ExecuteContext,
    code_id: u64,
    ado_type: String,
    version: String,
    action_fees: Option<Vec<ActionFee>>,
    publisher: Option<Addr>,
) -> Result<Response, ContractError> {
    let storage = &execute_env.deps.storage;
    ensure!(
        ADOContract::default().is_owner_or_operator(*storage, execute_env.info.sender.as_str())?,
        ContractError::Unauthorized {}
    );
    let ado_version = ADOVersion::from_type(ado_type).with_version(version);
    ensure!(
        !ADO_TYPE.has(*storage, code_id),
        ContractError::InvalidADOVersion {
            msg: Some("Code Id already exists".into())
        }
    );
    ensure!(
        !CODE_ID.has(*storage, ado_version.as_str()),
        ContractError::InvalidADOVersion {
            msg: Some("Version already exists".into())
        }
    );

    let mut attributes: Vec<Attribute> = vec![];

    // Store the other code details code id , ado type and publisher
    let mut store_res = store_code_id(execute_env, &ado_version, code_id)?;
    attributes.append(&mut store_res.attributes);

    // Add Publisher
    let mut publisher_res = update_publisher(execute_env, &ado_version.as_str(), publisher)?;
    attributes.append(&mut publisher_res.attributes);

    // Update action fees
    if let Some(fees) = action_fees {
        let mut update_res = update_action_fees(execute_env, &ado_version.as_str(), fees)?;
        attributes.append(&mut update_res.attributes);
    }

    Ok(Response::default()
        .add_attributes(Attributes::publish())
        .add_attributes(attributes))
}

// This is only for owners to update the wrong code ids
pub fn update_code_id(
    execute_env: &mut ExecuteContext,
    code_id_key: &str,
    code_id: u64,
) -> Result<Response, ContractError> {
    let storage = &mut execute_env.deps.storage;
    ensure!(
        ADOContract::default().is_owner_or_operator(*storage, execute_env.info.sender.as_str())?,
        ContractError::Unauthorized {}
    );
    ensure!(
        CODE_ID.has(*storage, code_id_key),
        ContractError::InvalidADOVersion {
            msg: Some("Code Id already exists".into())
        }
    );
    ensure!(
        !ADO_TYPE.has(*storage, code_id),
        ContractError::InvalidADOVersion {
            msg: Some("Code Id already exists".into())
        }
    );
    let ado_version = ADOVersion::from_string(code_id_key);

    // Remove previous entry for this ado version, it will throw error if the ado didn't exist
    let prev_code_id = query::code_id_from_ado_version(*storage, ado_version.as_str()).unwrap();
    ADO_TYPE.remove(*storage, prev_code_id);

    let mut attributes: Vec<Attribute> = vec![];

    let mut store_res = store_code_id(execute_env, &ado_version, code_id)?;
    attributes.append(&mut store_res.attributes);

    Ok(Response::default()
        .add_attributes(Attributes::update_code_id(ado_version.as_str(), code_id))
        .add_attributes(attributes))
}

pub fn update_publisher(
    execute_env: &mut ExecuteContext,
    ado_version: &str,
    publisher: Option<Addr>,
) -> Result<Response, ContractError> {
    let storage = &mut execute_env.deps.storage;
    ensure!(
        CODE_ID.has(*storage, ado_version),
        ContractError::InvalidADOVersion {
            msg: Some("Ado doen't exist".into())
        }
    );
    // Set publisher as sender if publisher is not provided
    let publisher = publisher.unwrap_or(execute_env.info.sender.clone());

    // Validate publisher
    execute_env.deps.api.addr_validate(publisher.as_str())?;

    PUBLISHER.save(*storage, ado_version, &publisher)?;
    Ok(Response::default().add_attributes(Attributes::update_publisher(publisher.as_str())))
}

pub fn update_action_fees(
    execute_env: &mut ExecuteContext,
    ado_version: &str,
    fees: Vec<ActionFee>,
) -> Result<Response, ContractError> {
    let storage = &mut execute_env.deps.storage;
    ensure!(
        ADOContract::default().is_owner_or_operator(*storage, execute_env.info.sender.as_str())?,
        ContractError::Unauthorized {}
    );
    let ado_version = ADOVersion::from_string(ado_version);
    let mut attributes: Vec<Attribute> = vec![];
    // Check if ado exists, otherwise throw error
    query::ado_exists(*storage, &ado_version)?;

    for action_fee in fees {
        ACTION_FEES.save(
            *storage,
            &(ado_version.clone().into_string(), action_fee.clone().action),
            &action_fee,
        )?;
        attributes.push(Attributes::update_action_fee(&action_fee.action));
    }
    Ok(Response::default()
        .add_attributes(Attributes::update_action_fees(ado_version.as_str()))
        .add_attributes(attributes))
}

pub fn remove_action_fees(
    execute_env: &mut ExecuteContext,
    ado_version: &str,
    actions: Vec<String>,
) -> Result<Response, ContractError> {
    let storage = &mut execute_env.deps.storage;
    ensure!(
        ADOContract::default().is_owner_or_operator(*storage, execute_env.info.sender.as_str())?,
        ContractError::Unauthorized {}
    );
    let ado_version = ADOVersion::from_string(ado_version);
    // Check if ado exists, otherwise throw error
    query::ado_exists(*storage, &ado_version)?;

    let mut attributes: Vec<Attribute> = vec![];

    for action in actions {
        ACTION_FEES.remove(
            *storage,
            &(ado_version.clone().into_string(), action.clone()),
        );
        attributes.push(Attributes::remove_action_fee(action.as_str()));
    }
    Ok(Response::default()
        .add_attributes(Attributes::remove_action_fees(ado_version.as_str()))
        .add_attributes(attributes))
}

fn store_code_id(
    execute_env: &mut ExecuteContext,
    ado_version: &ADOVersion,
    code_id: u64,
) -> Result<Response, ContractError> {
    // If ado version is not strict, we cannot store it for a version
    let mut attributes: Vec<Attribute> = vec![];
    ensure!(
        ado_version.validate_strict(),
        ContractError::InvalidADOVersion {
            msg: Some("Ado version provided cannot be used to store code".into())
        }
    );
    ensure!(
        Version::parse(&ado_version.get_version()).is_ok(),
        ContractError::InvalidADOVersion {
            msg: Some("Invalid version".into())
        }
    );
    let storage = &mut execute_env.deps.storage;
    ensure!(
        !ADO_TYPE.has(*storage, code_id),
        ContractError::InvalidADOVersion {
            msg: Some("CodeId already exists".into())
        }
    );

    // Store the map for code_id -> `adotype@version`
    ADO_TYPE
        .save(*storage, code_id, &ado_version.clone().into_string())
        .unwrap();

    // Store the map for `adotype@version` -> code_id
    CODE_ID
        .save(*storage, ado_version.as_str(), &code_id)
        .unwrap();

    attributes.append(&mut Attributes::store_code_id(ado_version, code_id));

    // Update Metadata
    METADATA
        .update::<_, StdError>(*storage, ado_version.as_str(), |prev| match prev {
            Some(mut meta) => {
                meta.updated_on = execute_env.env.block.time;
                meta.last_updated_by = execute_env.info.sender.clone();
                Ok(meta)
            }
            None => Ok(ADOMetadata {
                last_updated_by: execute_env.info.sender.clone(),
                published_on: execute_env.env.block.time,
                updated_on: execute_env.env.block.time,
            }),
        })
        .unwrap();
    attributes.append(&mut Attributes::update_metadata(ado_version));

    // Store Latest Version
    let current_ado_version = LATEST_VERSION.may_load(*storage, &ado_version.get_type())?;
    let mut latest_version = ado_version.get_version();
    if let Some(curr_version) = current_ado_version {
        let new_version = semver::Version::parse(&ado_version.get_version()).unwrap();
        let curr_version = semver::Version::parse(&curr_version).unwrap();
        if new_version < curr_version {
            latest_version = curr_version.to_string();
        }
    }
    LATEST_VERSION.save(*storage, &ado_version.get_type(), &latest_version)?;
    attributes.append(&mut Attributes::update_latest_version(ado_version));

    Ok(Response::default().add_attributes(attributes))
}
