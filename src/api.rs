use abstract_core::objects::module::ModuleId;
use abstract_sdk::ModuleInterface;
use abstract_sdk::{
    features::{AccountIdentification, Dependencies},
    AbstractSdkError, AbstractSdkResult,
};
use cosmwasm_std::{to_binary, wasm_execute, Coin, CosmosMsg, Deps};
use croncat_integration_utils::{
    task_creation::{get_croncat_contract, get_latest_croncat_contract},
    MANAGER_NAME,
};
use croncat_integration_utils::{CronCatTaskRequest, TASKS_NAME};
use croncat_sdk_manager::msg::ManagerExecuteMsg;
use croncat_sdk_tasks::{
    msg::{TasksExecuteMsg, TasksQueryMsg},
    types::TaskResponse,
};
use cw20::Cw20ExecuteMsg;
use cw_asset::AssetListUnchecked;

use crate::contract::{sort_funds, CRONCAT_ID};

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
    /// Create task message
    /// On success it will return [`croncat_integration_utils::CronCatTaskExecutionInfo`] in reply data
    /// so you can save `task_hash` and `version` or any other useful information
    pub fn create_task(
        &self,
        task: CronCatTaskRequest,
        funds: Vec<Coin>,
    ) -> AbstractSdkResult<CosmosMsg> {
        let modules = self.base.modules(self.deps);

        let croncat_factory_address = modules.module_address(CRON_CAT_FACTORY)?;
        let tasks_addr = get_latest_croncat_contract(
            &self.deps.querier,
            croncat_factory_address,
            TASKS_NAME.to_string(),
        )
        .map_err(|e| AbstractSdkError::generic_err(e.to_string()))?;
        Ok(wasm_execute(
            tasks_addr,
            &TasksExecuteMsg::CreateTask {
                task: Box::new(task),
            },
            funds,
        )?
        .into())
    }

    /// Refill a task's balance messages
    pub fn refill_task(
        &self,
        task_hash: String,
        task_version: String,
        assets: AssetListUnchecked,
    ) -> AbstractSdkResult<Vec<CosmosMsg>> {
        let modules = self.base.modules(self.deps);
        let querier = &self.deps.querier;
        let (funds, cw20s) = sort_funds(self.deps, assets)?;
        let mut messages = vec![];

        let croncat_factory_address = modules.module_address(self.module_id)?;
        let manager_addr = get_croncat_contract(
            querier,
            croncat_factory_address,
            MANAGER_NAME.to_string(),
            task_version,
        )
        .map_err(|e| AbstractSdkError::generic_err(e.to_string()))?;

        // Note: It will be up to one cw20 for now
        // but maybe later we can support multiple if we could confirm some "recipe" won't lock agents gas limit
        for cw20 in cw20s {
            let cw20_transfer = wasm_execute(
                cw20.address,
                &Cw20ExecuteMsg::Send {
                    contract: manager_addr.to_string(),
                    amount: cw20.amount,
                    msg: to_binary(
                        &croncat_sdk_manager::msg::ManagerReceiveMsg::RefillTaskBalance {
                            task_hash: task_hash.clone(),
                        },
                    )?,
                },
                vec![],
            )?;
            messages.push(cw20_transfer.into());
        }
        if !funds.is_empty() {
            let refill_msg = wasm_execute(
                manager_addr,
                &ManagerExecuteMsg::RefillTaskBalance {
                    task_hash: task_hash.clone(),
                },
                funds,
            )?;
            messages.push(refill_msg.into());
        }
        Ok(messages)
    }
}

impl<'a, T: CronCatInterface> CronCat<'a, T> {
    /// task_information
    // TODO: should we provide here task's balance or make another method,
    // NOTE: we would have to make another query
    pub fn query_task_information(
        &self,
        task_hash: String,
        task_version: String,
    ) -> AbstractSdkResult<TaskResponse> {
        let modules = self.base.modules(self.deps);
        let querier = &self.deps.querier;

        let croncat_factory_address = modules.module_address(self.module_id)?;
        let tasks_addr = get_croncat_contract(
            querier,
            croncat_factory_address,
            TASKS_NAME.to_string(),
            task_version,
        )
        .map_err(|e| AbstractSdkError::generic_err(e.to_string()))?;
        let task_response =
            querier.query_wasm_smart(tasks_addr, &TasksQueryMsg::Task { task_hash })?;
        Ok(task_response)
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
