use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

#[cosmwasm_schema::cw_serde]
pub struct Config {
    pub factory_addr: Addr,
}

pub const CONFIG: Item<Config> = Item::new("config");
// Map: `task_hash`:`task_version`
pub const ACTIVE_TASKS: Map<&str, String> = Map::new("active_tasks");
