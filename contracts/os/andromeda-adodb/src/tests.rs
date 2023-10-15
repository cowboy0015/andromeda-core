use std::ops::Add;

use andromeda_std::error::ContractError;
use andromeda_std::testing::mock_querier::MOCK_KERNEL_CONTRACT;
#[cfg(test)]
use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage};
use cosmwasm_std::{from_binary, Addr, Attribute, Empty, Env, MessageInfo, OwnedDeps, Uint128};

use crate::attributes::Attributes;
use crate::contract::{execute, instantiate, query};
use crate::state::ACTION_FEES;

use andromeda_std::os::adodb::{
    ADOMetadata, ADOVersion, ActionFee, ExecuteMsg, InstantiateMsg, QueryMsg,
};

use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

type DepsAlias = OwnedDeps<MockStorage, MockApi, MockQuerier, Empty>;

pub const CREATOR: &str = "creator";

pub fn mock_test_env() -> (DepsAlias, MessageInfo, Env) {
    let mut deps: DepsAlias = mock_dependencies();
    let info = mock_info(CREATOR, &[]);
    let env = mock_env();
    let msg = InstantiateMsg {
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
        owner: None,
    };
    instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
    (deps, info, env)
}

#[test]
fn proper_initialization() {
    mock_test_env();
}

#[test]
fn test_publish() {
    let (mut deps, info, env) = mock_test_env();

    let ado_type = "address_list";
    let ado_version = "0.1.0";
    let code_id = 1u64;
    let action_fees = None;

    let msg = ExecuteMsg::Publish {
        publisher: None,
        code_id,
        ado_type: ado_type.into(),
        action_fees,
        version: ado_version.into(),
    };
    let resp = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    let ado_version = ADOVersion::from_string(ado_type).with_version(ado_version);

    let mut expected: Vec<Attribute> = vec![];
    expected.append(Attributes::publish().as_mut());
    expected.append(Attributes::store_code_id(&ado_version, code_id).as_mut());
    expected.append(Attributes::update_metadata(&ado_version).as_mut());
    expected.append(Attributes::update_latest_version(&ado_version).as_mut());
    expected.append(Attributes::update_publisher(info.sender.as_str()).as_mut());

    assert_eq!(resp.attributes, expected);

    // TEST CodeId
    let msg = QueryMsg::CodeId {
        key: ado_version.clone().into_string(),
    };
    let res: u64 = from_binary(&query(deps.as_ref(), env.clone(), msg).unwrap()).unwrap();
    assert_eq!(res, code_id);

    // TEST Ado Version
    let msg = QueryMsg::ADOType { code_id };
    let res: String = from_binary(&query(deps.as_ref(), env, msg).unwrap()).unwrap();
    assert_eq!(res, ado_version.into_string());
}

#[test]
fn test_publish_unauthorized() {
    let (mut deps, mut info, env) = mock_test_env();

    let ado_type = "address_list";
    let ado_version = "0.1.0";
    let code_id = 1u64;
    let action_fees = None;

    let msg = ExecuteMsg::Publish {
        publisher: None,
        code_id,
        ado_type: ado_type.into(),
        action_fees,
        version: ado_version.into(),
    };
    info.sender = Addr::unchecked("different");
    execute(deps.as_mut(), env, info, msg).unwrap_err();
}

#[test]
fn test_publish_duplicate() {
    let (mut deps, info, env) = mock_test_env();

    let ado_type = "address_list";
    let ado_version = "0.1.0";
    let code_id = 1u64;
    let action_fees = None;

    let msg = ExecuteMsg::Publish {
        publisher: None,
        code_id,
        ado_type: ado_type.into(),
        action_fees: action_fees.clone(),
        version: ado_version.into(),
    };
    execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
    // Try publishing same ado again with same version
    execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();

    let msg = ExecuteMsg::Publish {
        publisher: None,
        code_id: code_id.add(1),
        ado_type: ado_type.into(),
        action_fees,
        version: ado_version.into(),
    };
    // Try publishing same ado again with differnt code_id
    execute(deps.as_mut(), env, info, msg).unwrap_err();
}

