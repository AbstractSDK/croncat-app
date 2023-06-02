mod common;

use std::{borrow::BorrowMut, cell::RefMut};

use abstract_core::{
    app::{BaseInstantiateMsg, BaseQueryMsg},
    objects::{
        gov_type::GovernanceDetails,
        module::{ModuleInfo, ModuleVersion},
        namespace::Namespace,
    },
};
use abstract_interface::{
    Abstract, AbstractAccount, AppDeployer, ManagerQueryFns, VCExecFns, VCQueryFns,
};
use abstract_sdk::{
    mock_module::{self, MockModule},
    prelude::*,
};
use app::{
    contract::{CRONCAT_ID, CRONCAT_MODULE_VERSION},
    msg::{AppInstantiateMsg, AppQueryMsg, ConfigResponse, InstantiateMsg, QueryMsg},
    state::Config,
    App, AppQueryMsgFns,
};
use common::contracts;
use cosmwasm_schema::serde::{Deserialize, Serialize};
use croncat_sdk_agents::msg::InstantiateMsg as AgentsInstantiateMsg;
use croncat_sdk_factory::msg::{
    FactoryInstantiateMsg, FactoryQueryMsg, ModuleInstantiateInfo, VersionKind,
};
use croncat_sdk_manager::msg::ManagerInstantiateMsg;
use croncat_sdk_tasks::msg::TasksInstantiateMsg;

use cw_multi_test::Executor;
// Use prelude to get all the necessary imports
use cw_orch::{anyhow, deploy::Deploy, prelude::*};

use cosmwasm_std::{coin, testing::mock_dependencies, to_binary, Addr, OwnedDeps, Uint128};
// consts for testing
const ADMIN: &str = "admin";
const VERSION: &str = "1.0";
const DENOM: &str = "abstr";
const PAUSE_ADMIN: &str = "cosmos338dwgj5wm2tuahvfjdldz5s8hmt7l5aznw8jz9s2mmgj5c52jqgfq000";

fn setup_croncat_contracts(mut app: RefMut<cw_multi_test::App>) -> anyhow::Result<Addr> {
    let sender = Addr::unchecked(ADMIN);
    let pause_admin = Addr::unchecked(PAUSE_ADMIN);

    let factory_code_id = app.store_code(contracts::croncat_factory_contract());
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

    // Instantiate manager
    let code_id = app.store_code(contracts::croncat_manager_contract());
    let msg = ManagerInstantiateMsg {
        version: Some("1.0".to_owned()),
        croncat_tasks_key: ("tasks".to_owned(), [1, 0]),
        croncat_agents_key: ("agents".to_owned(), [1, 0]),
        pause_admin: pause_admin.clone(),
        gas_price: None,
        treasury_addr: None,
        cw20_whitelist: None,
    };
    let module_instantiate_info = ModuleInstantiateInfo {
        code_id,
        version: [1, 0],
        commit_id: "commit1".to_owned(),
        checksum: "checksum123".to_owned(),
        changelog_url: None,
        schema: None,
        msg: to_binary(&msg).unwrap(),
        contract_name: "manager".to_owned(),
    };
    app.execute_contract(
        sender.clone(),
        factory_addr.clone(),
        &croncat_factory::msg::ExecuteMsg::Deploy {
            kind: VersionKind::Agents,
            module_instantiate_info,
        },
        &[Coin {
            denom: DENOM.to_owned(),
            amount: Uint128::new(1),
        }],
    )
    .unwrap();

    // Instantiate agents
    let code_id = app.store_code(contracts::croncat_agents_contract());
    let msg = AgentsInstantiateMsg {
        version: Some(VERSION.to_owned()),
        croncat_manager_key: ("manager".to_owned(), [1, 0]),
        croncat_tasks_key: ("tasks".to_owned(), [1, 0]),
        pause_admin: pause_admin.clone(),
        agent_nomination_duration: None,
        min_tasks_per_agent: None,
        min_coins_for_agent_registration: None,
        agents_eject_threshold: None,
        min_active_agent_count: None,
        allowed_agents: Some(vec![]),
        public_registration: true,
    };
    let module_instantiate_info = ModuleInstantiateInfo {
        code_id,
        version: [1, 0],
        commit_id: "commit123".to_owned(),
        checksum: "checksum321".to_owned(),
        changelog_url: None,
        schema: None,
        msg: to_binary(&msg).unwrap(),
        contract_name: "agents".to_owned(),
    };
    app.execute_contract(
        Addr::unchecked(ADMIN),
        factory_addr.to_owned(),
        &croncat_factory::msg::ExecuteMsg::Deploy {
            kind: VersionKind::Agents,
            module_instantiate_info,
        },
        &[],
    )
    .unwrap();

    // Instantiate tasks
    let code_id = app.store_code(contracts::croncat_tasks_contract());
    let msg = TasksInstantiateMsg {
        version: Some(VERSION.to_owned()),
        chain_name: "atom".to_owned(),
        pause_admin,
        croncat_manager_key: ("manager".to_owned(), [1, 0]),
        croncat_agents_key: ("agents".to_owned(), [1, 0]),
        slot_granularity_time: None,
        gas_base_fee: None,
        gas_action_fee: None,
        gas_query_fee: None,
        gas_limit: None,
    };
    let module_instantiate_info = ModuleInstantiateInfo {
        code_id,
        version: [1, 0],
        commit_id: "commit1".to_owned(),
        checksum: "checksum2".to_owned(),
        changelog_url: None,
        schema: None,
        msg: to_binary(&msg).unwrap(),
        contract_name: "tasks".to_owned(),
    };
    app.execute_contract(
        Addr::unchecked(ADMIN),
        factory_addr.to_owned(),
        &croncat_factory::msg::ExecuteMsg::Deploy {
            kind: VersionKind::Tasks,
            module_instantiate_info,
        },
        &[],
    )
    .unwrap();
    Ok(factory_addr)
}

/// Set up the test environment with the contract installed
fn setup() -> anyhow::Result<(AbstractAccount<Mock>, Abstract<Mock>, App<Mock>)> {
    // Create a sender
    let sender = Addr::unchecked(ADMIN);
    // Create the mock
    let mock = Mock::new(&sender);
    mock.set_balance(&sender, vec![coin(100, DENOM)])?;
    // Instantiating croncat contracts
    let factory_addr = setup_croncat_contracts(mock.app.as_ref().borrow_mut())?;

    // Construct the counter interface
    let contract = App::new(CRONCAT_ID, mock.clone());

    // Deploy Abstract to the mock
    let abstr_deployment = Abstract::deploy_on(mock.clone(), Empty {})?;
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
        .claim_namespaces(1, vec!["croncat".to_string()])?;

    contract.deploy(CRONCAT_MODULE_VERSION.parse()?)?;
    account.install_module(
        CRONCAT_ID,
        &InstantiateMsg {
            base: BaseInstantiateMsg {
                ans_host_address: abstr_deployment.ans_host.addr_str()?,
            },
            module: AppInstantiateMsg {
                factory_addr: factory_addr.into_string(),
            },
        },
    )?;

    let module_addr = account.manager.module_info(CRONCAT_ID)?.unwrap().address;
    contract.set_address(&module_addr);

    Ok((account, abstr_deployment, contract))
}

#[test]
fn successful_install() -> anyhow::Result<()> {
    // Set up the environment and contract
    let (account, abstr, contract) = setup()?;
    
    let config: ConfigResponse = contract.config()?;
    println!("{config:?}");
    Ok(())
}
