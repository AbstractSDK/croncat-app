mod common;

use std::cell::RefMut;

use abstract_core::{app::BaseInstantiateMsg, objects::gov_type::GovernanceDetails};
use abstract_interface::{Abstract, AbstractAccount, AppDeployer, VCExecFns};
use app::{
    contract::{APP_ID, APP_VERSION},
    msg::{AppInstantiateMsg, InstantiateMsg},
    App,
};
use common::contracts;
use croncat_sdk_factory::msg::FactoryInstantiateMsg;
use cw_multi_test::Executor;
// Use prelude to get all the necessary imports
use cw_orch::{anyhow, deploy::Deploy, prelude::*};

use cosmwasm_std::Addr;

// consts for testing
const ADMIN: &str = "admin";

fn setup_croncat_contracts(mut app: RefMut<cw_multi_test::App>) -> anyhow::Result<Addr> {
    let sender = Addr::unchecked(ADMIN);

    let factory_code_id = app.store_code(contracts::croncat_factory_contract());
    let manager_code_id = app.store_code(contracts::croncat_manager_contract());
    let agents_code_id = app.store_code(contracts::croncat_agents_contract());
    let tasks_code_id = app.store_code(contracts::croncat_tasks_contract());

    let factory_addr = app.instantiate_contract(
        factory_code_id,
        sender.clone(),
        &FactoryInstantiateMsg {
            owner_addr: Some(sender.to_string()),
        },
        &[],
        "croncat-factory",
        None,
    )?;
    Ok(factory_addr)
}

/// Set up the test environment with the contract installed
fn setup() -> anyhow::Result<(AbstractAccount<Mock>, Abstract<Mock>)> {
    // Create a sender
    let sender = Addr::unchecked(ADMIN);
    // Create the mock
    let mock = Mock::new(&sender);
    // Instantiating croncat contracts
    let factory_addr = setup_croncat_contracts(mock.app.borrow_mut())?;

    // Construct the counter interface
    let contract = App::new(APP_ID, mock.clone());

    // Deploy Abstract to the mock
    let abstr_deployment = Abstract::deploy_on(mock, "1.0.0".parse().unwrap())?;
    // Create a new account to install the app onto
    let account =
        abstr_deployment
            .account_factory
            .create_default_account(GovernanceDetails::Monarchy {
                monarch: ADMIN.to_string(),
            })?;
    // claim the namespace so app can be deployed
    abstr_deployment
        .version_control
        .claim_namespaces(1, vec!["my-namespace".to_string()])?;

    contract.deploy(APP_VERSION.parse()?)?;
    account.install_module(
        APP_ID,
        &InstantiateMsg {
            base: BaseInstantiateMsg {
                ans_host_address: abstr_deployment.ans_host.addr_str()?,
            },
            module: AppInstantiateMsg { factory_addr },
        },
    )?;
    Ok((account, abstr_deployment))
}

#[test]
fn successful_install() -> anyhow::Result<()> {
    // Set up the environment and contract
    let (account, abstr) = setup()?;

    Ok(())
}
