use crate::{
    contract::{CroncatApp, CroncatResult},
    error::AppError,
    state::{ACTIVE_TASKS, CW20_TO_TRANSFER},
};

use abstract_sdk::features::AbstractResponse;
use cosmwasm_std::{DepsMut, Env, Event, Reply, Response};
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
            None => Ok((task.version, task.amount_for_one_task.cw20.is_some())),
        },
    )?;

    Ok(app.tag_response(
        Response::new().add_attribute("task_hash", task.task_hash),
        "create_task_reply",
    ))
}

pub fn cw20_withdraw_reply(
    deps: DepsMut,
    _env: Env,
    app: CroncatApp,
    reply: Reply,
) -> CroncatResult {
    let res = reply.result.unwrap();

    for Event { ty, attributes, .. } in res.events {
        if ty == "wasm"
            && attributes
                .iter()
                .any(|attr| attr.key == "action" && attr.value == "transfer")
        {
            let addr = attributes
                .iter()
                .find(|&attr| attr.key == "_contract_addr")
                .unwrap()
                .value
                .as_ref();
            let amount = attributes
                .iter()
                .find(|&attr| attr.key == "amount")
                .unwrap()
                .value
                .parse::<cosmwasm_std::Uint128>()
                .unwrap();
            CW20_TO_TRANSFER.update(deps.storage, addr, |am| {
                CroncatResult::Ok(am.unwrap_or_default() + amount)
            })?;
        }
    }
    Ok(app.tag_response(Response::new(), "cw20_withdraw_reply"))
}