#[test]
fn test_publish_invalid_version() {
    let (mut deps, info, env) = mock_test_env();

    let ado_type = "address_list";
    let ado_version = "0.1.0.0";
    let code_id = 1u64;
    let action_fees = None;

    let msg = ExecuteMsg::Publish {
        publisher: None,
        code_id,
        ado_type: ado_type.into(),
        action_fees,
        version: ado_version.into(),
    };
    // It should fail
    execute(deps.as_mut(), env, info, msg).unwrap_err();
}

#[test]
fn test_latest_version() {
    let (mut deps, info, env) = mock_test_env();

    let ado_type = "address_list";
    let action_fees = None;
    let samples: Vec<(u64, &str)> = vec![(1, "0.1.1"), (2, "0.1.2"), (3, "0.1.0")];
    samples.iter().for_each(|(code_id, version)| {
        let msg = ExecuteMsg::Publish {
            publisher: None,
            code_id: *code_id,
            ado_type: ado_type.into(),
            action_fees: action_fees.clone(),
            version: version.to_string(),
        };

        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        let msg = QueryMsg::CodeId {
            key: format!("{ado_type}@{version}"),
        };
        let res: u64 = from_binary(&query(deps.as_ref(), env.clone(), msg).unwrap()).unwrap();
        // New codeid should be latest now
        assert_eq!(res, *code_id);
    });

    // Test for latest
    let msg = QueryMsg::CodeId {
        key: format!("{ado_type}@latest"),
    };
    let res: u64 = from_binary(&query(deps.as_ref(), env.clone(), msg).unwrap()).unwrap();
    assert_eq!(res, 2);

    // Test for default
    let msg = QueryMsg::CodeId {
        key: ado_type.to_string(),
    };
    let res: u64 = from_binary(&query(deps.as_ref(), env, msg).unwrap()).unwrap();
    assert_eq!(res, 2);
}

#[test]
fn test_update() {
    let (mut deps, info, env) = mock_test_env();

    let ado_version = ADOVersion::from_string("address_list@0.1.0");
    let code_id = 1;

    let publish_msg = ExecuteMsg::Publish {
        publisher: None,
        code_id,
        ado_type: ado_version.get_type(),
        action_fees: None,
        version: ado_version.get_version(),
    };

    execute(deps.as_mut(), env.clone(), info.clone(), publish_msg).unwrap();

    let publish_msg = ExecuteMsg::Publish {
        publisher: None,
        code_id: 101,
        ado_type: "new-ado".to_string(),
        action_fees: None,
        version: "0.1.1".to_string(),
    };

    execute(deps.as_mut(), env.clone(), info.clone(), publish_msg).unwrap();

    let code_id = 2;
    let update_msg = ExecuteMsg::UpdateCodeId {
        code_id_key: ado_version.clone().into_string(),
        code_id,
    };
    execute(deps.as_mut(), env.clone(), info.clone(), update_msg).unwrap();

    // Test updates for code id state
    test_codes(&deps, &env, &ado_version, code_id);

    // Lets revert the codeId back to 1, it should work
    let update_msg = ExecuteMsg::UpdateCodeId {
        code_id_key: ado_version.clone().into_string(),
        code_id: 1,
    };
    execute(deps.as_mut(), env.clone(), info.clone(), update_msg).unwrap();
    // Test updates for code id state
    test_codes(&deps, &env, &ado_version, 1);

    // Lets try to update ado that doesn't exist
    let update_msg = ExecuteMsg::UpdateCodeId {
        code_id_key: "do_not_exist@0.1.1".to_string(),
        code_id: 5,
    };
    // It will Error
    execute(deps.as_mut(), env.clone(), info.clone(), update_msg).unwrap_err();

    // Lets try to update ado with codeId that already exist
    let update_msg = ExecuteMsg::UpdateCodeId {
        code_id_key: ado_version.into_string(),
        code_id: 101,
    };
    // It will Error
    execute(deps.as_mut(), env, info, update_msg).unwrap_err();
}

