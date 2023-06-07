use crate::{
    contract::{CroncatApp, CroncatResult},
    error::AppError,
    state::ACTIVE_TASKS,
};

use abstract_sdk::features::AbstractResponse;
use cosmwasm_std::{DepsMut, Env, Reply, Response};
use croncat_integration_utils::reply_handler::reply_handle_croncat_task_creation;

pub fn create_task_reply(deps: DepsMut, _env: Env, app: CroncatApp, reply: Reply) -> CroncatResult {
    let (task, _bin) = reply_handle_croncat_task_creation(reply)?;
    ACTIVE_TASKS.update(
        deps.storage,
        &task.task_hash,
        |task_version| match task_version {
            Some(_) => Err(AppError::TaskAlreadyExists {
                task_hash: task.task_hash.clone(),
            }),
            None => Ok(task.version),
        },
    )?;
    Ok(app.tag_response(
        Response::default()
            // TODO: Or whole TaskExecutionInfo?
            .add_attribute("task_hash", task.task_hash),
        "instantiate_reply",
    ))
}
