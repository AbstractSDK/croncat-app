use abstract_core::objects::module::ModuleId;
use abstract_sdk::{
    features::{AccountIdentification, Dependencies},
    AbstractSdkResult,
};
use abstract_sdk::{AppInterface, ModuleInterface};
use cosmwasm_std::{Addr, CosmosMsg, Deps};
use croncat_integration_utils::CronCatTaskRequest;
use croncat_sdk_manager::types::TaskBalanceResponse;
use croncat_sdk_tasks::types::TaskResponse;
use cw_asset::AssetListUnchecked;

use crate::contract::CRONCAT_ID;
use crate::msg::{AppExecuteMsg, AppQueryMsg};

// Entry for the cron_cat factory address, stored in the ANS
pub const CRON_CAT_FACTORY: &str = "croncat:factory";

// API for Abstract SDK users
/// Interact with the cron_cat adapter in your module.
pub trait CronCatInterface: AccountIdentification + Dependencies {
    /// Construct a new cron_cat interface
    fn cron_cat<'a>(&'a self, deps: Deps<'a>) -> CronCat<Self> {
        CronCat {
            base: self,
            deps,
            module_id: CRONCAT_ID,
        }
    }
}

impl<T: AccountIdentification + Dependencies> CronCatInterface for T {}

#[derive(Clone)]
pub struct CronCat<'a, T: CronCatInterface> {
    base: &'a T,
    module_id: ModuleId<'a>,
    deps: Deps<'a>,
}

impl<'a, T: CronCatInterface> CronCat<'a, T> {
    /// Get address of this module
    pub fn module_address(&self) -> AbstractSdkResult<Addr> {
        self.base.modules(self.deps).module_address(self.module_id)
    }
    /// Create task
    /// On success it will return [`croncat_integration_utils::CronCatTaskExecutionInfo`] in reply data, 
    /// you can save task_hash or any other useful information in dependent module.
    /// This way you can track which tasks were created only by this module
    pub fn create_task(
        &self,
        task: CronCatTaskRequest,
        assets: AssetListUnchecked,
    ) -> AbstractSdkResult<CosmosMsg> {
        self.base.apps(self.deps).request(
            self.module_id,
            AppExecuteMsg::CreateTask {
                task: Box::new(task),
                assets,
            },
        )
    }

    /// Refill a task's balance messages
    pub fn refill_task(
        &self,
        task_hash: String,
        assets: AssetListUnchecked,
    ) -> AbstractSdkResult<CosmosMsg> {
        self.base.apps(self.deps).request(
            self.module_id,
            AppExecuteMsg::RefillTask { task_hash, assets },
        )
    }

    pub fn remove_task(&self, task_hash: String) -> AbstractSdkResult<CosmosMsg> {
        self.base
            .apps(self.deps)
            .request(self.module_id, AppExecuteMsg::RemoveTask { task_hash })
    }
}

impl<'a, T: CronCatInterface> CronCat<'a, T> {
    /// Task information
    pub fn query_task_information(&self, task_hash: String) -> AbstractSdkResult<TaskResponse> {
        self.base
            .apps(self.deps)
            .query(self.module_id, AppQueryMsg::TaskInfo { task_hash })
    }

    /// Task balance
    pub fn query_task_balance(&self, task_hash: String) -> AbstractSdkResult<TaskBalanceResponse> {
        self.base
            .apps(self.deps)
            .query(self.module_id, AppQueryMsg::TaskBalance { task_hash })
    }

