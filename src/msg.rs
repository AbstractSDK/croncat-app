use cosmwasm_schema::QueryResponses;
use cosmwasm_std::Addr;
use croncat_integration_utils::CronCatTaskRequest;
use cw_asset::AssetListUnchecked;

use crate::{contract::CroncatApp, state::Config};

// Expose the top-level app messages
abstract_app::app_messages!(CroncatApp, AppExecuteMsg, AppQueryMsg);

/// App instantiate message
#[cosmwasm_schema::cw_serde]
pub struct AppInstantiateMsg {}

/// App execute messages
#[cosmwasm_schema::cw_serde]
#[cfg_attr(feature = "interface", derive(cw_orch::ExecuteFns))]
#[cfg_attr(feature = "interface", impl_into(ExecuteMsg))]
pub enum AppExecuteMsg {
    UpdateConfig {},
    CreateTask {
        task: Box<CronCatTaskRequest>,
        task_tag: String,
        assets: AssetListUnchecked,
    },
    RemoveTask {
        task_tag: String,
    },
    RefillTask {
        task_tag: String,
        assets: AssetListUnchecked,
    },
}

#[cosmwasm_schema::cw_serde]
#[cfg_attr(feature = "interface", derive(cw_orch::QueryFns))]
#[cfg_attr(feature = "interface", impl_into(QueryMsg))]
#[derive(QueryResponses)]
pub enum AppQueryMsg {
    #[returns(ConfigResponse)]
    Config {},
    #[returns(Vec<(Addr, String)>)]
    ActiveTasks {
        start_after: Option<(String, String)>,
        limit: Option<u32>,
    },
    #[returns(croncat_sdk_tasks::types::TaskResponse)]
    TaskInfo {
        creator_addr: String,
        task_tag: String,
    },
    #[returns(croncat_sdk_manager::types::TaskBalanceResponse)]
    TaskBalance {
        creator_addr: String,
        task_tag: String,
    },
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
