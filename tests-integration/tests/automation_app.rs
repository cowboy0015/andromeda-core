use std::str::FromStr;

use andromeda_app::app::AppComponent;
use andromeda_app_contract::mock::{
    mock_andromeda_process, mock_claim_ownership_msg, mock_get_address_msg,
    mock_get_components_msg, mock_process_instantiate_msg,
};
use andromeda_automation::{
    condition::LogicGate,
    evaluation::Operators,
    oracle::{RegularTypes, TypeOfResponse},
};
use andromeda_condition::mock::{
    mock_andromeda_condition, mock_condition_get_results_msg, mock_condition_instantiate_msg,
};
use andromeda_counter::mock::{
    mock_andromeda_counter, mock_counter_increment_one_msg, mock_counter_increment_two_msg,
    mock_counter_instantiate_msg,
};

use andromeda_evaluation::mock::{
    mock_andromeda_evaluation, mock_evaluation_instantiate_msg, mock_evaluation_msg,
};

use andromeda_execute::mock::{
    mock_andromeda_execute, mock_execute_instantiate_msg, mock_execute_msg,
};
use andromeda_oracle::mock::{mock_andromeda_oracle, mock_oracle_instantiate_msg, mock_oracle_msg};
use andromeda_storage::mock::{
    mock_andromeda_storage, mock_storage_instantiate_msg, mock_storage_remove_msg,
    mock_storage_store_msg,
};
use andromeda_task_balancer::mock::{
    mock_andromeda_task_balancer, mock_task_balancer_instantiate_msg,
    mock_task_balancer_remove_msg, mock_task_balancer_store_msg,
};

use andromeda_testing::mock::MockAndromeda;
use common::{
    ado_base::recipient::{ADORecipient, Recipient},
    app::AndrAddress,
};
use cosmwasm_std::{coin, to_binary, Addr, BlockInfo, Coin, Decimal, Uint128};
use cw_multi_test::{App, Executor};
use cw_utils::Expiration;

fn mock_app() -> App {
    App::new(|router, _api, storage| {
        router
            .bank
            .init_balance(
                storage,
                &Addr::unchecked("owner"),
                [coin(999999, "uandr")].to_vec(),
            )
            .unwrap();
    })
}

fn mock_andromeda(app: &mut App, admin_address: Addr) -> MockAndromeda {
    MockAndromeda::new(app, &admin_address)
}

