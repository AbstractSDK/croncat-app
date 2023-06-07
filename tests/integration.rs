mod common;

use std::cell::RefMut;

use abstract_core::{app::BaseInstantiateMsg, objects::gov_type::GovernanceDetails};
use abstract_interface::{Abstract, AbstractAccount, AppDeployer, VCExecFns};

use app::{
    contract::{CRONCAT_ID, CRONCAT_MODULE_VERSION},
    msg::{AppInstantiateMsg, InstantiateMsg},
    App, AppExecuteMsgFns, AppQueryMsgFns,
};
use common::contracts;

use croncat_integration_utils::{AGENTS_NAME, MANAGER_NAME, TASKS_NAME};
use croncat_sdk_agents::msg::InstantiateMsg as AgentsInstantiateMsg;
use croncat_sdk_factory::msg::{FactoryInstantiateMsg, ModuleInstantiateInfo, VersionKind};
use croncat_sdk_manager::msg::ManagerInstantiateMsg;
use croncat_sdk_tasks::{
    msg::TasksInstantiateMsg,
    types::{Action, TaskRequest},
};

use cw20::{Cw20Coin, Cw20ExecuteMsg};
use cw_multi_test::Executor;
// Use prelude to get all the necessary imports
use cw_orch::{anyhow, deploy::Deploy, prelude::*};

use cosmwasm_std::{coins, to_binary, Addr, BankMsg, Uint128, WasmMsg};
// consts for testing
const ADMIN: &str = "admin";
const VERSION: &str = "1.0";
const DENOM: &str = "abstr";
const PAUSE_ADMIN: &str = "cosmos338dwgj5wm2tuahvfjdldz5s8hmt7l5aznw8jz9s2mmgj5c52jqgfq000";

fn setup_croncat_contracts(
    mut app: RefMut<cw_multi_test::App>,
    proxy_addr: String,
) -> anyhow::Result<(Addr, Addr)> {
    let sender = Addr::unchecked(ADMIN);
    let pause_admin = Addr::unchecked(PAUSE_ADMIN);

    // Instantiate cw20
    let cw20_code_id = app.store_code(contracts::cw20_contract());
    let cw20_addr = app.instantiate_contract(
        cw20_code_id,
        sender.clone(),
        &cw20_base::msg::InstantiateMsg {
            name: "croncatcoins".to_owned(),
            symbol: "ccc".to_owned(),
            decimals: 6,
            initial_balances: vec![Cw20Coin {
                address: proxy_addr,
                amount: Uint128::new(100),
            }],
            mint: None,
            marketing: None,
        },
        &[],
        "cw20-contract".to_owned(),
        None,
    )?;

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
        croncat_tasks_key: (TASKS_NAME.to_owned(), [1, 0]),
        croncat_agents_key: (AGENTS_NAME.to_owned(), [1, 0]),
        pause_admin: pause_admin.clone(),
        gas_price: None,
        treasury_addr: None,
        cw20_whitelist: Some(vec![cw20_addr.to_string()]),
    };
    let module_instantiate_info = ModuleInstantiateInfo {
        code_id,
        version: [1, 0],
        commit_id: "commit1".to_owned(),
        checksum: "checksum123".to_owned(),
        changelog_url: None,
        schema: None,
        msg: to_binary(&msg).unwrap(),
        contract_name: MANAGER_NAME.to_owned(),
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
        croncat_manager_key: (MANAGER_NAME.to_owned(), [1, 0]),
        croncat_tasks_key: (TASKS_NAME.to_owned(), [1, 0]),
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
        contract_name: AGENTS_NAME.to_owned(),
    };
    app.execute_contract(
        sender.clone(),
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
        croncat_manager_key: (MANAGER_NAME.to_owned(), [1, 0]),
        croncat_agents_key: (AGENTS_NAME.to_owned(), [1, 0]),
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
        contract_name: TASKS_NAME.to_owned(),
    };
    app.execute_contract(
        sender,
        factory_addr.to_owned(),
        &croncat_factory::msg::ExecuteMsg::Deploy {
            kind: VersionKind::Tasks,
            module_instantiate_info,
        },
        &[],
    )
    .unwrap();

    Ok((factory_addr, cw20_addr))
}

/// Set up the test environment with the contract installed
fn setup() -> anyhow::Result<(AbstractAccount<Mock>, Abstract<Mock>, App<Mock>, Addr)> {
    // Create a sender
    let sender = Addr::unchecked(ADMIN);
    // Create the mock
    let mock = Mock::new(&sender);

    // Construct the counter interface
    let mut contract = App::new(CRONCAT_ID, mock.clone());
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

    // Instantiating croncat contracts
    mock.set_balance(&sender, coins(100, DENOM))?;
    let (factory_addr, cw20_addr) =
        setup_croncat_contracts(mock.app.as_ref().borrow_mut(), account.proxy.addr_str()?)?;

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
    let manager_addr = account.manager.address()?;
    contract.set_sender(&manager_addr);
    mock.set_balance(&account.proxy.address()?, coins(50_000, DENOM))?;

    Ok((account, abstr_deployment, contract, cw20_addr))
}

#[test]
fn successful_task_creation() -> anyhow::Result<()> {
    // Set up the environment and contract
    let (_account, _abstr, contract, cw20_addr) = setup()?;

    let cw20_amount = Some(Cw20Coin {
        address: cw20_addr.to_string(),
        amount: Uint128::new(100),
    });
    let task = TaskRequest {
        interval: croncat_sdk_tasks::types::Interval::Once,
        boundary: None,
        stop_on_fail: false,
        actions: vec![
            Action {
                msg: BankMsg::Send {
                    to_address: "receiver".to_owned(),
                    amount: coins(1, DENOM),
                }
                .into(),
                gas_limit: None,
            },
            Action {
                msg: WasmMsg::Execute {
                    contract_addr: cw20_addr.to_string(),
                    msg: to_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: "bob".to_owned(),
                        amount: Uint128::new(100),
                    })?,
                    funds: vec![],
                }
                .into(),
                gas_limit: Some(120),
            },
        ],
        queries: None,
        transforms: None,
        cw20: cw20_amount.clone(),
    };

    // TODO: MAKE IT COMPILE LOL
    // let execute_msg = app::msg::AppExecuteMsg::CreateTask {
    //     task: Box::new(task),
    //     funds: coins(45_000, DENOM),
    //     cw20_funds: cw20_amount
    // };
    // account
    //     .manager
    //     .exec_on_module(to_binary(&execute_msg)?, CRONCAT_ID.to_owned())?;

    contract
        .create_task(coins(45_000, DENOM), Box::new(task), cw20_amount)
        .unwrap();

    let active_tasks: Vec<String> = contract.active_tasks()?;
    assert_eq!(active_tasks.len(), 1);

    contract.refill_task(coins(100, DENOM), active_tasks[0].clone(), None)?;

    contract.remove_task(active_tasks[0].clone())?;

    let active_tasks: Vec<String> = contract.active_tasks()?;
    assert_eq!(active_tasks.len(), 0);
    Ok(())
}
