use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};

use crate::contract::{CroncatApp, CroncatResult};
use crate::msg::AppInstantiateMsg;
use crate::state::{Config, CONFIG};

pub fn instantiate_handler(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _app: CroncatApp,
    msg: AppInstantiateMsg,
) -> CroncatResult {
    let factory_addr = deps.api.addr_validate(&msg.factory_addr)?;
    let config: Config = Config { factory_addr };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new())
}
