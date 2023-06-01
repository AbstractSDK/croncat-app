use abstract_sdk::features::AbstractResponse;
use cosmwasm_std::{to_binary, DepsMut, Env, MessageInfo, Response, WasmMsg};
use croncat_sdk_factory::msg::FactoryQueryMsg;
use croncat_sdk_tasks::msg::TasksExecuteMsg;
use croncat_sdk_tasks::types::TaskRequest;

use crate::contract::{CroncatApp, CroncatResult};

use crate::error::AppError;
use crate::msg::AppExecuteMsg;
use crate::state::{Config, CONFIG};

pub fn execute_handler(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    app: CroncatApp,
    msg: AppExecuteMsg,
) -> CroncatResult {
    match msg {
        AppExecuteMsg::UpdateConfig { factory_addr } => {
            update_config(deps, info, app, factory_addr)
        }
        AppExecuteMsg::CreateTask { task } => create_task(deps, info, app, task),
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
    deps: DepsMut,
    msg_info: MessageInfo,
    app: CroncatApp,
    task_request: Box<TaskRequest>,
) -> CroncatResult {
    app.admin.assert_admin(deps.as_ref(), &msg_info.sender)?;
    let config = CONFIG.load(deps.storage)?;

    let metadata_res: croncat_sdk_factory::msg::ContractMetadataResponse =
        deps.querier.query_wasm_smart(
            &config.factory_addr,
            &FactoryQueryMsg::LatestContract {
                contract_name: "tasks".to_owned(),
            },
        )?;
    let tasks_addr = metadata_res
        .metadata
        .ok_or(AppError::UnknownVersion {})?
        .contract_addr;
    let response = Response::default().add_message(WasmMsg::Execute {
        contract_addr: tasks_addr.to_string(),
        msg: to_binary(&TasksExecuteMsg::CreateTask { task: task_request })?,
        // TODO: take funds from manager?
        funds: msg_info.funds,
    });
    // TODO: parse reply
    Ok(app.tag_response(response, "create_task"))
}
