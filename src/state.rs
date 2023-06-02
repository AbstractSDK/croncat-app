use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

#[cosmwasm_schema::cw_serde]
pub struct Config {
    pub factory_addr: Addr,
}

pub const CONFIG: Item<Config> = Item::new("config");
// Map: `tasks_addr`:[`task_hash`]
pub const ACTIVE_TASKS: Map<Addr, Vec<Vec<u8>>> = Map::new("active_tasks");
