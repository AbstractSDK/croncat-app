use crate::{
    contract::{CroncatApp, CroncatResult},
    error::AppError,
    state::ACTIVE_TASKS,
};

use abstract_sdk::features::AbstractResponse;
use cosmwasm_std::{from_binary, DepsMut, Env, Reply, Response};
use croncat_sdk_tasks::types::TaskExecutionInfo;
use cw_utils::parse_reply_execute_data;

pub fn create_task_reply(deps: DepsMut, _env: Env, app: CroncatApp, reply: Reply) -> CroncatResult {
    let execute_data = parse_reply_execute_data(reply)?;
    let task: TaskExecutionInfo = from_binary(&execute_data.data.unwrap())?;
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
