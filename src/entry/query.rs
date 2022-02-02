use cosmwasm_std::{to_binary, Binary, BlockInfo, Deps, Env, Order, Pair, StdResult};
use cw0::maybe_addr;

use crate::{
    AllNftInfoResponse, ApprovedForAllResponse, ContractInfoResponse, Expiration,
    HighestTokenIdResponse, MinterResponse, NftInfoResponse, NumTokensResponse, OwnerOfResponse,
    QueryMsg, TokenId, TokensResponse,
};
use cw_storage_plus::Bound;

use crate::types::state::{Approval, DummyNftContract, TokenInfo};

const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 30;

impl<'a> DummyNftContract<'a> {
    pub fn contract_info(&self, deps: Deps) -> StdResult<ContractInfoResponse> {
        self.contract_info.load(deps.storage)
    }

    pub fn num_tokens(&self, deps: Deps) -> StdResult<NumTokensResponse> {
        let count = self.token_count(deps.storage)?;
        Ok(NumTokensResponse { count })
    }

    pub fn highest_token_id(&self, deps: Deps) -> StdResult<HighestTokenIdResponse> {
        self.highest_token_id
            .may_load(deps.storage)
            .map(|highest_token_id| HighestTokenIdResponse { highest_token_id })
    }

    pub fn nft_info(&self, deps: Deps, token_id: TokenId) -> StdResult<NftInfoResponse> {
        let info = self.tokens.load(deps.storage, token_id)?;
        Ok(NftInfoResponse {
            token_uri: info.token_uri,
            extension: info.extension,
        })
    }

    pub fn owner_of(
        &self,
        deps: Deps,
        env: Env,
        token_id: TokenId,
        include_expired: bool,
    ) -> StdResult<OwnerOfResponse> {
        let info = self.tokens.load(deps.storage, token_id)?;
        Ok(OwnerOfResponse {
            owner: info.owner.to_string(),
            approvals: humanize_approvals(&env.block, &info, include_expired),
        })
    }

    pub fn all_approvals(
        &self,
        deps: Deps,
        env: Env,
        owner: String,
        include_expired: bool,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> StdResult<ApprovedForAllResponse> {
        let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
        let start_addr = maybe_addr(deps.api, start_after)?;
        let start = start_addr.map(|addr| Bound::exclusive(addr.as_ref()));

        let owner_addr = deps.api.addr_validate(&owner)?;
        let res: StdResult<Vec<_>> = self
            .operators
            .prefix(&owner_addr)
            .range(deps.storage, start, None, Order::Ascending)
            .filter(|r| {
                include_expired || r.is_err() || !r.as_ref().unwrap().1.is_expired(&env.block)
            })
            .take(limit)
            .map(parse_approval)
            .collect();
        Ok(ApprovedForAllResponse { operators: res? })
    }

    pub fn tokens(
        &self,
        deps: Deps,
        owner: String,
        start_after: Option<TokenId>,
        limit: Option<u32>,
    ) -> StdResult<TokensResponse> {
        let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
        let start = start_after.map(|token_id| Bound::exclusive(token_id.to_bytes()));

        let owner_addr = deps.api.addr_validate(&owner)?;
        let pks: Vec<_> = self
            .tokens
            .idx
            .owner
            .prefix(owner_addr)
            .keys(deps.storage, start, None, Order::Ascending)
            .take(limit)
            .collect();

        let tokens: Result<Vec<_>, _> = pks.iter().map(|v| TokenId::from_bytes(v)).collect();
        Ok(TokensResponse { tokens: tokens? })
    }

    pub fn all_tokens(
        &self,
        deps: Deps,
        start_after: Option<TokenId>,
        limit: Option<u32>,
    ) -> StdResult<TokensResponse> {
        let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
        let start = start_after.map(|token_id| Bound::exclusive(token_id.to_bytes()));

        let tokens: StdResult<Vec<TokenId>> = self
            .tokens
            .range(deps.storage, start, None, Order::Ascending)
            .take(limit)
            .map(|item| item.and_then(|(k, _)| TokenId::from_bytes(&k)))
            .collect();
        Ok(TokensResponse { tokens: tokens? })
    }

    pub fn all_nft_info(
        &self,
        deps: Deps,
        env: Env,
        token_id: TokenId,
        include_expired: bool,
    ) -> StdResult<AllNftInfoResponse> {
        let info = self.tokens.load(deps.storage, token_id)?;
        Ok(AllNftInfoResponse {
            access: OwnerOfResponse {
                owner: info.owner.to_string(),
                approvals: humanize_approvals(&env.block, &info, include_expired),
            },
            info: NftInfoResponse {
                token_uri: info.token_uri,
                extension: info.extension,
            },
        })
    }
}

impl<'a> DummyNftContract<'a> {
    pub fn minter(&self, deps: Deps) -> StdResult<MinterResponse> {
        let minter_addr = self.minter.load(deps.storage)?;
        Ok(MinterResponse {
            minter: minter_addr.to_string(),
        })
    }

    pub fn query(&self, deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
        match msg {
            QueryMsg::Minter {} => to_binary(&self.minter(deps)?),
            QueryMsg::ContractInfo {} => to_binary(&self.contract_info(deps)?),
            QueryMsg::NftInfo { token_id } => to_binary(&self.nft_info(deps, token_id)?),
            QueryMsg::OwnerOf {
                token_id,
                include_expired,
            } => {
                to_binary(&self.owner_of(deps, env, token_id, include_expired.unwrap_or(false))?)
            }
            QueryMsg::AllNftInfo {
                token_id,
                include_expired,
            } => to_binary(&self.all_nft_info(
                deps,
                env,
                token_id,
                include_expired.unwrap_or(false),
            )?),
            QueryMsg::ApprovedForAll {
                owner,
                include_expired,
                start_after,
                limit,
            } => to_binary(&self.all_approvals(
                deps,
                env,
                owner,
                include_expired.unwrap_or(false),
                start_after,
                limit,
            )?),
            QueryMsg::NumTokens {} => to_binary(&self.num_tokens(deps)?),
            QueryMsg::Tokens {
                owner,
                start_after,
                limit,
            } => to_binary(&self.tokens(deps, owner, start_after, limit)?),
            QueryMsg::AllTokens { start_after, limit } => {
                to_binary(&self.all_tokens(deps, start_after, limit)?)
            }
            QueryMsg::HighestTokenId {} => to_binary(&self.highest_token_id(deps)?),
        }
    }
}

fn parse_approval(item: StdResult<Pair<Expiration>>) -> StdResult<crate::Approval> {
    item.and_then(|(k, expires)| {
        let spender = String::from_utf8(k)?;
        Ok(crate::Approval { spender, expires })
    })
}

fn humanize_approvals(
    block: &BlockInfo,
    info: &TokenInfo,
    include_expired: bool,
) -> Vec<crate::Approval> {
    info.approvals
        .iter()
        .filter(|apr| include_expired || !apr.is_expired(block))
        .map(humanize_approval)
        .collect()
}

fn humanize_approval(approval: &Approval) -> crate::Approval {
    crate::Approval {
        spender: approval.spender.to_string(),
        expires: approval.expires,
    }
}
