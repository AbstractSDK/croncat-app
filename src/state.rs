use cosmwasm_std::{Addr, Empty};
use cw_storage_plus::{Item, Map};

#[cosmwasm_schema::cw_serde]
pub struct Config {
    pub factory_addr: Addr,
}

pub const CONFIG: Item<Config> = Item::new("config");

/// Map: `task_hash`: (`task_version`, `with_cw20`)
pub const ACTIVE_TASKS: Map<&str, (String, bool)> = Map::new("active_tasks");

pub const CW20_TO_TRANSFER: Map<&str, cosmwasm_std::Uint128> = Map::new("cw20_to_transfer");

/// Contracts that can still hold some locked cw20's
pub const TASKS_WITH_CW20: Map<&str, Empty> = Map::new("tasks_with_cw20");