    /// Active tasks
    pub fn query_active_tasks(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> AbstractSdkResult<Vec<String>> {
        self.base.apps(self.deps).query(
            self.module_id,
            AppQueryMsg::ActiveTasks { start_after, limit },
        )
    }
}

// #[cfg(test)]
// mod test {
//     use super::*;
//     use crate::msg::ExecuteMsg;
//     use abstract_core::adapter::AdapterRequestMsg;
//     use abstract_sdk::mock_module::MockModule;
//     use cosmwasm_std::testing::mock_dependencies;
//     use cosmwasm_std::wasm_execute;
//     use speculoos::prelude::*;

//     #[test]
//     fn swap_msg() {
//         let mut deps = mock_dependencies();
//         deps.querier = abstract_testing::mock_querier();
//         let stub = MockModule::new();
//         let cron_cat = stub
//             .cron_cat(deps.as_ref(), "junoswap".into())
//             .with_module_id(abstract_testing::prelude::TEST_MODULE_ID);

//         let cron_cat_name = "junoswap".to_string();
//         let offer_asset = OfferAsset::new("juno", 1000u128);
//         let ask_asset = AssetEntry::new("uusd");
//         let max_spread = Some(Decimal::percent(1));
//         let belief_price = Some(Decimal::percent(2));

//         let expected = expected_request_with_test_proxy(CronCatExecuteMsg::Action {
//             cron_cat: cron_cat_name,
//             action: CronCatAction::Swap {
//                 offer_asset: offer_asset.clone(),
//                 ask_asset: ask_asset.clone(),
//                 max_spread,
//                 belief_price,
//             },
//         });

//         let actual = cron_cat.swap(offer_asset, ask_asset, max_spread, belief_price);

//         assert_that!(actual).is_ok();

//         let actual = match actual.unwrap() {
//             CosmosMsg::Wasm(msg) => msg,
//             _ => panic!("expected wasm msg"),
//         };
//         let expected = wasm_execute(
//             abstract_testing::prelude::TEST_MODULE_ADDRESS,
//             &expected,
//             vec![],
//         )
//         .unwrap();

//         assert_that!(actual).is_equal_to(expected);
//     }

//     #[test]
//     fn custom_swap_msg() {
//         let mut deps = mock_dependencies();
//         deps.querier = abstract_testing::mock_querier();
//         let stub = MockModule::new();
//         let cron_cat_name = "astroport".to_string();

//         let cron_cat = stub
//             .cron_cat(deps.as_ref(), cron_cat_name.clone())
//             .with_module_id(abstract_testing::prelude::TEST_MODULE_ID);

//         let offer_assets = vec![OfferAsset::new("juno", 1000u128)];
//         let ask_assets = vec![AskAsset::new("uusd", 1000u128)];
//         let max_spread = Some(Decimal::percent(1));
//         let router = Some(SwapRouter::Custom("custom_router".to_string()));

//         let expected = expected_request_with_test_proxy(CronCatExecuteMsg::Action {
//             cron_cat: cron_cat_name,
//             action: CronCatAction::CustomSwap {
//                 offer_assets: offer_assets.clone(),
//                 ask_assets: ask_assets.clone(),
//                 max_spread,
//                 router: router.clone(),
//             },
//         });

//         let actual = cron_cat.custom_swap(offer_assets, ask_assets, max_spread, router);

//         assert_that!(actual).is_ok();

//         let actual = match actual.unwrap() {
//             CosmosMsg::Wasm(msg) => msg,
//             _ => panic!("expected wasm msg"),
//         };
//         let expected = wasm_execute(
//             abstract_testing::prelude::TEST_MODULE_ADDRESS,
//             &expected,
//             vec![],
//         )
//         .unwrap();

//         assert_that!(actual).is_equal_to(expected);
//     }

//     #[test]
//     fn provide_liquidity_msg() {
//         let mut deps = mock_dependencies();
//         deps.querier = abstract_testing::mock_querier();
//         let stub = MockModule::new();
//         let cron_cat_name = "junoswap".to_string();

//         let cron_cat = stub
//             .cron_cat(deps.as_ref(), cron_cat_name.clone())
//             .with_module_id(abstract_testing::prelude::TEST_MODULE_ID);

//         let assets = vec![OfferAsset::new("taco", 1000u128)];
//         let max_spread = Some(Decimal::percent(1));

//         let expected = expected_request_with_test_proxy(CronCatExecuteMsg::Action {
//             cron_cat: cron_cat_name,
//             action: CronCatAction::ProvideLiquidity {
//                 assets: assets.clone(),
//                 max_spread,
//             },
//         });

//         let actual = cron_cat.provide_liquidity(assets, max_spread);

//         assert_that!(actual).is_ok();

//         let actual = match actual.unwrap() {
//             CosmosMsg::Wasm(msg) => msg,
//             _ => panic!("expected wasm msg"),
//         };
//         let expected = wasm_execute(
//             abstract_testing::prelude::TEST_MODULE_ADDRESS,
//             &expected,
//             vec![],
//         )
//         .unwrap();

//         assert_that!(actual).is_equal_to(expected);
//     }

//     #[test]
//     fn provide_liquidity_symmetric_msg() {
//         let mut deps = mock_dependencies();
//         deps.querier = abstract_testing::mock_querier();
//         let stub = MockModule::new();
//         let cron_cat_name = "junoswap".to_string();

//         let cron_cat = stub
//             .cron_cat(deps.as_ref(), cron_cat_name.clone())
//             .with_module_id(abstract_testing::prelude::TEST_MODULE_ID);

//         let offer = OfferAsset::new("taco", 1000u128);
//         let paired = vec![AssetEntry::new("bell")];
//         let _max_spread = Some(Decimal::percent(1));

//         let expected = expected_request_with_test_proxy(CronCatExecuteMsg::Action {
//             cron_cat: cron_cat_name,
//             action: CronCatAction::ProvideLiquiditySymmetric {
//                 offer_asset: offer.clone(),
//                 paired_assets: paired.clone(),
//             },
//         });

//         let actual = cron_cat.provide_liquidity_symmetric(offer, paired);

//         assert_that!(actual).is_ok();

//         let actual = match actual.unwrap() {
//             CosmosMsg::Wasm(msg) => msg,
//             _ => panic!("expected wasm msg"),
//         };
//         let expected = wasm_execute(
//             abstract_testing::prelude::TEST_MODULE_ADDRESS,
//             &expected,
//             vec![],
//         )
//         .unwrap();

//         assert_that!(actual).is_equal_to(expected);
//     }

//     #[test]
//     fn withdraw_liquidity_msg() {
//         let mut deps = mock_dependencies();
//         deps.querier = abstract_testing::mock_querier();
//         let stub = MockModule::new();
//         let cron_cat_name = "junoswap".to_string();

//         let cron_cat = stub
//             .cron_cat(deps.as_ref(), cron_cat_name.clone())
//             .with_module_id(abstract_testing::prelude::TEST_MODULE_ID);

//         let lp_token = AssetEntry::new("taco");
//         let withdraw_amount: Uint128 = 1000u128.into();

//         let expected = expected_request_with_test_proxy(CronCatExecuteMsg::Action {
//             cron_cat: cron_cat_name,
//             action: CronCatAction::WithdrawLiquidity {
//                 lp_token: lp_token.clone(),
//                 amount: withdraw_amount,
//             },
//         });

//         let actual = cron_cat.withdraw_liquidity(lp_token, withdraw_amount);

//         assert_that!(actual).is_ok();

//         let actual = match actual.unwrap() {
//             CosmosMsg::Wasm(msg) => msg,
//             _ => panic!("expected wasm msg"),
//         };
//         let expected = wasm_execute(
//             abstract_testing::prelude::TEST_MODULE_ADDRESS,
//             &expected,
//             vec![],
//         )
//         .unwrap();

//         assert_that!(actual).is_equal_to(expected);
//     }
// }
