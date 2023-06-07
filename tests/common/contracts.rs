use cosmwasm_std::Empty;
use cw_multi_test::{Contract, ContractWrapper};

pub(crate) use croncat_integration_testing::contracts::{
    croncat_agents_contract, croncat_factory_contract, croncat_manager_contract,
    croncat_tasks_contract,
};

pub(crate) fn cw20_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    );
    Box::new(contract)
}
