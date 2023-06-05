use abstract_core::objects::AnsAsset;
use abstract_sdk::features::AbstractResponse;
use abstract_sdk::{AdapterInterface, Execution, TransferInterface, Transferable};
use cosmwasm_std::{to_binary, Coin, Deps, DepsMut, Env, MessageInfo, Response, SubMsg, WasmMsg};
use croncat_sdk_factory::msg::FactoryQueryMsg;
use croncat_sdk_tasks::msg::TasksExecuteMsg;
use croncat_sdk_tasks::types::TaskRequest;
use cw20::{Cw20Coin, Cw20CoinVerified, Cw20ExecuteMsg};
use cw_asset::{Asset, AssetBase, AssetInfo};

use crate::contract::{CroncatApp, CroncatResult};

use crate::error::AppError;
use crate::msg::AppExecuteMsg;
use crate::replies::TASK_CREATE_REPLY_ID;
use crate::state::{Config, CONFIG};

pub fn execute_handler(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    app: CroncatApp,
    msg: AppExecuteMsg,
) -> CroncatResult {
    match msg {
        AppExecuteMsg::UpdateConfig { factory_addr } => {
            update_config(deps, info, app, factory_addr)
        }
        AppExecuteMsg::CreateTask {
            task,
            funds,
            cw20_funds,
        } => create_task(deps.as_ref(), env, info, app, task, funds, cw20_funds),
    }
}

/// Update the configuration of the app
fn update_config(
    deps: DepsMut,
    msg_info: MessageInfo,
    app: CroncatApp,
    new_factory_addr: String,
) -> CroncatResult {
    // Only the admin should be able to call this
    app.admin.assert_admin(deps.as_ref(), &msg_info.sender)?;

    let factory_addr = deps.api.addr_validate(&new_factory_addr)?;

    CONFIG.save(deps.storage, &Config { factory_addr })?;
    Ok(app.tag_response(Response::default(), "update_config"))
}

/// Create a task
fn create_task(
    deps: Deps,
    env: Env,
    msg_info: MessageInfo,
    app: CroncatApp,
    task_request: Box<TaskRequest>,
    funds: Vec<Coin>,
    cw20_funds: Option<Cw20Coin>,
) -> CroncatResult {
    app.admin.assert_admin(deps, &msg_info.sender)?;

    let config = CONFIG.load(deps.storage)?;
    let metadata_res: croncat_sdk_factory::msg::ContractMetadataResponse =
        deps.querier.query_wasm_smart(
            &config.factory_addr,
            &FactoryQueryMsg::LatestContract {
                contract_name: "tasks".to_owned(),
            },
        )?;
    let tasks_addr = metadata_res
        .metadata
        .ok_or(AppError::UnknownVersion {})?
        .contract_addr;

    // Withdraw funds
    let bank = app.bank(deps);
    let funds_msgs = if let Some(cw20) = cw20_funds {
        let info = AssetInfo::Cw20(deps.api.addr_validate(&cw20.address)?);
        let asset = Asset::new(info, cw20.amount);
        let cw20_transfer = WasmMsg::Execute {
            contract_addr: cw20.address,
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: tasks_addr.to_string(),
                amount: cw20.amount,
            })?,
            funds: vec![],
        };
        let bank_actions = vec![
            bank.withdraw(&env, funds.clone())?,
            bank.withdraw(&env, vec![asset])?,
        ];
        vec![
            app.executor(deps).execute(bank_actions)?,
            cw20_transfer.into(),
        ]
    } else {
        let bank_actions = vec![bank.withdraw(&env, funds.clone())?];
        vec![app.executor(deps).execute(bank_actions)?]
    };

    let create_task_submsg = SubMsg::reply_on_success(
        WasmMsg::Execute {
            contract_addr: tasks_addr.to_string(),
            msg: to_binary(&TasksExecuteMsg::CreateTask { task: task_request })?,
            funds,
        },
        TASK_CREATE_REPLY_ID,
    );

    let response = Response::default()
        .add_messages(funds_msgs)
        .add_submessage(create_task_submsg);
    Ok(app.tag_response(response, "create_task"))
}
