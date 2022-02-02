//! Instantiating and migrating the contract.
use cosmwasm_std::{DepsMut, Empty, Env, MessageInfo, Response, StdError, StdResult};

use crate::{ContractInfoResponse, InstantiateMsg, MigrateMsg};
use cw2::{get_contract_version, set_contract_version};

use crate::types::state::DummyNftContract;

// version info for migration info
const CONTRACT_NAME: &str = "dummy.finance/nfts";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

impl<'a> DummyNftContract<'a> {
    pub fn instantiate(
        &self,
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        msg: InstantiateMsg,
    ) -> StdResult<Response<Empty>> {
        set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

        let info = ContractInfoResponse {
            name: msg.name,
            symbol: msg.symbol,
        };
        self.contract_info.save(deps.storage, &info)?;
        let minter = deps.api.addr_validate(&msg.minter)?;
        self.minter.save(deps.storage, &minter)?;
        Ok(Response::default())
    }

    pub fn migrate(&self, deps: DepsMut, msg: MigrateMsg) -> StdResult<Response<Empty>> {
        let version = get_contract_version(deps.storage)?;
        if version.contract != CONTRACT_NAME {
            return Err(StdError::generic_err("Can only upgrade from same type"));
        }

        // Validate the minter first
        let minter = match &msg.minter {
            None => None,
            Some(minter) => Some(deps.api.addr_validate(minter)?),
        };

        cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
        let mut info = self.contract_info(deps.as_ref())?;
        if let Some(name) = msg.name {
            info.name = name;
        }
        if let Some(symbol) = msg.symbol {
            info.symbol = symbol;
        }
        self.contract_info.save(deps.storage, &info)?;

        if let Some(minter) = minter {
            self.minter.save(deps.storage, &minter)?;
        }
        Ok(Response::default())
    }
}
