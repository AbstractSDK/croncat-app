use abstract_core::app;
use abstract_sdk::base::{ExecuteEndpoint, InstantiateEndpoint, MigrateEndpoint, QueryEndpoint};
use cosmwasm_schema::QueryResponses;
use cosmwasm_std::Coin;
use croncat_sdk_tasks::types::TaskRequest;
use cw20::Cw20Coin;

use crate::{contract::CroncatApp, state::Config};

/// Abstract App instantiate msg
pub type InstantiateMsg = <CroncatApp as InstantiateEndpoint>::InstantiateMsg;
pub type ExecuteMsg = <CroncatApp as ExecuteEndpoint>::ExecuteMsg;
pub type QueryMsg = <CroncatApp as QueryEndpoint>::QueryMsg;
pub type MigrateMsg = <CroncatApp as MigrateEndpoint>::MigrateMsg;

impl app::AppExecuteMsg for AppExecuteMsg {}
impl app::AppQueryMsg for AppQueryMsg {}

/// App instantiate message
#[cosmwasm_schema::cw_serde]
pub struct AppInstantiateMsg {
    pub factory_addr: String,
}

/// App execute messages
#[cosmwasm_schema::cw_serde]
#[cfg_attr(feature = "interface", derive(cw_orch::ExecuteFns))]
#[cfg_attr(feature = "interface", impl_into(ExecuteMsg))]
pub enum AppExecuteMsg {
    UpdateConfig {
        factory_addr: String,
    },
    CreateTask {
        task: Box<TaskRequest>,
        funds: Vec<Coin>,
        cw20_funds: Option<Cw20Coin>,
    },
    RemoveTask {
        task_hash: String,
    }
}

#[cosmwasm_schema::cw_serde]
#[cfg_attr(feature = "interface", derive(cw_orch::QueryFns))]
#[cfg_attr(feature = "interface", impl_into(QueryMsg))]
#[derive(QueryResponses)]
pub enum AppQueryMsg {
    #[returns(ConfigResponse)]
    Config {},
    #[returns(Vec<String>)]
    ActiveTasks {},
}

#[cosmwasm_schema::cw_serde]
pub enum AppMigrateMsg {}

#[cosmwasm_schema::cw_serde]
pub enum Cw20HookMsg {
    Deposit {},
}

#[cosmwasm_schema::cw_serde]
pub struct ConfigResponse {
    pub config: Config,
}
