use crate::msg::AppMigrateMsg;
use crate::replies::{TASK_CREATE_REPLY_ID, TASK_REMOVE_REPLY_ID};
use crate::CRON_CAT_FACTORY;
use crate::{
    error::AppError,
    handlers,
    msg::{AppExecuteMsg, AppInstantiateMsg, AppQueryMsg},
    replies::{self, INSTANTIATE_REPLY_ID},
};
use abstract_app::AppContract;
use cosmwasm_std::Response;

/// The version of your app
pub const CRONCAT_MODULE_VERSION: &str = env!("CARGO_PKG_VERSION");
/// The id of the app
pub const CRONCAT_ID: &str = "croncat:app";

/// The type of the result returned by your app's entry points.
pub type CroncatResult<T = Response> = Result<T, AppError>;

/// The type of the app that is used to build your app and access the Abstract SDK features.
pub type CroncatApp =
    AppContract<AppError, AppInstantiateMsg, AppExecuteMsg, AppQueryMsg, AppMigrateMsg>;

const CRONCAT_APP: CroncatApp = CroncatApp::new(CRONCAT_ID, CRONCAT_MODULE_VERSION, None)
    .with_instantiate(handlers::instantiate_handler)
    .with_execute(handlers::execute_handler)
    .with_query(handlers::query_handler)
    .with_migrate(handlers::migrate_handler)
    .with_replies(&[
        (INSTANTIATE_REPLY_ID, replies::instantiate_reply),
        (TASK_CREATE_REPLY_ID, replies::create_task_reply),
        (TASK_REMOVE_REPLY_ID, replies::task_remove_reply),
    ]);

// Export handlers
#[cfg(feature = "export")]
abstract_app::export_endpoints!(CRONCAT_APP, CroncatApp);

// Small helpers
// TODO: should be able to move those somewhere
pub(crate) fn check_users_balance_nonempty(
    deps: cosmwasm_std::Deps,
    proxy_addr: cosmwasm_std::Addr,
    manager_addr: cosmwasm_std::Addr,
) -> Result<bool, AppError> {
    let coins: Vec<cw20::Cw20CoinVerified> = deps.querier.query_wasm_smart(
        manager_addr,
        &croncat_sdk_manager::msg::ManagerQueryMsg::UsersBalances {
            address: proxy_addr.into_string(),
            from_index: None,
            // One is enough to know
            limit: Some(1),
        },
    )?;
    Ok(!coins.is_empty())
}

pub(crate) fn sort_funds(
    deps: cosmwasm_std::Deps,
    assets: cw_asset::AssetListUnchecked,
) -> Result<(Vec<cosmwasm_std::Coin>, Vec<cw20::Cw20CoinVerified>), cw_asset::AssetError> {
    let assets = assets.check(deps.api, None)?;
    let (funds, cw20s) =
        assets
            .into_iter()
            .fold((vec![], vec![]), |(mut funds, mut cw20s), asset| {
                match &asset.info {
                    cw_asset::AssetInfoBase::Native(denom) => {
                        funds.push(cosmwasm_std::coin(asset.amount.u128(), denom))
                    }
                    cw_asset::AssetInfoBase::Cw20(address) => cw20s.push(cw20::Cw20CoinVerified {
                        address: address.clone(),
                        amount: asset.amount,
                    }),
                    _ => todo!(),
                }
                (funds, cw20s)
            });
    Ok((funds, cw20s))
}

pub(crate) fn factory_addr(
    querier: &cosmwasm_std::QuerierWrapper,
    ans_host: &abstract_sdk::feature_objects::AnsHost,
) -> Result<cosmwasm_std::Addr, crate::error::AppError> {
    let factory_entry =
        abstract_core::objects::UncheckedContractEntry::try_from(CRON_CAT_FACTORY.to_owned())?
            .into();
    let factory_addr = ans_host.query_contract(querier, &factory_entry)?;
    Ok(factory_addr)
}
