use crate::contract::{CroncatApp, CroncatResult};
use crate::msg::{AppQueryMsg, ConfigResponse};
use crate::state::{ACTIVE_TASKS, CONFIG};
use cosmwasm_std::{to_binary, Binary, Deps, Env, StdResult};
use croncat_integration_utils::task_creation::get_croncat_contract;
use croncat_integration_utils::{MANAGER_NAME, TASKS_NAME};
use croncat_sdk_manager::msg::ManagerQueryMsg;
use croncat_sdk_manager::types::TaskBalanceResponse;
use croncat_sdk_tasks::msg::TasksQueryMsg;
use croncat_sdk_tasks::types::TaskResponse;
use cw_storage_plus::Bound;

pub fn query_handler(
    deps: Deps,
    _env: Env,
    _app: &CroncatApp,
    msg: AppQueryMsg,
) -> CroncatResult<Binary> {
    match msg {
        AppQueryMsg::Config {} => to_binary(&query_config(deps)?),
        AppQueryMsg::ActiveTasks { start_after, limit } => {
            to_binary(&query_active_tasks(deps, start_after, limit)?)
        }
        AppQueryMsg::TaskInfo { task_hash } => to_binary(&query_task_info(deps, task_hash)?),
        AppQueryMsg::TaskBalance { task_hash } => to_binary(&query_task_balance(deps, task_hash)?),
    }
    .map_err(Into::into)
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse { config })
}

fn query_active_tasks(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<String>> {
    let keys = ACTIVE_TASKS.keys(
        deps.storage,
        start_after.as_deref().map(Bound::exclusive),
        None,
        cosmwasm_std::Order::Ascending,
    );
    match limit {
        Some(limit) => keys.take(limit as usize).collect(),
        None => keys.collect(),
    }
}

fn query_task_info(deps: Deps, task_hash: String) -> StdResult<TaskResponse> {
    let config = CONFIG.load(deps.storage)?;
    let task_version = ACTIVE_TASKS.load(deps.storage, &task_hash)?;
    let tasks_addr = get_croncat_contract(
        &deps.querier,
        config.factory_addr,
        TASKS_NAME.to_owned(),
        task_version,
    )
    .unwrap();

    let task_info: TaskResponse = deps
        .querier
        .query_wasm_smart(tasks_addr, &TasksQueryMsg::Task { task_hash })?;
    Ok(task_info)
}

fn query_task_balance(deps: Deps, task_hash: String) -> StdResult<TaskBalanceResponse> {
    let task_version = ACTIVE_TASKS.load(deps.storage, &task_hash)?;
    let config = CONFIG.load(deps.storage)?;
    let manager_addr = get_croncat_contract(
        &deps.querier,
        config.factory_addr,
        MANAGER_NAME.to_owned(),
        task_version,
    )
    .unwrap();

    let task_balance: TaskBalanceResponse = deps
        .querier
        .query_wasm_smart(manager_addr, &ManagerQueryMsg::TaskBalance { task_hash })?;
    Ok(task_balance)
}