#[test]
fn test_automatiom_app() {
    let owner = Addr::unchecked("owner");

    let mut router = mock_app();
    let andr = mock_andromeda(&mut router, owner.clone());
    // Store contract codes
    let condition_code_id = router.store_code(mock_andromeda_condition());
    let counter_code_id = router.store_code(mock_andromeda_counter());
    let evaluation_code_id = router.store_code(mock_andromeda_evaluation());
    let execute_code_id = router.store_code(mock_andromeda_execute());
    let oracle_code_id = router.store_code(mock_andromeda_oracle());
    let storage_code_id = router.store_code(mock_andromeda_storage());
    let task_balancer_code_id = router.store_code(mock_andromeda_task_balancer());
    let process_code_id = router.store_code(mock_andromeda_process());

    andr.store_code_id(&mut router, "condition", condition_code_id);
    andr.store_code_id(&mut router, "counter", counter_code_id);
    andr.store_code_id(&mut router, "evaluation", evaluation_code_id);
    andr.store_code_id(&mut router, "execute", execute_code_id);
    andr.store_code_id(&mut router, "oracle", oracle_code_id);
    andr.store_code_id(&mut router, "storage", storage_code_id);
    andr.store_code_id(&mut router, "task_balancer", task_balancer_code_id);
    andr.store_code_id(&mut router, "process", process_code_id);

    // Generate App Components
    // App component names must be less than 3 characters or longer than 54 characters to force them to be 'invalid' as the MockApi struct used within the CosmWasm App struct only contains those two validation checks
    let evaluation_andr_address = AndrAddress {
        identifier: "eval".to_string(),
    };
    let execute_andr_address = AndrAddress {
        identifier: "execute".to_string(),
    };
    let condition_andr_address = AndrAddress {
        identifier: "condition".to_string(),
    };
    let oracle_andr_address = AndrAddress {
        identifier: "oracle".to_string(),
    };
    let task_balancer_andr_address = AndrAddress {
        identifier: "task_balancer".to_string(),
    };
    let counter_andr_address = AndrAddress {
        identifier: "counter".to_string(),
    };
    let increment_one = to_binary(&"eyJpbmNyZW1lbnRfb25lIjp7fX0=".to_string()).unwrap();
    let increment_two = to_binary(&"eyJpbmNyZW1lbnRfdHdvIjp7fX0=".to_string()).unwrap();
    let reset = to_binary(&"eyJyZXNldCI6e319".to_string()).unwrap();
    let query_count = to_binary(&"eyJjb3VudCI6e319".to_string()).unwrap();
    let addr_task_balancer = Addr::unchecked("task_balancer");
    let addr_process = Addr::unchecked("process");

    // Condition ADO
    let condition_init_msg = mock_condition_instantiate_msg(
        LogicGate::And,
        vec![evaluation_andr_address],
        execute_andr_address.clone(),
    );
    let condition_app_component = AppComponent {
        name: "1".to_string(),
        ado_type: "condition".to_string(),
        instantiate_msg: to_binary(&condition_init_msg).unwrap(),
    };

    // Counter ADO
    let counter_init_msg = mock_counter_instantiate_msg(vec![execute_andr_address]);
    let counter_app_component = AppComponent {
        name: "2".to_string(),
        ado_type: "counter".to_string(),
        instantiate_msg: to_binary(&counter_init_msg).unwrap(),
    };

    // Evaluation ADO
    let evaluation_init_msg = mock_evaluation_instantiate_msg(
        condition_andr_address.clone(),
        oracle_andr_address,
        task_balancer_andr_address.clone(),
        Some(Uint128::new(1)),
        Operators::Equal,
    );

    let evaluation_app_component = AppComponent {
        name: "3".to_string(),
        ado_type: "evaluation".to_string(),
        instantiate_msg: to_binary(&evaluation_init_msg).unwrap(),
    };

    // Execute ADO that increments by one
    let execute_init_msg = mock_execute_instantiate_msg(
        counter_andr_address.clone(),
        condition_andr_address.clone(),
        task_balancer_andr_address.identifier,
        increment_one,
    );

    let execute_app_component = AppComponent {
        name: "4".to_string(),
        ado_type: "execute".to_string(),
        instantiate_msg: to_binary(&execute_init_msg).unwrap(),
    };

    // Oracle ADO

    let oracle_init_msg = mock_oracle_instantiate_msg(
        counter_andr_address.identifier,
        query_count,
        TypeOfResponse::RegularType(RegularTypes::Uint128),
        None,
    );

    let oracle_app_component = AppComponent {
        name: "5".to_string(),
        ado_type: "oracle".to_string(),
        instantiate_msg: to_binary(&oracle_init_msg).unwrap(),
    };

    // Storage ADO

    let storage_init_msg = mock_storage_instantiate_msg(addr_task_balancer, addr_process, 3);

    let storage_app_component = AppComponent {
        name: "6".to_string(),
        ado_type: "storage".to_string(),
        instantiate_msg: to_binary(&storage_init_msg).unwrap(),
    };

    // Task balancer ADO

    let task_balancer_init_msg = mock_task_balancer_instantiate_msg(3, storage_code_id);

    let task_balancer_app_component = AppComponent {
        name: "7".to_string(),
        ado_type: "task_balancer".to_string(),
        instantiate_msg: to_binary(&task_balancer_init_msg).unwrap(),
    };

    // Process ADO

    let process_components = vec![
        oracle_app_component,
        counter_app_component,
        execute_app_component,
        storage_app_component,
        condition_app_component,
        evaluation_app_component,
        task_balancer_app_component,
    ];
    let process_init_msg = mock_process_instantiate_msg(
        "Pro".to_string(),
        process_components.clone(),
        andr.registry_address.to_string(),
        vec![condition_andr_address.identifier],
    );

    let process_addr = router
        .instantiate_contract(
            process_code_id,
            owner.clone(),
            &process_init_msg,
            &[],
            "Pro",
            Some(owner.to_string()),
        )
        .unwrap();

    let components: Vec<AppComponent> = router
        .wrap()
        .query_wasm_smart(process_addr.clone(), &mock_get_components_msg())
        .unwrap();

    assert_eq!(components, process_components);

    // router
    //     .execute_contract(
    //         owner.clone(),
    //         app_addr.clone(),
    //         &mock_claim_ownership_msg(None),
    //         &[],
    //     )
    //     .unwrap();

    // let crowdfund_addr: String = router
    //     .wrap()
    //     .query_wasm_smart(
    //         app_addr.clone(),
    //         &mock_get_address_msg(crowdfund_app_component.name),
    //     )
    //     .unwrap();

    // // Mint Tokens
    // let mint_msg = mock_crowdfund_quick_mint_msg(5, owner.to_string());
    // router
    //     .execute_contract(
    //         owner.clone(),
    //         Addr::unchecked(crowdfund_addr.clone()),
    //         &mint_msg,
    //         &[],
    //     )
    //     .unwrap();

    // // Start Sale
    // let token_price = coin(100, "uandr");
    // let sale_recipient = Recipient::ADO(ADORecipient {
    //     address: AndrAddress {
    //         identifier: splitter_app_component.name,
    //     },
    //     msg: Some(to_binary(&mock_splitter_send_msg()).unwrap()),
    // });
    // let start_msg = mock_start_crowdfund_msg(
    //     Expiration::AtHeight(router.block_info().height + 5),
    //     token_price.clone(),
    //     Uint128::from(3u128),
    //     Some(1),
    //     sale_recipient,
    // );
    // router
    //     .execute_contract(
    //         owner.clone(),
    //         Addr::unchecked(crowdfund_addr.clone()),
    //         &start_msg,
    //         &[],
    //     )
    //     .unwrap();

    // // Buy Tokens
    // let buyers = vec![buyer_one, buyer_two, buyer_three];
    // for buyer in buyers.clone() {
    //     let purchase_msg = mock_purchase_msg(Some(1));
    //     router
    //         .execute_contract(
    //             buyer,
    //             Addr::unchecked(crowdfund_addr.clone()),
    //             &purchase_msg,
    //             &[token_price.clone()],
    //         )
    //         .unwrap();
    // }

    // // End Sale
    // let block_info = router.block_info();
    // router.set_block(BlockInfo {
    //     height: block_info.height + 5,
    //     time: block_info.time,
    //     chain_id: block_info.chain_id,
    // });
    // let end_sale_msg = mock_end_crowdfund_msg(None);
    // router
    //     .execute_contract(
    //         owner.clone(),
    //         Addr::unchecked(crowdfund_addr.clone()),
    //         &end_sale_msg,
    //         &[],
    //     )
    //     .unwrap();
    // router
    //     .execute_contract(owner, Addr::unchecked(crowdfund_addr), &end_sale_msg, &[])
    //     .unwrap();

    // // Check final state
    // //Check token transfers
    // let cw721_addr: String = router
    //     .wrap()
    //     .query_wasm_smart(
    //         app_addr.clone(),
    //         &mock_get_address_msg(cw721_component.name),
    //     )
    //     .unwrap();
    // for (i, buyer) in buyers.iter().enumerate() {
    //     let query_msg = mock_cw721_owner_of(i.to_string(), None);
    //     let owner: OwnerOfResponse = router
    //         .wrap()
    //         .query_wasm_smart(cw721_addr.clone(), &query_msg)
    //         .unwrap();

    //     assert_eq!(owner.owner, buyer.to_string());
    // }

    // //Check vault balances
    // let vault_one_addr: String = router
    //     .wrap()
    //     .query_wasm_smart(
    //         app_addr.clone(),
    //         &mock_get_address_msg(vault_one_app_component.name),
    //     )
    //     .unwrap();
    // let balance_one: Vec<Coin> = router
    //     .wrap()
    //     .query_wasm_smart(
    //         vault_one_addr,
    //         &mock_vault_get_balance(
    //             vault_one_recipient_addr.to_string(),
    //             Some("uandr".to_string()),
    //             None,
    //         ),
    //     )
    //     .unwrap();
    // assert!(!balance_one.is_empty());
    // assert_eq!(balance_one[0], coin(150, "uandr"));

    // let vault_two_addr: String = router
    //     .wrap()
    //     .query_wasm_smart(
    //         app_addr,
    //         &mock_get_address_msg(vault_two_app_component.name),
    //     )
    //     .unwrap();
    // let balance_two: Vec<Coin> = router
    //     .wrap()
    //     .query_wasm_smart(
    //         vault_two_addr,
    //         &mock_vault_get_balance(
    //             vault_two_recipient_addr.to_string(),
    //             Some("uandr".to_string()),
    //             None,
    //         ),
    //     )
    //     .unwrap();
    // assert!(!balance_two.is_empty());
    // assert_eq!(balance_two[0], coin(150, "uandr"));
}
