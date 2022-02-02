use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

use crate::*;

pub(crate) mod execute;
pub(crate) mod lifecycle;
pub(crate) mod query;

#[cfg(test)]
mod tests;

// This makes a conscious choice on the various generics used by the contract
#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let tract = DummyNftContract::default();
    tract.instantiate(deps, env, info, msg)
}

#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> StdResult<Response> {
    let tract = DummyNftContract::default();
    tract.migrate(deps, msg)
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let tract = DummyNftContract::default();
    tract.execute(deps, env, info, msg)
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    let tract = DummyNftContract::default();
    tract.query(deps, env, msg)
}
