use cosmwasm_std::{Addr, Empty};
use cw_storage_plus::{Item, Map};

#[cosmwasm_schema::cw_serde]
pub struct Config {
    pub factory_addr: Addr,
}

pub const CONFIG: Item<Config> = Item::new("config");

/// Map: `task_hash`: `task_version`
pub const ACTIVE_TASKS: Map<&str, String> = Map::new("active_tasks");

/// Map to track how many tasks on each contract`(version, task_hash)`
// pub const ACTIVE_VERSIONS: Map<&str, Vec<String>> = Map::new("active_versions");

/// Contracts that can still hold some locked cw20's
pub const TASKS_WITH_CW20: Map<&str, Empty> = Map::new("tasks_with_cw20");
