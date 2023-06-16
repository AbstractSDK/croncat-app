use crate::{
    contract::{check_users_balance_nonempty, CroncatApp, CroncatResult},
    error::AppError,
    state::{ACTIVE_TASKS, REMOVED_TASK_MANAGER_ADDR},
};

use abstract_sdk::{
    features::{AbstractResponse, AccountIdentification},
    Execution,
};
use cosmwasm_std::{wasm_execute, CosmosMsg, DepsMut, Env, Reply, Response};
use croncat_sdk_manager::msg::ManagerExecuteMsg;

pub fn create_task_reply(deps: DepsMut, _env: Env, app: CroncatApp, reply: Reply) -> CroncatResult {
    // TODO: https://github.com/AbstractSDK/contracts/issues/364
    // let (task, _bin) = reply_handle_croncat_task_creation(reply)?;

    let events = reply.result.unwrap().events;
    let create_task_event = events
        .into_iter()
        .find(|ev| {
            ev.ty == "wasm"
                && ev
                    .attributes
                    .iter()
                    .any(|attr| attr.key == "action" && attr.value == "create_task")
        })
        .unwrap();
    let task_hash = create_task_event
        .attributes
        .iter()
        .find(|&attr| attr.key == "task_hash")
        .unwrap()
        .value
        .clone();
    let task_version = create_task_event
        .attributes
        .into_iter()
        .find(|attr| attr.key == "task_version")
        .unwrap()
        .value;
    ACTIVE_TASKS.update(deps.storage, &task_hash, |ver| match ver {
        Some(_) => Err(AppError::TaskAlreadyExists {
            task_hash: task_hash.clone(),
        }),
        None => Ok(task_version),
    })?;

    Ok(app.tag_response(
        Response::new()
            .set_data(task_hash.as_bytes())
            .add_attribute("task_hash", task_hash),
        "create_task_reply",
    ))
}

pub fn task_remove_reply(
    deps: DepsMut,
    _env: Env,
    app: CroncatApp,
    _reply: Reply,
) -> CroncatResult {
    let manager_addr = REMOVED_TASK_MANAGER_ADDR.load(deps.storage)?;
    let response = if check_users_balance_nonempty(
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
    Ok(app.tag_response(response, "task_remove_reply"))
}
