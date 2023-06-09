mod execute;
mod instantiate;

pub use execute::{create_task_reply, cw20_withdraw_reply};
pub use instantiate::instantiate_reply;

pub const INSTANTIATE_REPLY_ID: u64 = 1u64;

pub const TASK_CREATE_REPLY_ID: u64 = 2u64;

pub const CW20_WITHDRAW_REPLY_ID: u64 = 3u64;
