use crate::contract::{CroncatApp, CroncatResult};
use crate::msg::{AppQueryMsg, ConfigResponse};
use crate::state::{ACTIVE_TASKS, CONFIG};
use crate::utils::factory_addr;
use abstract_sdk::features::AbstractNameService;
use cosmwasm_std::{to_binary, Addr, Binary, Deps, Env, StdResult};
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
    app: &CroncatApp,
    msg: AppQueryMsg,
) -> CroncatResult<Binary> {
    match msg {
        AppQueryMsg::Config {} => to_binary(&query_config(deps)?),
        AppQueryMsg::ActiveTasks { start_after, limit } => {
            to_binary(&query_active_tasks(deps, start_after, limit)?)
        }
        AppQueryMsg::TaskInfo {
            creator_addr,
            task_tag,
        } => to_binary(&query_task_info(deps, app, creator_addr, task_tag)?),
        AppQueryMsg::TaskBalance {
            creator_addr,
            task_tag,
        } => to_binary(&query_task_balance(deps, app, creator_addr, task_tag)?),
    }
    .map_err(Into::into)
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse { config })
}

fn query_active_tasks(
    deps: Deps,
    start_after: Option<(String, String)>,
    limit: Option<u32>,
) -> StdResult<Vec<(Addr, String)>> {
    let start_after = match start_after {
        Some((addr, tag)) => Some((deps.api.addr_validate(&addr)?, tag)),
        None => None,
    };
    let keys = ACTIVE_TASKS.keys(
        deps.storage,
        start_after.map(Bound::exclusive),
        None,
        cosmwasm_std::Order::Ascending,
    );
    match limit {
        Some(limit) => keys.take(limit as usize).collect(),
        None => keys.collect(),
    }
}

fn query_task_info(
    deps: Deps,
    app: &CroncatApp,
    creator_addr: String,
    task_tag: String,
) -> CroncatResult<TaskResponse> {
    let creator_addr = deps.api.addr_validate(&creator_addr)?;
    let (task_hash, task_version) = ACTIVE_TASKS.load(deps.storage, (creator_addr, task_tag))?;

    let factory_addr = factory_addr(&deps.querier, &app.ans_host(deps)?)?;
    let tasks_addr = get_croncat_contract(
        &deps.querier,
        factory_addr,
        TASKS_NAME.to_owned(),
        task_version,
    )
    .unwrap();

    let task_info: TaskResponse = deps
        .querier
        .query_wasm_smart(tasks_addr, &TasksQueryMsg::Task { task_hash })?;
    Ok(task_info)
}

fn query_task_balance(
    deps: Deps,
    app: &CroncatApp,
    creator_addr: String,
    task_tag: String,
) -> CroncatResult<TaskBalanceResponse> {
    let creator_addr = deps.api.addr_validate(&creator_addr)?;
    let (task_hash, task_version) = ACTIVE_TASKS.load(deps.storage, (creator_addr, task_tag))?;

    let factory_addr = factory_addr(&deps.querier, &app.ans_host(deps)?)?;
    let manager_addr = get_croncat_contract(
        &deps.querier,
        factory_addr,
        MANAGER_NAME.to_owned(),
        task_version,
    )
    .unwrap();

    let task_balance: TaskBalanceResponse = deps
        .querier
        .query_wasm_smart(manager_addr, &ManagerQueryMsg::TaskBalance { task_hash })?;
    Ok(task_balance)
}
