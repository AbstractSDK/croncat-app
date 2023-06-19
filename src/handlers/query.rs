use crate::contract::{CroncatApp, CroncatResult};
use crate::msg::{AppQueryMsg, ConfigResponse};
use crate::state::{ACTIVE_TASKS, CONFIG};
use crate::utils::factory_addr;
use abstract_sdk::features::AbstractNameService;
use cosmwasm_std::{to_binary, Addr, Binary, Deps, Env, QuerierWrapper, StdResult};
use croncat_integration_utils::task_creation::get_croncat_contract;
use croncat_integration_utils::{MANAGER_NAME, TASKS_NAME};
use croncat_sdk_manager::msg::ManagerQueryMsg;
use croncat_sdk_manager::types::TaskBalanceResponse;
use croncat_sdk_tasks::msg::TasksQueryMsg;
use croncat_sdk_tasks::types::TaskResponse;
use cw_storage_plus::Bound;

pub const DEFAULT_LIMIT: u32 = 50;

fn check_if_task_exists(
    querier: &QuerierWrapper,
    factory_addr: Addr,
    task_hash: String,
    task_version: String,
) -> bool {
    let manager_addr =
        match get_croncat_contract(querier, factory_addr, MANAGER_NAME.to_owned(), task_version) {
            Ok(addr) => addr,
            Err(_) => return false,
        };
    match croncat_manager::state::TASKS_BALANCES.query(querier, manager_addr, task_hash.as_bytes())
    {
        Ok(Some(_)) => true,
        _ => false,
    }
}

pub fn query_handler(
    deps: Deps,
    _env: Env,
    app: &CroncatApp,
    msg: AppQueryMsg,
) -> CroncatResult<Binary> {
    match msg {
        AppQueryMsg::Config {} => to_binary(&query_config(deps)?),
        AppQueryMsg::ActiveTasks {
            start_after,
            limit,
            checked,
        } => to_binary(&query_active_tasks(deps, app, start_after, limit, checked)?),
        AppQueryMsg::ActiveTasksByCreator {
            creator_addr,
            start_after,
            limit,
            checked,
        } => to_binary(&query_active_tasks_by_creator(
            deps,
            app,
            creator_addr,
            start_after,
            limit,
            checked,
        )?),
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
    app: &CroncatApp,
    start_after: Option<(String, String)>,
    limit: Option<u32>,
    checked: Option<bool>,
) -> CroncatResult<Vec<(Addr, String)>> {
    let check = checked.unwrap_or(false);
    let limit = limit.unwrap_or(DEFAULT_LIMIT) as usize;

    let start_after = match start_after {
        Some((addr, tag)) => Some((deps.api.addr_validate(&addr)?, tag)),
        None => None,
    };
    let iter = ACTIVE_TASKS.range(
        deps.storage,
        start_after.map(Bound::exclusive),
        None,
        cosmwasm_std::Order::Ascending,
    );

    let res: StdResult<Vec<(Addr, String)>> = match check {
        true => {
            let factory_addr = factory_addr(&deps.querier, &app.ans_host(deps)?)?;

            // filter tasks that doesn't exist on croncat contract anymore
            iter.filter(|res| {
                res.as_ref().map_or(true, |(_, (task_hash, version))| {
                    check_if_task_exists(
                        &deps.querier,
                        factory_addr.clone(),
                        task_hash.clone(),
                        version.clone(),
                    )
                })
            })
            .map(|res| res.map(|(k, _)| k))
            .take(limit)
            .collect()
        }
        false => iter.map(|res| res.map(|(k, _)| k)).take(limit).collect(),
    };
    res.map_err(Into::into)
}

fn query_active_tasks_by_creator(
    deps: Deps,
    app: &CroncatApp,
    creator: String,
    start_after: Option<String>,
    limit: Option<u32>,
    checked: Option<bool>,
) -> CroncatResult<Vec<String>> {
    let addr = deps.api.addr_validate(&creator)?;
    let check = checked.unwrap_or(false);
    let limit = limit.unwrap_or(DEFAULT_LIMIT) as usize;

    let iter = ACTIVE_TASKS.prefix(addr).range(
        deps.storage,
        start_after.map(Bound::exclusive),
        None,
        cosmwasm_std::Order::Ascending,
    );

    let res: StdResult<Vec<String>> = match check {
        true => {
            let factory_addr = factory_addr(&deps.querier, &app.ans_host(deps)?)?;

            // filter tasks that doesn't exist on croncat contract anymore
            iter.filter(|res| {
                res.as_ref().map_or(true, |(_, (task_hash, version))| {
                    check_if_task_exists(
                        &deps.querier,
                        factory_addr.clone(),
                        task_hash.clone(),
                        version.clone(),
                    )
                })
            })
            .map(|res| res.map(|(k, _)| k))
            .take(limit)
            .collect()
        }
        false => iter.map(|res| res.map(|(k, _)| k)).take(limit).collect(),
    };
    res.map_err(Into::into)
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
