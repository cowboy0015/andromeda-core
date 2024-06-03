use andromeda_non_fungible_tokens::{
    crowdfund::{
        CampaignConfig, CampaignStage, ExecuteMsg, InstantiateMsg, SimpleTierOrder, Tier,
        TierMetaData,
    },
    cw721::{ExecuteMsg as Cw721ExecuteMsg, TokenExtension},
};

use andromeda_std::{
    common::{reply::ReplyId, MillisecondsExpiration},
    error::ContractError,
    os::economics::ExecuteMsg as EconomicsExecuteMsg,
    testing::mock_querier::{MOCK_ADO_PUBLISHER, MOCK_KERNEL_CONTRACT},
};
use cosmwasm_std::{
    testing::{mock_env, mock_info},
    to_json_binary, Addr, CosmosMsg, DepsMut, Env, Order, Response, Storage, SubMsg, Uint128,
    Uint64, WasmMsg,
};

use crate::{
    contract::{execute, instantiate},
    state::{CAMPAIGN_CONFIG, CAMPAIGN_STAGE, CURRENT_CAP, TIERS, TIER_ORDERS},
    testing::mock_querier::{mock_dependencies_custom, mock_zero_price_tier, MOCK_DEFAULT_LIMIT},
};

use super::mock_querier::{mock_campaign_config, mock_campaign_tiers, MOCK_DEFAULT_OWNER};

fn init(deps: DepsMut, config: CampaignConfig, tiers: Vec<Tier>) -> Response {
    let msg = InstantiateMsg {
        campaign_config: config,
        tiers,
        owner: None,
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
    };

    let info = mock_info(MOCK_DEFAULT_OWNER, &[]);
    instantiate(deps, mock_env(), info, msg).unwrap()
}

fn get_campaign_config(storage: &dyn Storage) -> CampaignConfig {
    CAMPAIGN_CONFIG.load(storage).unwrap()
}

fn get_tiers(storage: &dyn Storage) -> Vec<Tier> {
    TIERS
        .range_raw(storage, None, None, Order::Ascending)
        .map(|res| res.unwrap().1)
        .collect()
}

fn future_time(env: &Env) -> MillisecondsExpiration {
    MillisecondsExpiration::from_seconds(env.block.time.seconds() + 5000)
}
fn past_time() -> MillisecondsExpiration {
    MillisecondsExpiration::from_seconds(0) // Past timestamp
}
fn set_campaign_stage(store: &mut dyn Storage, stage: &CampaignStage) {
    CAMPAIGN_STAGE.save(store, stage).unwrap();
}
fn set_current_cap(store: &mut dyn Storage, cur_cap: &Uint128) {
    CURRENT_CAP.save(store, cur_cap).unwrap();
}
fn set_campaign_config(store: &mut dyn Storage, config: &CampaignConfig) {
    CAMPAIGN_CONFIG.save(store, config).unwrap();
}
fn set_tiers(storage: &mut dyn Storage, tiers: Vec<Tier>) {
    for tier in tiers {
        TIERS.save(storage, tier.level.into(), &tier).unwrap();
    }
}

fn get_user_orders(
    storage: &dyn Storage,
    user: Addr,
) -> Result<Vec<SimpleTierOrder>, ContractError> {
    TIER_ORDERS
        .prefix(user)
        .range(storage, None, None, Order::Ascending)
        .map(|res| {
            Ok(res.map(|(level, order_info)| SimpleTierOrder {
                level: Uint64::new(level),
                amount: Uint128::new(order_info.amount().unwrap()),
            })?)
        })
        .collect()
}
#[cfg(test)]
mod test {
    use andromeda_non_fungible_tokens::crowdfund::{
        Cw20HookMsg, PresaleTierOrder, SimpleTierOrder, TierOrder,
    };
    use andromeda_std::{
        amp::{messages::AMPPkt, AndrAddr, Recipient},
        common::{denom::Asset, encode_binary},
        testing::mock_querier::MOCK_CW20_CONTRACT,
    };
    use cosmwasm_std::{coin, coins, testing::MOCK_CONTRACT_ADDR, wasm_execute, BankMsg, Coin};
    use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

    use crate::{
        state::{get_current_cap, set_current_stage, set_tier_orders},
        testing::mock_querier::{MOCK_DEFAULT_OWNER, MOCK_WITHDRAWAL_ADDRESS},
    };

    use super::*;

    const MOCK_NATIVE_DENOM: &str = "uandr";
    const INVA1LID_DENOM: &str = "other";

    struct InstantiateTestCase {
        name: String,
        config: CampaignConfig,
        tiers: Vec<Tier>,
        expected_res: Result<Response, ContractError>,
    }
    #[test]
    fn test_instantiate() {
        let test_cases: Vec<InstantiateTestCase> = vec![
            InstantiateTestCase {
                name: "instantiate with native token".to_string(),
                config: mock_campaign_config(Asset::NativeToken(MOCK_NATIVE_DENOM.to_string())),
                tiers: mock_campaign_tiers(),
                expected_res: Ok(Response::new()
                    .add_attribute("method", "instantiate")
                    .add_attribute("type", "crowdfund")
                    .add_attribute("kernel_address", MOCK_KERNEL_CONTRACT)
                    .add_attribute(MOCK_DEFAULT_OWNER, MOCK_DEFAULT_OWNER)),
            },
            InstantiateTestCase {
                name: "instantiate with invalid native token".to_string(),
                config: mock_campaign_config(Asset::NativeToken(INVA1LID_DENOM.to_string())),
                tiers: mock_campaign_tiers(),
                expected_res: Err(ContractError::InvalidAsset {
                    asset: Asset::NativeToken(INVA1LID_DENOM.to_string()).to_string(),
                }),
            },
            InstantiateTestCase {
                name: "instantiate with cw20".to_string(),
                config: mock_campaign_config(Asset::Cw20Token(AndrAddr::from_string(
                    MOCK_CW20_CONTRACT.to_string(),
                ))),
                tiers: mock_campaign_tiers(),
                expected_res: Ok(Response::new()
                    .add_attribute("method", "instantiate")
                    .add_attribute("type", "crowdfund")
                    .add_attribute("kernel_address", MOCK_KERNEL_CONTRACT)
                    .add_attribute(MOCK_DEFAULT_OWNER, MOCK_DEFAULT_OWNER)),
            },
            InstantiateTestCase {
                name: "instantiate with invalid cw20".to_string(),
                config: mock_campaign_config(Asset::Cw20Token(AndrAddr::from_string(
                    "cw20_contract1".to_string(),
                ))),
                tiers: mock_campaign_tiers(),
                expected_res: Err(ContractError::InvalidAsset {
                    asset: Asset::Cw20Token(AndrAddr::from_string("cw20_contract1".to_string()))
                        .to_string(),
                }),
            },
            InstantiateTestCase {
                name: "instantiate with invalid tiers including zero price tier".to_string(),
                config: mock_campaign_config(Asset::NativeToken(MOCK_NATIVE_DENOM.to_string())),
                tiers: vec![mock_zero_price_tier(Uint64::zero())],
                expected_res: Err(ContractError::InvalidTier {
                    operation: "all".to_string(),
                    msg: "Price can not be zero".to_string(),
                }),
            },
        ];

        for test in test_cases {
            let mut deps = mock_dependencies_custom(&[coin(100000, MOCK_NATIVE_DENOM)]);
            let info = mock_info(MOCK_DEFAULT_OWNER, &[]);
            let msg = InstantiateMsg {
                campaign_config: test.config.clone(),
                tiers: test.tiers.clone(),
                owner: None,
                kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
            };
            let res = instantiate(deps.as_mut(), mock_env(), info, msg);

            assert_eq!(res, test.expected_res, "Test case: {}", test.name);
            if res.is_ok() {
                assert_eq!(
                    get_campaign_config(&deps.storage),
                    test.config,
                    "Test case: {}",
                    test.name
                );
                assert_eq!(
                    get_tiers(deps.as_ref().storage),
                    test.tiers,
                    "Test case: {}",
                    test.name
                );
            }
        }
    }

