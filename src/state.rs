use cosmwasm_std::Addr;
use cw_storage_plus::Item;

#[cosmwasm_schema::cw_serde]
pub struct Config {
    pub factory_addr: Addr
}

pub const CONFIG: Item<Config> = Item::new("config");
