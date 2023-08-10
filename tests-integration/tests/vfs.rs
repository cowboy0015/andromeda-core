use andromeda_app::app::AppComponent;
use andromeda_app_contract::mock::{
    mock_andromeda_app, mock_app_instantiate_msg, mock_claim_ownership_msg, mock_get_address_msg,
    mock_get_components_msg,
};
use andromeda_crowdfund::mock::{
    mock_andromeda_crowdfund, mock_crowdfund_instantiate_msg, mock_crowdfund_quick_mint_msg,
};
use andromeda_cw721::mock::{mock_andromeda_cw721, mock_cw721_instantiate_msg};
use andromeda_std::amp::AndrAddr;
use andromeda_vfs::mock::mock_vfs_add_parent_path;

use andromeda_testing::mock::MockAndromeda;
use cosmwasm_std::{coin, to_binary, Addr};
use cw_multi_test::{App, Executor};

fn mock_app() -> App {
    let owner = Addr::unchecked("owner");

    App::new(|router, _api, storage| {
        router
            .bank
            .init_balance(storage, &owner, [coin(999999, "uandr")].to_vec())
            .unwrap();
    })
}

fn mock_andromeda(app: &mut App, admin_address: Addr) -> MockAndromeda {
    MockAndromeda::new(app, &admin_address)
}

// TODO: THIS TEST WORKS ON CHAIN BUT NOT HERE
#[test]
fn test_vfs_app() {
    let owner = Addr::unchecked("owner");

    let mut router = mock_app();
    let andr = mock_andromeda(&mut router, owner.clone());
    // Store contract codes
    let cw721_code_id = router.store_code(mock_andromeda_cw721());
    let crowdfund_code_id = router.store_code(mock_andromeda_crowdfund());
    let app_code_id = router.store_code(mock_andromeda_app());

    andr.store_code_id(&mut router, "cw721", cw721_code_id);
    andr.store_code_id(&mut router, "crowdfund", crowdfund_code_id);
    andr.store_code_id(&mut router, "app", app_code_id);

    // Generate App Components
    // App component names must be less than 3 characters or longer than 54 characters to force them to be 'invalid' as the MockApi struct used within the CosmWasm App struct only contains those two validation checks

    let crowdfund_init_msg = mock_crowdfund_instantiate_msg(
        AndrAddr::from_string("./2".to_string()),
        false,
        None,
        andr.kernel_address.to_string(),
        None,
    );
    let crowdfund_app_component = AppComponent {
        name: "1".to_string(),
        ado_type: "crowdfund".to_string(),
        instantiate_msg: to_binary(&crowdfund_init_msg).unwrap(),
    };

    let cw721_init_msg = mock_cw721_instantiate_msg(
        "Test Tokens".to_string(),
        "TT".to_string(),
        "./1", // Crowdfund must be minter
        None,
        andr.kernel_address.to_string(),
        None,
    );
    let cw721_component = AppComponent {
        name: "2".to_string(),
        ado_type: "cw721".to_string(),
        instantiate_msg: to_binary(&cw721_init_msg).unwrap(),
    };

    let app_components = vec![cw721_component, crowdfund_app_component.clone()];
    let app_init_msg = mock_app_instantiate_msg(
        "app".to_string(),
        app_components.clone(),
        andr.kernel_address.clone(),
        None,
    );

    let app_addr = router
        .instantiate_contract(
            app_code_id,
            owner.clone(),
            &app_init_msg,
            &[],
            "Crowdfund App",
            Some(owner.to_string()),
        )
        .unwrap();

    let components: Vec<AppComponent> = router
        .wrap()
        .query_wasm_smart(app_addr.clone(), &mock_get_components_msg())
        .unwrap();

    assert_eq!(components, app_components);

    router
        .execute_contract(
            owner.clone(),
            app_addr.clone(),
            &mock_claim_ownership_msg(None),
            &[],
        )
        .unwrap();

    let crowdfund_addr: String = router
        .wrap()
        .query_wasm_smart(
            app_addr,
            &mock_get_address_msg(crowdfund_app_component.name),
        )
        .unwrap();

    // Mint Tokens
    let mint_msg = mock_crowdfund_quick_mint_msg(5, owner.to_string());
    router
        .execute_contract(
            owner.clone(),
            Addr::unchecked(crowdfund_addr),
            &mint_msg,
            &[],
        )
        .unwrap();

    // Try to update minter path /owner/app/1 from unauthorised user
    let unauthorised_user = Addr::unchecked("unauthorized_sender");

    let unauthorised_app_addr = router
        .instantiate_contract(
            app_code_id,
            unauthorised_user,
            &app_init_msg,
            &[],
            "Crowdfund App",
            Some(owner.to_string()),
        )
        .unwrap();

    let vfs_parent_msg = mock_vfs_add_parent_path("app", owner);
    router
        .execute_contract(
            Addr::unchecked(unauthorised_app_addr),
            Addr::unchecked(andr.vfs_address),
            &vfs_parent_msg,
            &[],
        )
        .unwrap_err();
}
