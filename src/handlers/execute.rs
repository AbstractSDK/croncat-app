use abstract_sdk::features::AbstractResponse;
use abstract_sdk::{Execution, TransferInterface};
use cosmwasm_std::{
    to_binary, Addr, Coin, Deps, DepsMut, Empty, Env, MessageInfo, Response, SubMsg, WasmMsg,
};
use croncat_integration_utils::task_creation::get_latest_croncat_contract;
use croncat_integration_utils::{MANAGER_NAME, TASKS_NAME};
use croncat_sdk_manager::msg::{ManagerExecuteMsg, ManagerQueryMsg};
use croncat_sdk_tasks::msg::{TasksExecuteMsg, TasksQueryMsg};
use croncat_sdk_tasks::types::{TaskRequest, TaskResponse};
use cw20::{Cw20Coin, Cw20ExecuteMsg};
use cw_asset::{Asset, AssetInfo};

use crate::contract::{CroncatApp, CroncatResult};

use crate::error::AppError;
use crate::msg::AppExecuteMsg;
use crate::replies::TASK_CREATE_REPLY_ID;
use crate::state::{Config, ACTIVE_TASKS, CONFIG, TASKS_WITH_CW20};

pub fn execute_handler(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    app: CroncatApp,
    msg: AppExecuteMsg,
) -> CroncatResult {
    match msg {
        AppExecuteMsg::UpdateConfig { factory_addr } => {
            update_config(deps, info, app, factory_addr)
        }
        AppExecuteMsg::CreateTask {
            task,
            funds,
            cw20_funds,
        } => create_task(deps.as_ref(), env, info, app, task, funds, cw20_funds),
        AppExecuteMsg::RemoveTask { task_hash } => remove_task(deps, env, info, app, task_hash),
        AppExecuteMsg::RefillTask {
            task_hash,
            funds,
            cw20_funds,
        } => refill_task(deps.as_ref(), env, info, app, task_hash, funds, cw20_funds),
        AppExecuteMsg::MoveFunds {} => move_funds(deps.as_ref(), env, info, app),
        AppExecuteMsg::Tick { .. } => todo!(),
    }
}

/// Update the configuration of the app
fn update_config(
    deps: DepsMut,
    msg_info: MessageInfo,
    app: CroncatApp,
    new_factory_addr: String,
) -> CroncatResult {
    // Only the admin should be able to call this
    app.admin.assert_admin(deps.as_ref(), &msg_info.sender)?;

    let factory_addr = deps.api.addr_validate(&new_factory_addr)?;

    CONFIG.save(deps.storage, &Config { factory_addr })?;
    Ok(app.tag_response(Response::default(), "update_config"))
}

/// Create a task
fn create_task(
    deps: Deps,
    env: Env,
    msg_info: MessageInfo,
    app: CroncatApp,
    task_request: Box<TaskRequest>,
    funds: Vec<Coin>,
    cw20_funds: Option<Cw20Coin>,
) -> CroncatResult {
    app.admin.assert_admin(deps, &msg_info.sender)?;

    let config = CONFIG.load(deps.storage)?;

    let tasks_addr = get_latest_croncat_contract(
        &deps.querier,
        config.factory_addr.clone(),
        TASKS_NAME.to_owned(),
    )?;

    // Withdraw funds
    let bank = app.bank(deps);
    let funds_msgs = if let Some(cw20) = cw20_funds {
        let info = AssetInfo::Cw20(deps.api.addr_validate(&cw20.address)?);
        let asset = Asset::new(info, cw20.amount);
        let manager_addr = get_latest_croncat_contract(
            &deps.querier,
            config.factory_addr,
            MANAGER_NAME.to_owned(),
        )?;
        let cw20_transfer = WasmMsg::Execute {
            contract_addr: cw20.address,
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: manager_addr.to_string(),
                amount: cw20.amount,
                msg: to_binary(&croncat_sdk_manager::msg::ManagerReceiveMsg::RefillTempBalance {})?,
            })?,
            funds: vec![],
        };
        let bank_actions = vec![
            bank.withdraw(&env, funds.clone())?,
            bank.withdraw(&env, vec![asset])?,
        ];
        vec![
            app.executor(deps).execute(bank_actions)?,
            cw20_transfer.into(),
        ]
    } else {
        let bank_actions = vec![bank.withdraw(&env, funds.clone())?];
        vec![app.executor(deps).execute(bank_actions)?]
    };

    let create_task_submsg = SubMsg::reply_on_success(
        WasmMsg::Execute {
            contract_addr: tasks_addr.to_string(),
            msg: to_binary(&TasksExecuteMsg::CreateTask { task: task_request })?,
            funds,
        },
        TASK_CREATE_REPLY_ID,
    );

    let response = Response::default()
        .add_messages(funds_msgs)
        .add_submessage(create_task_submsg);
    Ok(app.tag_response(response, "create_task"))
}

fn local_remove_task(
    deps: &mut DepsMut,
    task_version: &str,
    task_hash: &str,
    tasks_addr: &Addr,
) -> Result<TaskResponse, AppError> {
    ACTIVE_TASKS.remove(deps.storage, task_hash);

    let task_response: TaskResponse = deps.querier.query_wasm_smart(
        tasks_addr.to_string(),
        &TasksQueryMsg::Task {
            task_hash: task_hash.to_owned(),
        },
    )?;
    if let Some(task) = &task_response.task {
        if task.amount_for_one_task.cw20.is_some() {
            TASKS_WITH_CW20.save(deps.storage, task_version, &Empty {})?;
        }
    }
    Ok(task_response)
}