#[test]
fn test_update_action_fees() {
    let (mut deps, info, env) = mock_test_env();

    let ado_version = ADOVersion::from_string("address_list@0.1.0");
    let code_id = 1u64;
    let action_fees = vec![
        ActionFee {
            action: "action".to_string(),
            amount: Uint128::from(1u128),
            asset: "somecw20token".to_string(),
            receiver: None,
        },
        ActionFee {
            action: "action2".to_string(),
            amount: Uint128::from(2u128),
            asset: "uusd".to_string(),
            receiver: None,
        },
    ];

    let update_msg = ExecuteMsg::UpdateActionFees {
        action_fees: action_fees.clone(),
        ado_type: ado_version.clone().into_string(),
    };

    // Its should error as ado type doesn't exist yet
    execute(deps.as_mut(), env.clone(), info.clone(), update_msg.clone()).unwrap_err();

    let publish_msg = ExecuteMsg::Publish {
        publisher: None,
        code_id,
        ado_type: ado_version.get_type(),
        action_fees: None,
        version: ado_version.get_version(),
    };

    execute(deps.as_mut(), env.clone(), info.clone(), publish_msg).unwrap();

    // Now it will success
    execute(deps.as_mut(), env.clone(), info, update_msg.clone()).unwrap();

    // TEST ACTION FEE
    for action_fee in action_fees {
        let fee = ACTION_FEES
            .load(
                deps.as_ref().storage,
                &(ado_version.clone().into_string(), action_fee.clone().action),
            )
            .unwrap();
        assert_eq!(fee, action_fee);
    }

    // Test unauthorised
    let unauth_info = mock_info("not_owner", &[]);
    let resp = execute(deps.as_mut(), env, unauth_info, update_msg);
    assert!(resp.is_err());
}

#[test]
fn test_remove_action_fees() {
    let (mut deps, info, env) = mock_test_env();

    let ado_version = ADOVersion::from_string("address_list@0.1.0");
    let code_id = 1u64;
    let action_fees = vec![
        ActionFee {
            action: "action1".to_string(),
            amount: Uint128::from(1u128),
            asset: "somecw20token".to_string(),
            receiver: None,
        },
        ActionFee {
            action: "action2".to_string(),
            amount: Uint128::from(2u128),
            asset: "uusd".to_string(),
            receiver: None,
        },
    ];

    let publish_msg = ExecuteMsg::Publish {
        publisher: None,
        code_id,
        ado_type: ado_version.get_type(),
        action_fees: Some(action_fees.clone()),
        version: ado_version.get_version(),
    };

    execute(deps.as_mut(), env.clone(), info.clone(), publish_msg).unwrap();

    let (removed_actions, action_fees) = action_fees.split_at(1);

    let msg = ExecuteMsg::RemoveActionFees {
        ado_type: ado_version.clone().into_string(),
        actions: removed_actions
            .iter()
            .map(|a| a.action.to_string())
            .collect(),
    };

    let unauth_info = mock_info("not_owner", &[]);
    let res = execute(deps.as_mut(), env.clone(), unauth_info, msg.clone()).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {});

    let res = execute(deps.as_mut(), env, info, msg);
    assert!(res.is_ok());

    // TEST REMOVED ACTION FEE
    for action_fee in removed_actions {
        let fee = ACTION_FEES
            .may_load(
                deps.as_ref().storage,
                &(ado_version.clone().into_string(), action_fee.clone().action),
            )
            .unwrap();
        assert!(fee.is_none());
    }

    // TEST OTHER ACTION FEE
    for action_fee in action_fees {
        let fee = ACTION_FEES
            .load(
                deps.as_ref().storage,
                &(ado_version.clone().into_string(), action_fee.clone().action),
            )
            .unwrap();
        assert_eq!(&fee, action_fee);
    }
}

