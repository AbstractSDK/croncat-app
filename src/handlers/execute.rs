use abstract_sdk::features::{AbstractResponse, AccountIdentification};
use abstract_sdk::{AccountAction, Execution};
use cosmwasm_std::{
    to_binary, wasm_execute, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, ReplyOn, Response,
};
use croncat_integration_utils::task_creation::get_latest_croncat_contract;
use croncat_integration_utils::{MANAGER_NAME, TASKS_NAME};
use croncat_sdk_manager::msg::ManagerExecuteMsg;
use croncat_sdk_tasks::msg::{TasksExecuteMsg, TasksQueryMsg};
use croncat_sdk_tasks::types::{TaskRequest, TaskResponse};
use cw20::{Cw20Coin, Cw20ExecuteMsg};

use crate::contract::{
    check_users_balance_nonempty, get_croncat_contract, CroncatApp, CroncatResult,
};

use crate::msg::AppExecuteMsg;
use crate::replies::{TASK_CREATE_REPLY_ID, TASK_REMOVE_REPLY_ID};
use crate::state::{Config, ACTIVE_TASKS, CONFIG, REMOVED_TASK_MANAGER_ADDR};

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
    _env: Env,
    msg_info: MessageInfo,
    app: CroncatApp,
    task_request: Box<TaskRequest>,
    funds: Vec<Coin>,
    cw20_funds: Option<Cw20Coin>,
) -> CroncatResult {
    app.admin.assert_admin(deps, &msg_info.sender)?;

    let config = CONFIG.load(deps.storage)?;
    let executor = app.executor(deps);

    let tasks_addr = get_latest_croncat_contract(
        &deps.querier,
        config.factory_addr.clone(),
        TASKS_NAME.to_owned(),
    )?;
    let create_task_msg: CosmosMsg = wasm_execute(
        tasks_addr,
        &TasksExecuteMsg::CreateTask { task: task_request },
        funds,
    )?
    .into();
    let create_task_submessage = executor.execute_with_reply(
        vec![create_task_msg.into()],
        ReplyOn::Success,
        TASK_CREATE_REPLY_ID,
    )?;

    let messages = if let Some(cw20) = cw20_funds {
        let manager_addr = get_latest_croncat_contract(
            &deps.querier,
            config.factory_addr,
            MANAGER_NAME.to_owned(),
        )?;
        let cw20_transfer: CosmosMsg = wasm_execute(
            cw20.address,
            &Cw20ExecuteMsg::Send {
                contract: manager_addr.to_string(),
                amount: cw20.amount,
                msg: to_binary(&croncat_sdk_manager::msg::ManagerReceiveMsg::RefillTempBalance {})?,
            },
            vec![],
        )?
        .into();
        vec![executor.execute(vec![cw20_transfer.into()])?]
    } else {
        vec![]
    };

    let response = Response::default()
        .add_messages(messages)
        .add_submessage(create_task_submessage);
    Ok(app.tag_response(response, "create_task"))
}

/// Remove a task
fn remove_task(
    deps: DepsMut,
    _env: Env,
    msg_info: MessageInfo,
    app: CroncatApp,
    task_hash: String,
) -> CroncatResult {
    app.admin.assert_admin(deps.as_ref(), &msg_info.sender)?;

    let config = CONFIG.load(deps.storage)?;
    let task_version = ACTIVE_TASKS.load(deps.storage, &task_hash)?;

    let tasks_addr = get_croncat_contract(
        &deps.querier,
        config.factory_addr.clone(),
        TASKS_NAME,
        &task_version,
    )?;
    let manager_addr = get_croncat_contract(
        &deps.querier,
        config.factory_addr,
        MANAGER_NAME,
        &task_version,
    )?;

    ACTIVE_TASKS.remove(deps.storage, &task_hash);
    let task_response: TaskResponse = deps.querier.query_wasm_smart(
        tasks_addr.to_string(),
        &TasksQueryMsg::Task {
            task_hash: task_hash.to_owned(),
        },
    )?;

    let response = if task_response.task.is_some() {
        let remove_task_msg: CosmosMsg = wasm_execute(
            tasks_addr,
            &TasksExecuteMsg::RemoveTask { task_hash },
            vec![],
        )?
        .into();
        let executor_submessage = app.executor(deps.as_ref()).execute_with_reply(
            vec![remove_task_msg.into()],
            ReplyOn::Success,
            TASK_REMOVE_REPLY_ID,
        )?;
        REMOVED_TASK_MANAGER_ADDR.save(deps.storage, &manager_addr)?;
        Response::new().add_submessage(executor_submessage)
    } else if check_users_balance_nonempty(
        deps.as_ref(),
        app.proxy_address(deps.as_ref())?,
        manager_addr.clone(),
    )? {
        // withdraw locked balance
        let withdraw_msg: CosmosMsg = wasm_execute(
            manager_addr,
            &ManagerExecuteMsg::UserWithdraw { limit: None },
            vec![],
        )?
        .into();
        let executor_message = app
            .executor(deps.as_ref())
            .execute(vec![withdraw_msg.into()])?;
        Response::new().add_message(executor_message)
    } else {
        Response::new()
    };

    Ok(app.tag_response(response, "remove_task"))
}

/// Refill a task
fn refill_task(
    deps: Deps,
    _env: Env,
    msg_info: MessageInfo,
    app: CroncatApp,
    task_hash: String,
    funds: Vec<Coin>,
    cw20_funds: Option<Cw20Coin>,
) -> CroncatResult {
    app.admin.assert_admin(deps, &msg_info.sender)?;

    let config = CONFIG.load(deps.storage)?;
    let task_version = ACTIVE_TASKS.load(deps.storage, &task_hash)?;
    let executor = app.executor(deps);

    let manager_addr = get_croncat_contract(
        &deps.querier,
        config.factory_addr,
        MANAGER_NAME,
        &task_version,
    )?;

    let mut account_action: AccountAction = AccountAction::new();
    if let Some(cw20) = cw20_funds {
        let cw20_transfer: CosmosMsg = wasm_execute(
            cw20.address,
            &Cw20ExecuteMsg::Send {
                contract: manager_addr.to_string(),
                amount: cw20.amount,
                msg: to_binary(
                    &croncat_sdk_manager::msg::ManagerReceiveMsg::RefillTaskBalance {
                        task_hash: task_hash.clone(),
                    },
                )?,
            },
            vec![],
        )?
        .into();
        account_action.merge(cw20_transfer.into());
    };
    if !funds.is_empty() {
        let refill_task_msg: CosmosMsg = wasm_execute(
            manager_addr,
            &ManagerExecuteMsg::RefillTaskBalance { task_hash },
            funds,
        )?
        .into();
        account_action.merge(refill_task_msg.into());
    }
    let msg = executor.execute(vec![account_action])?;

    Ok(app.tag_response(Response::new().add_message(msg), "refill_task"))
}