/// Remove a task
fn remove_task(
    mut deps: DepsMut,
    env: Env,
    msg_info: MessageInfo,
    app: CroncatApp,
    task_hash: String,
) -> CroncatResult {
    app.admin.assert_admin(deps.as_ref(), &msg_info.sender)?;

    let config = CONFIG.load(deps.storage)?;
    let (task_version, with_cw20) = ACTIVE_TASKS.load(deps.storage, &task_hash)?;

    // TODO: create helper on factory
    let tasks_addr = croncat_factory::state::CONTRACT_ADDRS
        .query(
            &deps.querier,
            config.factory_addr.clone(),
            (
                TASKS_NAME,
                &task_version
                    .split('.')
                    .map(|num| num.parse::<u8>().unwrap())
                    .collect::<Vec<u8>>(),
            ),
        )?
        .unwrap();
    let manager_addr = croncat_factory::state::CONTRACT_ADDRS
        .query(
            &deps.querier,
            config.factory_addr,
            (
                MANAGER_NAME,
                task_version
                    .split('.')
                    .map(|num| num.parse::<u8>().unwrap())
                    .collect::<Vec<u8>>()
                    .as_ref(),
            ),
        )?
        .unwrap();

    let task_response = local_remove_task(&mut deps, &task_version, &task_hash, &tasks_addr)?;

    let response = if task_response.task.is_some() {
        let remove_task_msg = WasmMsg::Execute {
            contract_addr: tasks_addr.into_string(),
            msg: to_binary(&TasksExecuteMsg::RemoveTask { task_hash })?,
            funds: vec![],
        };
        Response::new().add_message(remove_task_msg)
    } else {
        Response::new()
    };

    let response = if with_cw20 && task_response.task.is_some()
        || check_for_cw20_leftovers(deps.as_ref(), env, &manager_addr)?
    {
        let withdraw_cw20_msg = WasmMsg::Execute {
            contract_addr: manager_addr.into_string(),
            msg: to_binary(&ManagerExecuteMsg::UserWithdraw { limit: None })?,
            funds: vec![],
        };
        response.add_message(withdraw_cw20_msg)
    } else {
        response
    };
    Ok(app.tag_response(response, "remove_task"))
}

fn check_for_cw20_leftovers(deps: Deps, env: Env, manager_addr: &Addr) -> Result<bool, AppError> {
    let coins: Vec<cw20::Cw20CoinVerified> = deps.querier.query_wasm_smart(
        manager_addr,
        &ManagerQueryMsg::UsersBalances {
            address: env.contract.address.into_string(),
            from_index: None,
            // One is enough for us
            limit: Some(1),
        },
    )?;
    Ok(!coins.is_empty())
}

/// Refill a task
fn refill_task(
    deps: Deps,
    env: Env,
    msg_info: MessageInfo,
    app: CroncatApp,
    task_hash: String,
    funds: Vec<Coin>,
    cw20_funds: Option<Cw20Coin>,
) -> CroncatResult {
    app.admin.assert_admin(deps, &msg_info.sender)?;

    let config = CONFIG.load(deps.storage)?;
    let (task_version, _with_cw20) = ACTIVE_TASKS.load(deps.storage, &task_hash)?;

    // TODO: create helper on factory
    let manager_addr = croncat_factory::state::CONTRACT_ADDRS
        .query(
            &deps.querier,
            config.factory_addr,
            (
                MANAGER_NAME,
                &task_version
                    .split('.')
                    .map(|num| num.parse::<u8>().unwrap())
                    .collect::<Vec<u8>>(),
            ),
        )?
        .unwrap();
    let bank = app.bank(deps);
    let messages = if let Some(cw20) = cw20_funds {
        let info = AssetInfo::Cw20(deps.api.addr_validate(&cw20.address)?);
        let asset = Asset::new(info, cw20.amount);

        let cw20_transfer = WasmMsg::Execute {
            contract_addr: cw20.address,
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: manager_addr.to_string(),
                amount: cw20.amount,
                msg: to_binary(
                    &croncat_sdk_manager::msg::ManagerReceiveMsg::RefillTaskBalance {
                        task_hash: task_hash.clone(),
                    },
                )?,
            })?,
            funds: vec![],
        };
        let bank_actions = vec![
            bank.withdraw(&env, funds.clone())?,
            bank.withdraw(&env, vec![asset])?,
        ];
        vec![
            app.executor(deps).execute(bank_actions)?,
            cw20_transfer.into(),
        ]
    } else {
        let bank_actions = vec![bank.withdraw(&env, funds.clone())?];
        vec![app.executor(deps).execute(bank_actions)?]
    };

    let response = Response::default().add_messages(messages);
    if funds.is_empty() {
        Ok(app.tag_response(response, "refill_task"))
    } else {
        let refill_task_msg = WasmMsg::Execute {
            contract_addr: manager_addr.to_string(),
            msg: to_binary(&ManagerExecuteMsg::RefillTaskBalance { task_hash })?,
            funds,
        };
        Ok(app.tag_response(response.add_message(refill_task_msg), "refill_task"))
    }
}

/// Move funds
/// Moves funds from module to the account contract
fn move_funds(deps: Deps, env: Env, msg_info: MessageInfo, app: CroncatApp) -> CroncatResult {
    // TODO: do we care if it's called not by admin?
    app.admin.assert_admin(deps, &msg_info.sender)?;

    let funds = deps.querier.query_all_balances(env.contract.address)?;
    let move_funds_msg = app.bank(deps).deposit(funds)?.messages().pop().unwrap();
    Ok(app.tag_response(Response::new().add_message(move_funds_msg), "move_funds"))
}