#[test]
fn test_metadata() {
    let (mut deps, info, mut env) = mock_test_env();

    let ado_version = ADOVersion::from_string("address_list@0.1.0");
    let code_id = 1u64;

    let publish_msg = ExecuteMsg::Publish {
        publisher: None,
        code_id,
        ado_type: ado_version.get_type(),
        action_fees: None,
        version: ado_version.get_version(),
    };

    execute(deps.as_mut(), env.clone(), info.clone(), publish_msg).unwrap();

    let msg = QueryMsg::ADOMetadata {
        ado_version: ado_version.clone().into_string(),
    };

    let res: ADOMetadata =
        from_binary(&query(deps.as_ref(), env.clone(), msg.clone()).unwrap()).unwrap();

    assert_eq!(
        res,
        ADOMetadata {
            published_on: env.block.time,
            updated_on: env.block.time,
            last_updated_by: info.sender.clone()
        }
    );
    let published_time = env.block.time;
    // set new env time
    env.block.time = env.block.time.plus_minutes(10);

    let update_msg = ExecuteMsg::UpdateCodeId {
        code_id_key: ado_version.into_string(),
        code_id: code_id.add(1),
    };
    execute(deps.as_mut(), env.clone(), info.clone(), update_msg).unwrap();

    let res: ADOMetadata = from_binary(&query(deps.as_ref(), env.clone(), msg).unwrap()).unwrap();

    assert_eq!(
        res,
        ADOMetadata {
            published_on: published_time,
            updated_on: env.block.time,
            last_updated_by: info.sender
        }
    );
}

#[test]
fn test_publisher() {
    let (mut deps, info, env) = mock_test_env();

    let ado_version = ADOVersion::from_string("address_list@0.1.0");
    let code_id = 1u64;
    let publisher = Addr::unchecked("publisher");
    let publish_msg = ExecuteMsg::Publish {
        publisher: Some(publisher.clone()),
        code_id,
        ado_type: ado_version.get_type(),
        action_fees: None,
        version: ado_version.get_version(),
    };

    execute(deps.as_mut(), env.clone(), info.clone(), publish_msg).unwrap();

    let msg = QueryMsg::ADOPublisher {
        ado_version: ado_version.clone().into_string(),
    };

    let res: Addr = from_binary(&query(deps.as_ref(), env.clone(), msg.clone()).unwrap()).unwrap();

    assert_eq!(res, publisher);

    let publisher = Addr::unchecked("new_publisher");
    let update_msg = ExecuteMsg::UpdatePublisher {
        ado_version: ado_version.into_string(),
        publisher: publisher.clone(),
    };
    execute(deps.as_mut(), env.clone(), info.clone(), update_msg).unwrap();

    // Check that new publisher is updated correctly
    let res: Addr = from_binary(&query(deps.as_ref(), env.clone(), msg).unwrap()).unwrap();
    assert_eq!(res, publisher);

    // Try to update publisher for an ado that doen't exist
    let update_msg = ExecuteMsg::UpdatePublisher {
        ado_version: "ado_do_not_exist@0.1.1".to_string(),
        publisher,
    };
    execute(deps.as_mut(), env, info, update_msg).unwrap_err();
}

fn test_codes(deps: &DepsAlias, env: &Env, ado_version: &ADOVersion, code_id: u64) {
    // TEST CodeId
    let msg = QueryMsg::CodeId {
        key: ado_version.clone().into_string(),
    };
    let res: u64 = from_binary(&query(deps.as_ref(), env.clone(), msg).unwrap()).unwrap();
    assert_eq!(res, code_id);

    // TEST Ado Version
    let msg = QueryMsg::ADOType { code_id };
    let res: String = from_binary(&query(deps.as_ref(), env.clone(), msg).unwrap()).unwrap();
    assert_eq!(res, ado_version.clone().into_string());

    // Test for latest
    let msg = QueryMsg::CodeId {
        key: format!("{ado_type}@latest", ado_type = ado_version.get_type()),
    };
    let res: u64 = from_binary(&query(deps.as_ref(), env.clone(), msg).unwrap()).unwrap();
    assert_eq!(res, code_id);
}