    struct TierTestCase {
        name: String,
        tier: Tier,
        expected_res: Result<Response, ContractError>,
        payee: String,
    }

    #[test]
    fn test_add_tier() {
        let valid_tier = Tier {
            level: Uint64::new(2u64),
            label: "Tier 2".to_string(),
            limit: Some(Uint128::new(100)),
            sold_amount: Uint128::zero(),
            price: Uint128::new(100),
            metadata: TierMetaData {
                extension: TokenExtension {
                    publisher: MOCK_ADO_PUBLISHER.to_string(),
                },
                token_uri: None,
            },
        };
        let duplicated_tier = Tier {
            level: Uint64::new(0u64),
            label: "Tier 2".to_string(),
            limit: Some(Uint128::new(100)),
            sold_amount: Uint128::zero(),
            price: Uint128::new(100),
            metadata: TierMetaData {
                extension: TokenExtension {
                    publisher: MOCK_ADO_PUBLISHER.to_string(),
                },
                token_uri: None,
            },
        };

        let test_cases: Vec<TierTestCase> = vec![
            TierTestCase {
                name: "standard add_tier".to_string(),
                tier: valid_tier.clone(),
                payee: MOCK_DEFAULT_OWNER.to_string(),
                expected_res: Ok(Response::new()
                    .add_attribute("action", "add_tier")
                    .add_attribute("level", valid_tier.level.to_string())
                    .add_attribute("label", valid_tier.label.clone())
                    .add_attribute("price", valid_tier.price.to_string())
                    .add_attribute("limit", valid_tier.limit.unwrap().to_string())
                    // Economics message
                    .add_submessage(SubMsg::reply_on_error(
                        CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: "economics_contract".to_string(),
                            msg: to_json_binary(&EconomicsExecuteMsg::PayFee {
                                payee: Addr::unchecked(MOCK_DEFAULT_OWNER),
                                action: "AddTier".to_string(),
                            })
                            .unwrap(),
                            funds: vec![],
                        }),
                        ReplyId::PayFee.repr(),
                    ))),
            },
            TierTestCase {
                name: "add_tier with unauthorized sender".to_string(),
                tier: valid_tier.clone(),
                expected_res: Err(ContractError::Unauthorized {}),
                payee: "owner1".to_string(),
            },
            TierTestCase {
                name: "add_tier with zero price tier".to_string(),
                tier: mock_zero_price_tier(Uint64::new(2)),
                expected_res: Err(ContractError::InvalidTier {
                    operation: "all".to_string(),
                    msg: "Price can not be zero".to_string(),
                }),
                payee: MOCK_DEFAULT_OWNER.to_string(),
            },
            TierTestCase {
                name: "add_tier with duplicated tier".to_string(),
                tier: duplicated_tier,
                expected_res: Err(ContractError::InvalidTier {
                    operation: "add".to_string(),
                    msg: "Tier with level 0 already exist".to_string(),
                }),
                payee: MOCK_DEFAULT_OWNER.to_string(),
            },
        ];
        for test in test_cases {
            let mut deps = mock_dependencies_custom(&[coin(100000, MOCK_NATIVE_DENOM)]);
            let _ = init(
                deps.as_mut(),
                mock_campaign_config(Asset::NativeToken(MOCK_NATIVE_DENOM.to_string())),
                mock_campaign_tiers(),
            );

            let info = mock_info(&test.payee, &[]);

            let msg = ExecuteMsg::AddTier {
                tier: test.tier.clone(),
            };

            let res = execute(deps.as_mut(), mock_env(), info, msg);
            assert_eq!(res, test.expected_res, "Test case: {}", test.name);
            if res.is_ok() {
                assert_eq!(
                    test.tier,
                    TIERS
                        .load(deps.as_ref().storage, test.tier.level.into())
                        .unwrap(),
                    "Test case: {}",
                    test.name
                );
            }
        }
    }

    #[test]
    fn test_update_tier() {
        let valid_tier = Tier {
            level: Uint64::zero(),
            label: "Tier 0".to_string(),
            limit: Some(Uint128::new(100)),
            sold_amount: Uint128::zero(),
            price: Uint128::new(100),
            metadata: TierMetaData {
                extension: TokenExtension {
                    publisher: MOCK_ADO_PUBLISHER.to_string(),
                },
                token_uri: None,
            },
        };
        let non_existing_tier = Tier {
            level: Uint64::new(2u64),
            label: "Tier 2".to_string(),
            limit: Some(Uint128::new(100)),
            sold_amount: Uint128::zero(),
            price: Uint128::new(100),
            metadata: TierMetaData {
                extension: TokenExtension {
                    publisher: MOCK_ADO_PUBLISHER.to_string(),
                },
                token_uri: None,
            },
        };

        let test_cases: Vec<TierTestCase> = vec![
            TierTestCase {
                name: "standard update_tier".to_string(),
                tier: valid_tier.clone(),
                payee: MOCK_DEFAULT_OWNER.to_string(),
                expected_res: Ok(Response::new()
                    .add_attribute("action", "update_tier")
                    .add_attribute("level", valid_tier.level.to_string())
                    .add_attribute("label", valid_tier.label.clone())
                    .add_attribute("price", valid_tier.price.to_string())
                    .add_attribute("limit", valid_tier.limit.unwrap().to_string())
                    // Economics message
                    .add_submessage(SubMsg::reply_on_error(
                        CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: "economics_contract".to_string(),
                            msg: to_json_binary(&EconomicsExecuteMsg::PayFee {
                                payee: Addr::unchecked(MOCK_DEFAULT_OWNER),
                                action: "UpdateTier".to_string(),
                            })
                            .unwrap(),
                            funds: vec![],
                        }),
                        ReplyId::PayFee.repr(),
                    ))),
            },
            TierTestCase {
                name: "update_tier with unauthorized sender".to_string(),
                tier: valid_tier.clone(),
                expected_res: Err(ContractError::Unauthorized {}),
                payee: "owner1".to_string(),
            },
            TierTestCase {
                name: "update_tier with zero price tier".to_string(),
                tier: mock_zero_price_tier(Uint64::zero()),
                expected_res: Err(ContractError::InvalidTier {
                    operation: "all".to_string(),
                    msg: "Price can not be zero".to_string(),
                }),
                payee: MOCK_DEFAULT_OWNER.to_string(),
            },
            TierTestCase {
                name: "update_tier with non existing tier".to_string(),
                tier: non_existing_tier,
                expected_res: Err(ContractError::InvalidTier {
                    operation: "update".to_string(),
                    msg: "Tier with level 2 does not exist".to_string(),
                }),
                payee: MOCK_DEFAULT_OWNER.to_string(),
            },
        ];
        for test in test_cases {
            let mut deps = mock_dependencies_custom(&[coin(100000, MOCK_NATIVE_DENOM)]);
            let _ = init(
                deps.as_mut(),
                mock_campaign_config(Asset::NativeToken(MOCK_NATIVE_DENOM.to_string())),
                mock_campaign_tiers(),
            );

            let info = mock_info(&test.payee, &[]);

            let msg = ExecuteMsg::UpdateTier {
                tier: test.tier.clone(),
            };

            let res = execute(deps.as_mut(), mock_env(), info, msg);
            assert_eq!(res, test.expected_res, "Test case: {}", test.name);
            if res.is_ok() {
                assert_eq!(
                    test.tier,
                    TIERS
                        .load(deps.as_ref().storage, test.tier.level.into())
                        .unwrap(),
                    "Test case: {}",
                    test.name
                );
            }
        }
    }

    #[test]
    fn test_remove_tier() {
        let valid_tier = Tier {
            level: Uint64::zero(),
            label: "Tier 0".to_string(),
            limit: Some(Uint128::new(100)),
            price: Uint128::new(100),
            sold_amount: Uint128::zero(),
            metadata: TierMetaData {
                extension: TokenExtension {
                    publisher: MOCK_ADO_PUBLISHER.to_string(),
                },
                token_uri: None,
            },
        };
        let non_existing_tier = Tier {
            level: Uint64::new(2u64),
            label: "Tier 2".to_string(),
            limit: Some(Uint128::new(100)),
            price: Uint128::new(100),
            sold_amount: Uint128::zero(),
            metadata: TierMetaData {
                extension: TokenExtension {
                    publisher: MOCK_ADO_PUBLISHER.to_string(),
                },
                token_uri: None,
            },
        };

        let test_cases: Vec<TierTestCase> = vec![
            TierTestCase {
                name: "standard remove_tier".to_string(),
                tier: valid_tier.clone(),
                payee: MOCK_DEFAULT_OWNER.to_string(),
                expected_res: Ok(Response::new()
                    .add_attribute("action", "remove_tier")
                    .add_attribute("level", valid_tier.level.to_string())
                    .add_submessage(SubMsg::reply_on_error(
                        CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: "economics_contract".to_string(),
                            msg: to_json_binary(&EconomicsExecuteMsg::PayFee {
                                payee: Addr::unchecked(MOCK_DEFAULT_OWNER),
                                action: "RemoveTier".to_string(),
                            })
                            .unwrap(),
                            funds: vec![],
                        }),
                        ReplyId::PayFee.repr(),
                    ))),
            },
            TierTestCase {
                name: "remove_tier with unauthorized sender".to_string(),
                tier: valid_tier.clone(),
                expected_res: Err(ContractError::Unauthorized {}),
                payee: "owner1".to_string(),
            },
            TierTestCase {
                name: "remove_tier with non existing tier level".to_string(),
                tier: non_existing_tier,
                expected_res: Err(ContractError::InvalidTier {
                    operation: "remove".to_string(),
                    msg: "Tier with level 2 does not exist".to_string(),
                }),
                payee: MOCK_DEFAULT_OWNER.to_string(),
            },
        ];
        for test in test_cases {
            let mut deps = mock_dependencies_custom(&[coin(100000, MOCK_NATIVE_DENOM)]);
            let _ = init(
                deps.as_mut(),
                mock_campaign_config(Asset::NativeToken(MOCK_NATIVE_DENOM.to_string())),
                mock_campaign_tiers(),
            );

            let info = mock_info(&test.payee, &[]);

            let msg = ExecuteMsg::RemoveTier {
                level: test.tier.level,
            };

            let res = execute(deps.as_mut(), mock_env(), info, msg);
            assert_eq!(res, test.expected_res, "Test case: {}", test.name);
            if res.is_ok() {
                assert!(
                    !TIERS.has(deps.as_ref().storage, test.tier.level.into()),
                    "Test case: {}",
                    test.name
                );
            }
        }
    }

    struct StartCampaignTestCase {
        name: String,
        tiers: Vec<Tier>,
        presale: Option<Vec<PresaleTierOrder>>,
        start_time: Option<MillisecondsExpiration>,
        end_time: MillisecondsExpiration,
        expected_res: Result<Response, ContractError>,
        payee: String,
    }

    #[test]
    fn test_start_campaign() {
        let mock_orderer = Addr::unchecked("mock_orderer".to_string());
        let valid_presale = vec![PresaleTierOrder {
            amount: Uint128::new(100u128),
            level: Uint64::new(1u64),
            orderer: mock_orderer.clone(),
        }];

        let invalid_presale = vec![PresaleTierOrder {
            amount: Uint128::new(100u128),
            level: Uint64::new(2u64),
            orderer: mock_orderer.clone(),
        }];

        let invalid_tiers = vec![Tier {
            level: Uint64::new(1u64),
            label: "Tier 1".to_string(),
            limit: Some(Uint128::new(1000u128)),
            sold_amount: Uint128::zero(),
            price: Uint128::new(10u128),
            metadata: TierMetaData {
                extension: TokenExtension {
                    publisher: MOCK_ADO_PUBLISHER.to_string(),
                },
                token_uri: None,
            },
        }];

        let env = mock_env();
        let test_cases: Vec<StartCampaignTestCase> = vec![
            StartCampaignTestCase {
                name: "standard start_campaign".to_string(),
                tiers: mock_campaign_tiers(),
                presale: Some(valid_presale.clone()),
                start_time: None,
                end_time: MillisecondsExpiration::from_seconds(env.block.time.seconds() + 100),
                payee: MOCK_DEFAULT_OWNER.to_string(),
                expected_res: Ok(Response::new()
                    .add_attribute("action", "start_campaign")
                    .add_submessage(SubMsg::reply_on_error(
                        CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: "economics_contract".to_string(),
                            msg: to_json_binary(&EconomicsExecuteMsg::PayFee {
                                payee: Addr::unchecked(MOCK_DEFAULT_OWNER),
                                action: "StartCampaign".to_string(),
                            })
                            .unwrap(),
                            funds: vec![],
                        }),
                        ReplyId::PayFee.repr(),
                    ))),
            },
            StartCampaignTestCase {
                name: "start_campaign with unauthorized sender".to_string(),
                tiers: mock_campaign_tiers(),
                presale: Some(valid_presale.clone()),
                start_time: None,
                end_time: MillisecondsExpiration::from_seconds(env.block.time.seconds() + 100),
                payee: "owner1".to_string(),
                expected_res: Err(ContractError::Unauthorized {}),
            },
            StartCampaignTestCase {
                name: "start_campaign with no unlimited tier".to_string(),
                tiers: invalid_tiers,
                presale: Some(valid_presale.clone()),
                start_time: None,
                end_time: MillisecondsExpiration::from_seconds(env.block.time.seconds() + 100),
                payee: MOCK_DEFAULT_OWNER.to_string(),
                expected_res: Err(ContractError::InvalidTiers {}),
            },
            StartCampaignTestCase {
                name: "start_campaign with invalid presales".to_string(),
                tiers: mock_campaign_tiers(),
                presale: Some(invalid_presale.clone()),
                start_time: None,
                end_time: MillisecondsExpiration::from_seconds(env.block.time.seconds() + 100),
                payee: MOCK_DEFAULT_OWNER.to_string(),
                expected_res: Err(ContractError::InvalidTier {
                    operation: "set_tier_orders".to_string(),
                    msg: "Tier with level 2 does not exist".to_string(),
                }),
            },
            StartCampaignTestCase {
                name: "start_campaign with invalid end_time".to_string(),
                tiers: mock_campaign_tiers(),
                presale: Some(valid_presale.clone()),
                start_time: None,
                end_time: MillisecondsExpiration::from_seconds(env.block.time.seconds() - 100),
                payee: MOCK_DEFAULT_OWNER.to_string(),
                expected_res: Err(ContractError::StartTimeAfterEndTime {}),
            },
            StartCampaignTestCase {
                name: "start_campaign with invalid start_time".to_string(),
                tiers: mock_campaign_tiers(),
                presale: Some(valid_presale.clone()),
                start_time: Some(MillisecondsExpiration::from_seconds(
                    env.block.time.seconds() + 1000,
                )),
                end_time: MillisecondsExpiration::from_seconds(env.block.time.seconds() + 500),
                payee: MOCK_DEFAULT_OWNER.to_string(),
                expected_res: Err(ContractError::StartTimeAfterEndTime {}),
            },
        ];
        for test in test_cases {
            let mut deps = mock_dependencies_custom(&[coin(100000, MOCK_NATIVE_DENOM)]);

            let _ = init(
                deps.as_mut(),
                mock_campaign_config(Asset::NativeToken(MOCK_NATIVE_DENOM.to_string())),
                test.tiers.clone(),
            );
            let info = mock_info(&test.payee, &[]);

            let msg = ExecuteMsg::StartCampaign {
                start_time: test.start_time,
                end_time: test.end_time,
                presale: test.presale.clone(),
            };

            let res = execute(deps.as_mut(), env.clone(), info, msg);
            assert_eq!(res, test.expected_res, "Test case: {}", test.name);

            if res.is_ok() {
                assert_eq!(
                    CAMPAIGN_CONFIG.load(&deps.storage).unwrap().start_time,
                    test.start_time
                );
                assert_eq!(
                    CAMPAIGN_CONFIG.load(&deps.storage).unwrap().end_time,
                    test.end_time
                );
                assert_eq!(
                    CAMPAIGN_STAGE.load(&deps.storage).unwrap(),
                    CampaignStage::ONGOING
                );
                for order in &test.presale.unwrap() {
                    let order_amount: u128 = order.amount.into();
                    let order_info = TIER_ORDERS
                        .load(&deps.storage, (mock_orderer.clone(), order.level.into()))
                        .unwrap();

                    assert_eq!(order_info.preordered, order_amount);
                    assert_eq!(order_info.ordered, 0u128);
                    assert_eq!(order_info.amount().unwrap(), order_amount);
                    let cur_limit = TIERS.load(&deps.storage, order.level.into()).unwrap().limit;
                    if cur_limit.is_some() {
                        assert_eq!(
                            TIERS
                                .load(&deps.storage, order.level.into())
                                .unwrap()
                                .sold_amount
                                .u128(),
                            order_amount
                        );
                    }
                }
            } else {
                assert_eq!(
                    CAMPAIGN_STAGE
                        .load(&deps.storage)
                        .unwrap_or(CampaignStage::READY),
                    CampaignStage::READY
                );
            }
        }
    }

    struct PurchaseTierTestCase {
        name: String,
        stage: CampaignStage,
        expected_res: Result<Response, ContractError>,
        payee: String,
        start_time: Option<MillisecondsExpiration>,
        end_time: MillisecondsExpiration,
        orders: Vec<SimpleTierOrder>,
        initial_cap: Uint128,
        funds: Vec<Coin>,
        denom: Asset,
    }
    #[test]
    fn test_execute_purchase_tiers_native() {
        // fixed total cost to 100 for valid purchase
        let env = mock_env();
        let buyer = "buyer";
        let test_cases: Vec<PurchaseTierTestCase> = vec![
            PurchaseTierTestCase {
                name: "Standard purchase with valid order using native tokens".to_string(),
                stage: CampaignStage::ONGOING,
                expected_res: Ok(Response::new()
                    .add_attribute("action", "purchase_tiers")
                    .add_attribute("payment", "1000native:uandr")
                    .add_attribute("total_cost", "100")
                    .add_attribute("refunded", "900")
                    .add_message(BankMsg::Send {
                        to_address: buyer.to_string(),
                        // Refund sent back as they only were able to mint one.
                        amount: coins(900, MOCK_NATIVE_DENOM),
                    })
                    .add_submessage(SubMsg::reply_on_error(
                        CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: "economics_contract".to_string(),
                            msg: to_json_binary(&EconomicsExecuteMsg::PayFee {
                                payee: Addr::unchecked(buyer),
                                action: "PurchaseTiers".to_string(),
                            })
                            .unwrap(),
                            funds: vec![],
                        }),
                        ReplyId::PayFee.repr(),
                    ))),
                payee: buyer.to_string(),
                start_time: Some(past_time()),
                end_time: future_time(&env),
                orders: vec![SimpleTierOrder {
                    level: Uint64::one(),
                    amount: Uint128::new(10),
                }],
                initial_cap: Uint128::new(500),
                funds: vec![coin(1000, MOCK_NATIVE_DENOM)],
                denom: Asset::NativeToken(MOCK_NATIVE_DENOM.to_string()),
            },
            PurchaseTierTestCase {
                name: "Purchasing more than limit".to_string(),
                stage: CampaignStage::ONGOING,
                expected_res: Err(ContractError::PurchaseLimitReached {}),
                payee: buyer.to_string(),
                start_time: Some(past_time()),
                end_time: future_time(&env),
                orders: vec![SimpleTierOrder {
                    level: Uint64::one(),
                    amount: Uint128::new(MOCK_DEFAULT_LIMIT + 1),
                }],
                initial_cap: Uint128::new(500),
                funds: vec![coin(10 * MOCK_DEFAULT_LIMIT + 20, MOCK_NATIVE_DENOM)],
                denom: Asset::NativeToken(MOCK_NATIVE_DENOM.to_string()),
            },
            PurchaseTierTestCase {
                name: "Purchase with insufficient funds using native tokens".to_string(),
                stage: CampaignStage::ONGOING,
                expected_res: Err(ContractError::InsufficientFunds {}),
                payee: buyer.to_string(),
                start_time: Some(past_time()),
                end_time: future_time(&env),
                orders: vec![SimpleTierOrder {
                    level: Uint64::one(),
                    amount: Uint128::new(20),
                }],
                initial_cap: Uint128::new(500),
                funds: vec![coin(10, MOCK_NATIVE_DENOM)],
                denom: Asset::NativeToken(MOCK_NATIVE_DENOM.to_string()),
            },
            PurchaseTierTestCase {
                name: "Purchase in wrong campaign stage using native tokens".to_string(),
                stage: CampaignStage::READY,
                expected_res: Err(ContractError::InvalidCampaignOperation {
                    operation: "purchase_tiers".to_string(),
                    stage: "READY".to_string(),
                }),
                payee: buyer.to_string(),
                start_time: Some(past_time()),
                end_time: future_time(&env),
                orders: vec![SimpleTierOrder {
                    level: Uint64::one(),
                    amount: Uint128::new(10),
                }],
                initial_cap: Uint128::new(500),
                funds: vec![coin(1000, MOCK_NATIVE_DENOM)],
                denom: Asset::NativeToken(MOCK_NATIVE_DENOM.to_string()),
            },
            PurchaseTierTestCase {
                name: "Purchase before campaign start using native tokens".to_string(),
                stage: CampaignStage::ONGOING,
                expected_res: Err(ContractError::CampaignNotStarted {}),
                payee: buyer.to_string(),
                start_time: Some(future_time(&env)),
                end_time: future_time(&env),
                orders: vec![SimpleTierOrder {
                    level: Uint64::one(),
                    amount: Uint128::new(10),
                }],
                initial_cap: Uint128::new(500),
                funds: vec![coin(1000, MOCK_NATIVE_DENOM)],
                denom: Asset::NativeToken(MOCK_NATIVE_DENOM.to_string()),
            },
            PurchaseTierTestCase {
                name: "Purchase after campaign end using native tokens".to_string(),
                stage: CampaignStage::ONGOING,
                expected_res: Err(ContractError::CampaignEnded {}),
                payee: buyer.to_string(),
                start_time: Some(past_time()),
                end_time: past_time(),
                orders: vec![SimpleTierOrder {
                    level: Uint64::one(),
                    amount: Uint128::new(10),
                }],
                initial_cap: Uint128::new(500),
                funds: vec![coin(1000, MOCK_NATIVE_DENOM)],
                denom: Asset::NativeToken(MOCK_NATIVE_DENOM.to_string()),
            },
            PurchaseTierTestCase {
                name: "Purchase with invalid denomination using native tokens".to_string(),
                stage: CampaignStage::ONGOING,
                expected_res: Err(ContractError::InvalidFunds {
                    msg: format!("Only native:{MOCK_NATIVE_DENOM} is accepted by the campaign."),
                }),
                payee: buyer.to_string(),
                start_time: Some(past_time()),
                end_time: future_time(&env),
                orders: vec![SimpleTierOrder {
                    level: Uint64::one(),
                    amount: Uint128::new(10),
                }],
                initial_cap: Uint128::new(500),
                funds: vec![coin(1000, INVA1LID_DENOM)],
                denom: Asset::NativeToken(MOCK_NATIVE_DENOM.to_string()),
            },
        ];
        for test in test_cases {
            let mut deps = mock_dependencies_custom(&test.funds);
            let info = mock_info(&test.payee, &test.funds);

            // Mock necessary storage setup
            set_campaign_stage(deps.as_mut().storage, &test.stage);
            set_current_cap(deps.as_mut().storage, &test.initial_cap);
            set_tiers(deps.as_mut().storage, mock_campaign_tiers());

            let mut mock_config = mock_campaign_config(test.denom);
            mock_config.start_time = test.start_time;
            mock_config.end_time = test.end_time;
            set_campaign_config(deps.as_mut().storage, &mock_config);

            let msg = ExecuteMsg::PurchaseTiers {
                orders: test.orders.clone(),
            };

            let res = execute(deps.as_mut(), env.clone(), info, msg);
            assert_eq!(res, test.expected_res, "Test case: {}", test.name);

            if res.is_ok() {
                // Check current capital
                let updated_cap = get_current_cap(deps.as_ref().storage);
                let expected_cap = test.initial_cap + Uint128::new(100);
                assert_eq!(updated_cap, expected_cap, "Test case: {}", test.name);

                // Check tier orders
                for order in &test.orders {
                    let stored_order_info = TIER_ORDERS
                        .load(
                            deps.as_ref().storage,
                            (Addr::unchecked(buyer), order.level.into()),
                        )
                        .unwrap();
                    assert_eq!(
                        stored_order_info.ordered,
                        order.amount.u128(),
                        "Test case: {}",
                        test.name
                    );
                }

                // Check tier limits
                for order in &test.orders {
                    let tier = TIERS
                        .load(deps.as_ref().storage, order.level.into())
                        .unwrap();
                    assert_eq!(
                        tier.sold_amount.u128(),
                        order.amount.u128(),
                        "Test case: {}",
                        test.name
                    );
                }
            }
        }
    }

    #[test]
    fn test_execute_purchase_tiers_cw20() {
        // fixed total cost to 100 for valid purchase
        let env = mock_env();
        let buyer = "buyer";
        let valid_denom = Asset::Cw20Token(AndrAddr::from_string(MOCK_CW20_CONTRACT.to_string()));
        let test_cases: Vec<PurchaseTierTestCase> = vec![
            PurchaseTierTestCase {
                name: "Standard purchase with valid order using cw20 token".to_string(),
                stage: CampaignStage::ONGOING,
                expected_res: Ok(Response::new()
                    .add_attribute("action", "purchase_tiers")
                    .add_attribute("payment", "1000cw20:cw20_contract")
                    .add_attribute("total_cost", "100")
                    .add_attribute("refunded", "900")
                    .add_message(
                        wasm_execute(
                            MOCK_CW20_CONTRACT.to_string(),
                            &Cw20ExecuteMsg::Transfer {
                                recipient: buyer.to_string(),
                                amount: Uint128::new(900u128),
                            },
                            vec![],
                        )
                        .unwrap(),
                    )
                    .add_submessage(SubMsg::reply_on_error(
                        CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: "economics_contract".to_string(),
                            msg: to_json_binary(&EconomicsExecuteMsg::PayFee {
                                payee: Addr::unchecked(MOCK_CW20_CONTRACT),
                                action: "Receive".to_string(),
                            })
                            .unwrap(),
                            funds: vec![],
                        }),
                        ReplyId::PayFee.repr(),
                    ))),
                payee: buyer.to_string(),
                start_time: Some(past_time()),
                end_time: future_time(&env),
                orders: vec![SimpleTierOrder {
                    level: Uint64::one(),
                    amount: Uint128::new(10),
                }],
                initial_cap: Uint128::new(500),
                funds: vec![],
                denom: valid_denom.clone(),
            },
            PurchaseTierTestCase {
                name: "Purchase with insufficient funds using cw20 token".to_string(),
                stage: CampaignStage::ONGOING,
                expected_res: Err(ContractError::InsufficientFunds {}),
                payee: buyer.to_string(),
                start_time: Some(past_time()),
                end_time: future_time(&env),
                orders: vec![SimpleTierOrder {
                    level: Uint64::one(),
                    amount: Uint128::new(200000),
                }],
                initial_cap: Uint128::new(500),
                funds: vec![],
                denom: valid_denom.clone(),
            },
            PurchaseTierTestCase {
                name: "Purchase in wrong campaign stage using cw20 token".to_string(),
                stage: CampaignStage::READY,
                expected_res: Err(ContractError::InvalidCampaignOperation {
                    operation: "purchase_tiers".to_string(),
                    stage: "READY".to_string(),
                }),
                payee: buyer.to_string(),
                start_time: Some(past_time()),
                end_time: future_time(&env),
                orders: vec![SimpleTierOrder {
                    level: Uint64::one(),
                    amount: Uint128::new(10),
                }],
                initial_cap: Uint128::new(500),
                funds: vec![],
                denom: valid_denom.clone(),
            },
            PurchaseTierTestCase {
                name: "Purchase before campaign start using cw20 token".to_string(),
                stage: CampaignStage::ONGOING,
                expected_res: Err(ContractError::CampaignNotStarted {}),
                payee: buyer.to_string(),
                start_time: Some(future_time(&env)),
                end_time: future_time(&env),
                orders: vec![SimpleTierOrder {
                    level: Uint64::one(),
                    amount: Uint128::new(10),
                }],
                initial_cap: Uint128::new(500),
                funds: vec![],
                denom: valid_denom.clone(),
            },
            PurchaseTierTestCase {
                name: "Purchase after campaign end using cw20 token".to_string(),
                stage: CampaignStage::ONGOING,
                expected_res: Err(ContractError::CampaignEnded {}),
                payee: buyer.to_string(),
                start_time: Some(past_time()),
                end_time: past_time(),
                orders: vec![SimpleTierOrder {
                    level: Uint64::one(),
                    amount: Uint128::new(10),
                }],
                initial_cap: Uint128::new(500),
                funds: vec![],
                denom: valid_denom.clone(),
            },
            PurchaseTierTestCase {
                name: "Purchase with invalid denomination using cw20 token".to_string(),
                stage: CampaignStage::ONGOING,
                expected_res: Err(ContractError::InvalidFunds {
                    msg: format!("Only cw20:{MOCK_CW20_CONTRACT} is accepted by the campaign."),
                }),
                payee: buyer.to_string(),
                start_time: Some(past_time()),
                end_time: future_time(&env),
                orders: vec![SimpleTierOrder {
                    level: Uint64::one(),
                    amount: Uint128::new(10),
                }],
                initial_cap: Uint128::new(500),
                funds: vec![],
                denom: Asset::Cw20Token(AndrAddr::from_string("cw20_contract124".to_string())),
            },
        ];

        for test in test_cases {
            let mut deps = mock_dependencies_custom(&test.funds);
            let Asset::Cw20Token(ref cw20) = test.denom else {
                todo!();
            };
            let info = mock_info(cw20.as_ref(), &[]);

            // Mock necessary storage setup
            set_campaign_stage(deps.as_mut().storage, &test.stage);
            set_current_cap(deps.as_mut().storage, &test.initial_cap);
            set_tiers(deps.as_mut().storage, mock_campaign_tiers());

            let mut mock_config = mock_campaign_config(valid_denom.clone());
            mock_config.start_time = test.start_time;
            mock_config.end_time = test.end_time;
            set_campaign_config(deps.as_mut().storage, &mock_config);

            let hook_msg = Cw20HookMsg::PurchaseTiers {
                orders: test.orders.clone(),
            };
            let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
                sender: buyer.to_owned(),
                amount: Uint128::new(1000u128),
                msg: encode_binary(&hook_msg).unwrap(),
            });

            let res = execute(deps.as_mut(), env.clone(), info, msg);
            assert_eq!(res, test.expected_res, "Test case: {}", test.name);

            if res.is_ok() {
                // Check current capital
                let updated_cap = get_current_cap(deps.as_ref().storage);
                let expected_cap = test.initial_cap + Uint128::new(100);
                assert_eq!(updated_cap, expected_cap, "Test case: {}", test.name);

                // Check tier orders
                for order in &test.orders {
                    let stored_order_info = TIER_ORDERS
                        .load(
                            deps.as_ref().storage,
                            (Addr::unchecked(buyer), order.level.into()),
                        )
                        .unwrap();
                    assert_eq!(
                        stored_order_info.ordered,
                        order.amount.u128(),
                        "Test case: {}",
                        test.name
                    );
                }

                // Check tier limits
                for order in &test.orders {
                    let tier = TIERS
                        .load(deps.as_ref().storage, order.level.into())
                        .unwrap();
                    assert_eq!(
                        tier.sold_amount.u128(),
                        order.amount.u128(),
                        "Test case: {}",
                        test.name
                    );
                }
            }
        }
    }

    struct EndCampaignTestCase {
        name: String,
        stage: CampaignStage,
        sender: String,
        current_cap: Uint128,
        soft_cap: Option<Uint128>,
        end_time: MillisecondsExpiration,
        denom: Asset,
        is_discard: bool,
        expected_res: Result<Response, ContractError>,
        expected_stage: CampaignStage,
    }
    #[test]
    fn test_execute_end_campaign() {
        let env = mock_env();
        let deps = mock_dependencies_custom(&[coin(100000, MOCK_NATIVE_DENOM)]);
        let recipient = Recipient::from_string(MOCK_WITHDRAWAL_ADDRESS.to_owned());
        let amp_msg = recipient
            .generate_amp_msg(&deps.as_ref(), Some(coins(10000, MOCK_NATIVE_DENOM)))
            .unwrap();
        let amp_pkt = AMPPkt::new(
            MOCK_CONTRACT_ADDR.to_string(),
            MOCK_CONTRACT_ADDR.to_string(),
            vec![amp_msg],
        );
        let amp_msg = amp_pkt
            .to_sub_msg(
                MOCK_KERNEL_CONTRACT,
                Some(coins(10000, MOCK_NATIVE_DENOM)),
                1,
            )
            .unwrap();

        let test_cases: Vec<EndCampaignTestCase> = vec![
            EndCampaignTestCase {
                name: "Successful campaign using native token".to_string(),
                stage: CampaignStage::ONGOING,
                sender: MOCK_DEFAULT_OWNER.to_string(),
                current_cap: Uint128::new(10000u128),
                soft_cap: Some(Uint128::new(9000u128)),
                end_time: MillisecondsExpiration::from_seconds(env.block.time.seconds()),
                denom: Asset::NativeToken(MOCK_NATIVE_DENOM.to_string()),
                is_discard: false,
                expected_res: Ok(Response::new()
                    .add_attribute("action", "end_campaign")
                    .add_attribute("result", CampaignStage::SUCCESS.to_string())
                    .add_submessage(amp_msg)
                    .add_submessage(SubMsg::reply_on_error(
                        CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: "economics_contract".to_string(),
                            msg: to_json_binary(&EconomicsExecuteMsg::PayFee {
                                payee: Addr::unchecked(MOCK_DEFAULT_OWNER.to_string()),
                                action: "EndCampaign".to_string(),
                            })
                            .unwrap(),
                            funds: vec![],
                        }),
                        ReplyId::PayFee.repr(),
                    ))),
                expected_stage: CampaignStage::SUCCESS,
            },
            EndCampaignTestCase {
                name: "Successful campaign using cw20".to_string(),
                stage: CampaignStage::ONGOING,
                sender: MOCK_DEFAULT_OWNER.to_string(),
                current_cap: Uint128::new(10000u128),
                soft_cap: Some(Uint128::new(9000u128)),
                end_time: MillisecondsExpiration::from_seconds(env.block.time.seconds()),
                denom: Asset::Cw20Token(AndrAddr::from_string(MOCK_CW20_CONTRACT.to_string())),
                is_discard: false,
                expected_res: Ok(Response::new()
                    .add_attribute("action", "end_campaign")
                    .add_attribute("result", CampaignStage::SUCCESS.to_string())
                    .add_message(
                        wasm_execute(
                            MOCK_CW20_CONTRACT.to_string(),
                            &Cw20ExecuteMsg::Transfer {
                                recipient: MOCK_WITHDRAWAL_ADDRESS.to_string(),
                                amount: Uint128::new(10000u128),
                            },
                            vec![],
                        )
                        .unwrap(),
                    )
                    .add_submessage(SubMsg::reply_on_error(
                        CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: "economics_contract".to_string(),
                            msg: to_json_binary(&EconomicsExecuteMsg::PayFee {
                                payee: Addr::unchecked(MOCK_DEFAULT_OWNER.to_string()),
                                action: "EndCampaign".to_string(),
                            })
                            .unwrap(),
                            funds: vec![],
                        }),
                        ReplyId::PayFee.repr(),
                    ))),
                expected_stage: CampaignStage::SUCCESS,
            },
            EndCampaignTestCase {
                name: "Failed campaign".to_string(),
                stage: CampaignStage::ONGOING,
                sender: MOCK_DEFAULT_OWNER.to_string(),
                current_cap: Uint128::new(10000u128),
                soft_cap: Some(Uint128::new(11000u128)),
                end_time: MillisecondsExpiration::from_seconds(env.block.time.seconds()),
                denom: Asset::Cw20Token(AndrAddr::from_string(MOCK_CW20_CONTRACT.to_string())),
                is_discard: false,
                expected_res: Ok(Response::new()
                    .add_attribute("action", "end_campaign")
                    .add_attribute("result", CampaignStage::FAILED.to_string())
                    .add_submessage(SubMsg::reply_on_error(
                        CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: "economics_contract".to_string(),
                            msg: to_json_binary(&EconomicsExecuteMsg::PayFee {
                                payee: Addr::unchecked(MOCK_DEFAULT_OWNER.to_string()),
                                action: "EndCampaign".to_string(),
                            })
                            .unwrap(),
                            funds: vec![],
                        }),
                        ReplyId::PayFee.repr(),
                    ))),
                expected_stage: CampaignStage::FAILED,
            },
            EndCampaignTestCase {
                name: "Discard campaign using native token".to_string(),
                stage: CampaignStage::ONGOING,
                sender: MOCK_DEFAULT_OWNER.to_string(),
                current_cap: Uint128::new(10000u128),
                soft_cap: Some(Uint128::new(9000u128)),
                end_time: MillisecondsExpiration::from_seconds(env.block.time.seconds()),
                denom: Asset::NativeToken(MOCK_NATIVE_DENOM.to_string()),
                is_discard: true,
                expected_res: Ok(Response::new()
                    .add_attribute("action", "discard_campaign")
                    .add_attribute("result", CampaignStage::FAILED.to_string())
                    .add_submessage(SubMsg::reply_on_error(
                        CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: "economics_contract".to_string(),
                            msg: to_json_binary(&EconomicsExecuteMsg::PayFee {
                                payee: Addr::unchecked(MOCK_DEFAULT_OWNER.to_string()),
                                action: "DiscardCampaign".to_string(),
                            })
                            .unwrap(),
                            funds: vec![],
                        }),
                        ReplyId::PayFee.repr(),
                    ))),
                expected_stage: CampaignStage::FAILED,
            },
            EndCampaignTestCase {
                name: "Pause campaign".to_string(),
                stage: CampaignStage::ONGOING,
                sender: MOCK_DEFAULT_OWNER.to_string(),
                current_cap: Uint128::new(0u128),
                soft_cap: None,
                end_time: MillisecondsExpiration::from_seconds(env.block.time.seconds() + 1000),
                denom: Asset::Cw20Token(AndrAddr::from_string(MOCK_CW20_CONTRACT.to_string())),
                is_discard: false,
                expected_res: Ok(Response::new()
                    .add_attribute("action", "end_campaign")
                    .add_attribute("result", CampaignStage::READY.to_string())
                    .add_submessage(SubMsg::reply_on_error(
                        CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: "economics_contract".to_string(),
                            msg: to_json_binary(&EconomicsExecuteMsg::PayFee {
                                payee: Addr::unchecked(MOCK_DEFAULT_OWNER.to_string()),
                                action: "EndCampaign".to_string(),
                            })
                            .unwrap(),
                            funds: vec![],
                        }),
                        ReplyId::PayFee.repr(),
                    ))),
                expected_stage: CampaignStage::READY,
            },
            EndCampaignTestCase {
                name: "End campaign from unauthorized sender".to_string(),
                stage: CampaignStage::ONGOING,
                sender: "sender".to_string(),
                current_cap: Uint128::new(10000u128),
                soft_cap: None,
                end_time: MillisecondsExpiration::from_seconds(env.block.time.seconds()),
                denom: Asset::NativeToken(MOCK_NATIVE_DENOM.to_string()),
                is_discard: false,
                expected_res: Err(ContractError::Unauthorized {}),
                expected_stage: CampaignStage::ONGOING,
            },
            EndCampaignTestCase {
                name: "End campaign on invalid stage".to_string(),
                stage: CampaignStage::READY,
                sender: MOCK_DEFAULT_OWNER.to_string(),
                current_cap: Uint128::new(10000u128),
                soft_cap: None,
                end_time: MillisecondsExpiration::from_seconds(env.block.time.seconds()),
                denom: Asset::NativeToken(MOCK_NATIVE_DENOM.to_string()),
                is_discard: false,
                expected_res: Err(ContractError::InvalidCampaignOperation {
                    operation: "end_campaign".to_string(),
                    stage: CampaignStage::READY.to_string(),
                }),
                expected_stage: CampaignStage::READY,
            },
            EndCampaignTestCase {
                name: "End unexpired campaign".to_string(),
                stage: CampaignStage::ONGOING,
                sender: MOCK_DEFAULT_OWNER.to_string(),
                current_cap: Uint128::new(10000u128),
                soft_cap: Some(Uint128::new(11000u128)),
                end_time: MillisecondsExpiration::from_seconds(env.block.time.seconds() + 100),
                denom: Asset::NativeToken(MOCK_NATIVE_DENOM.to_string()),
                is_discard: false,
                expected_res: Err(ContractError::CampaignNotExpired {}),
                expected_stage: CampaignStage::ONGOING,
            },
        ];
        for test in test_cases {
            let mut deps = mock_dependencies_custom(&[coin(100000, MOCK_NATIVE_DENOM)]);
            let mut mock_config = mock_campaign_config(test.denom.clone());
            let _ = init(deps.as_mut(), mock_config.clone(), vec![]);

            let info = mock_info(&test.sender, &[]);
            set_campaign_stage(deps.as_mut().storage, &test.stage);
            set_current_cap(deps.as_mut().storage, &test.current_cap);

            mock_config.end_time = test.end_time;
            mock_config.soft_cap = test.soft_cap;
            set_campaign_config(deps.as_mut().storage, &mock_config);
            let msg = if test.is_discard {
                ExecuteMsg::DiscardCampaign {}
            } else {
                ExecuteMsg::EndCampaign {}
            };

            let res = execute(deps.as_mut(), env.clone(), info, msg);
            assert_eq!(res, test.expected_res, "Test case: {}", test.name);
            if res.is_ok() {
                assert_eq!(
                    CAMPAIGN_STAGE
                        .load(&deps.storage)
                        .unwrap_or(CampaignStage::SUCCESS),
                    test.expected_stage,
                    "Test case: {}",
                    test.name
                );
            }
        }
    }

    struct ClaimTestCase {
        name: String,
        stage: CampaignStage,
        orders: Vec<TierOrder>,
        denom: Asset,
        expected_res: Result<Response, ContractError>,
    }

    #[test]
    fn test_execute_claim() {
        let orderer = Addr::unchecked("orderer");

        let test_cases = vec![
            ClaimTestCase {
                name: "Claim when campaign is successful ".to_string(),
                stage: CampaignStage::SUCCESS,
                orders: vec![
                    TierOrder {
                        is_presale: true,
                        amount: Uint128::one(),
                        level: Uint64::one(),
                        orderer: orderer.clone(),
                    },
                    TierOrder {
                        is_presale: false,
                        amount: Uint128::one(),
                        level: Uint64::one(),
                        orderer: orderer.clone(),
                    },
                ],
                denom: Asset::NativeToken(MOCK_NATIVE_DENOM.to_string()),
                expected_res: Ok(Response::new()
                    .add_attribute("action", "claim")
                    .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: "tier_contract".to_string(),
                        msg: to_json_binary(&Cw721ExecuteMsg::Mint {
                            token_id: "0".to_string(),
                            owner: orderer.to_string(),
                            extension: TokenExtension {
                                publisher: MOCK_ADO_PUBLISHER.to_string(),
                            },
                            token_uri: None,
                        })
                        .unwrap(),
                        funds: vec![],
                    }))
                    .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: "tier_contract".to_string(),
                        msg: to_json_binary(&Cw721ExecuteMsg::Mint {
                            token_id: "1".to_string(),
                            owner: orderer.to_string(),
                            extension: TokenExtension {
                                publisher: MOCK_ADO_PUBLISHER.to_string(),
                            },
                            token_uri: None,
                        })
                        .unwrap(),
                        funds: vec![],
                    }))
                    .add_submessage(SubMsg::reply_on_error(
                        CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: "economics_contract".to_string(),
                            msg: to_json_binary(&EconomicsExecuteMsg::PayFee {
                                payee: orderer.clone(),
                                action: "Claim".to_string(),
                            })
                            .unwrap(),
                            funds: vec![],
                        }),
                        ReplyId::PayFee.repr(),
                    ))),
            },
            ClaimTestCase {
                name: "Claim when native token accepting campaign failed ".to_string(),
                stage: CampaignStage::FAILED,
                orders: vec![
                    TierOrder {
                        is_presale: true,
                        amount: Uint128::one(),
                        level: Uint64::one(),
                        orderer: orderer.clone(),
                    },
                    TierOrder {
                        is_presale: false,
                        amount: Uint128::one(),
                        level: Uint64::one(),
                        orderer: orderer.clone(),
                    },
                ],
                denom: Asset::NativeToken(MOCK_NATIVE_DENOM.to_string()),
                expected_res: Ok(Response::new()
                    .add_attribute("action", "claim")
                    .add_message(BankMsg::Send {
                        to_address: orderer.to_string(),
                        amount: coins(10, MOCK_NATIVE_DENOM), // only non presale order refunded
                    })
                    .add_submessage(SubMsg::reply_on_error(
                        CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: "economics_contract".to_string(),
                            msg: to_json_binary(&EconomicsExecuteMsg::PayFee {
                                payee: orderer.clone(),
                                action: "Claim".to_string(),
                            })
                            .unwrap(),
                            funds: vec![],
                        }),
                        ReplyId::PayFee.repr(),
                    ))),
            },
            ClaimTestCase {
                name: "Claim when cw20 accepting campaign failed ".to_string(),
                stage: CampaignStage::FAILED,
                orders: vec![
                    TierOrder {
                        is_presale: true,
                        amount: Uint128::one(),
                        level: Uint64::one(),
                        orderer: orderer.clone(),
                    },
                    TierOrder {
                        is_presale: false,
                        amount: Uint128::one(),
                        level: Uint64::one(),
                        orderer: orderer.clone(),
                    },
                ],
                denom: Asset::Cw20Token(AndrAddr::from_string(MOCK_CW20_CONTRACT.to_string())),
                expected_res: Ok(Response::new()
                    .add_attribute("action", "claim")
                    .add_message(
                        wasm_execute(
                            MOCK_CW20_CONTRACT.to_string(),
                            &Cw20ExecuteMsg::Transfer {
                                recipient: orderer.to_string(),
                                amount: Uint128::new(10u128),
                            },
                            vec![],
                        )
                        .unwrap(),
                    )
                    .add_submessage(SubMsg::reply_on_error(
                        CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: "economics_contract".to_string(),
                            msg: to_json_binary(&EconomicsExecuteMsg::PayFee {
                                payee: orderer.clone(),
                                action: "Claim".to_string(),
                            })
                            .unwrap(),
                            funds: vec![],
                        }),
                        ReplyId::PayFee.repr(),
                    ))),
            },
            ClaimTestCase {
                name: "Claim without purchasing in successful campaign".to_string(),
                stage: CampaignStage::SUCCESS,
                orders: vec![],
                denom: Asset::NativeToken(MOCK_NATIVE_DENOM.to_string()),
                expected_res: Err(ContractError::NoPurchases {}),
            },
            ClaimTestCase {
                name: "Claim without purchasing in failed campaign".to_string(),
                stage: CampaignStage::FAILED,
                orders: vec![],
                denom: Asset::NativeToken(MOCK_NATIVE_DENOM.to_string()),
                expected_res: Err(ContractError::NoPurchases {}),
            },
            ClaimTestCase {
                name: "Claim on invalid stage".to_string(),
                stage: CampaignStage::READY,
                orders: vec![],
                denom: Asset::NativeToken(MOCK_NATIVE_DENOM.to_string()),
                expected_res: Err(ContractError::InvalidCampaignOperation {
                    operation: "Claim".to_string(),
                    stage: CampaignStage::READY.to_string(),
                }),
            },
        ];
        for test in test_cases {
            let mut deps = mock_dependencies_custom(&[coin(100000, MOCK_NATIVE_DENOM)]);
            let env = mock_env();
            let mock_config = mock_campaign_config(test.denom.clone());
            set_campaign_config(deps.as_mut().storage, &mock_config);
            set_current_stage(deps.as_mut().storage, test.stage).unwrap();
            set_tiers(deps.as_mut().storage, mock_campaign_tiers());
            set_tier_orders(deps.as_mut().storage, test.orders).unwrap();
            let msg = ExecuteMsg::Claim {};

            let info = mock_info(orderer.as_ref(), &[]);

            let res = execute(deps.as_mut(), env.clone(), info, msg);
            assert_eq!(res, test.expected_res, "Test case: {}", test.name);
            if res.is_ok() {
                // processed orders should be cleared
                let orders = get_user_orders(deps.as_ref().storage, orderer.clone()).unwrap();
                assert!(orders.is_empty(), "Test case: {}", test.name);
            }
        }
    }
}
