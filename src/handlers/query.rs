use crate::contract::{CroncatApp, CroncatResult};
use crate::msg::{AppQueryMsg, ConfigResponse};
use crate::state::{ACTIVE_TASKS, CONFIG};
use cosmwasm_std::{to_binary, Binary, Deps, Env, StdResult};

pub fn query_handler(
    deps: Deps,
    _env: Env,
    _app: &CroncatApp,
    msg: AppQueryMsg,
) -> CroncatResult<Binary> {
    match msg {
        AppQueryMsg::Config {} => to_binary(&query_config(deps)?),
        AppQueryMsg::ActiveTasks {} => to_binary(&query_active_tasks(deps)?),
    }
    .map_err(Into::into)
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse { config })
}

// TODO: pagination
fn query_active_tasks(deps: Deps) -> StdResult<Vec<String>> {
    ACTIVE_TASKS
        .keys(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .collect()
}
