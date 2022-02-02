use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, BlockInfo, StdResult, Storage};

use crate::{ContractInfoResponse, Expiration, TokenId};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex};

pub struct DummyNftContract<'a> {
    pub contract_info: Item<'a, ContractInfoResponse>,
    pub minter: Item<'a, Addr>,
    pub token_count: Item<'a, u64>,
    pub highest_token_id: Item<'a, TokenId>,
    /// Stored as (granter, operator) giving operator full control over granter's account
    pub operators: Map<'a, (&'a Addr, &'a Addr), Expiration>,
    pub tokens: IndexedMap<'a, TokenId, TokenInfo, TokenIndexes<'a>>,
    pub burned: Map<'a, TokenId, ()>,
}

impl Default for DummyNftContract<'static> {
    fn default() -> Self {
        let indexes = TokenIndexes {
            owner: MultiIndex::new(token_owner_idx, TOKENS_KEY, TOKENS_OWNER_KEY),
        };
        Self {
            contract_info: Item::new(CONTRACT_KEY),
            minter: Item::new(MINTER_KEY),
            token_count: Item::new(TOKEN_COUNT_KEY),
            highest_token_id: Item::new(HIGHEST_TOKEN_ID_KEY),
            operators: Map::new(OPERATOR_KEY),
            tokens: IndexedMap::new(TOKENS_KEY, indexes),
            burned: Map::new(BURNED_KEY),
        }
    }
}

const CONTRACT_KEY: &str = "nft_info";
const MINTER_KEY: &str = "minter";
const TOKEN_COUNT_KEY: &str = "num_tokens";
const HIGHEST_TOKEN_ID_KEY: &str = "highest_token_id";
const OPERATOR_KEY: &str = "operators";
const TOKENS_KEY: &str = "tokens";
const TOKENS_OWNER_KEY: &str = "tokens__owner";
const BURNED_KEY: &str = "burned";

impl<'a> DummyNftContract<'a> {
    pub fn token_count(&self, storage: &dyn Storage) -> StdResult<u64> {
        Ok(self.token_count.may_load(storage)?.unwrap_or_default())
    }

    pub fn increment_tokens(&self, storage: &mut dyn Storage) -> StdResult<u64> {
        let val = self.token_count(storage)? + 1;
        self.token_count.save(storage, &val)?;
        Ok(val)
    }

    pub fn decrement_tokens(&self, storage: &mut dyn Storage) -> StdResult<u64> {
        let val = self.token_count(storage)? - 1;
        self.token_count.save(storage, &val)?;
        Ok(val)
    }

    pub fn update_highest(&self, storage: &mut dyn Storage, token_id: TokenId) -> StdResult<()> {
        let new_highest = match self.highest_token_id.may_load(storage)? {
            Some(old_highest) => old_highest.max(token_id),
            None => token_id,
        };
        self.highest_token_id.save(storage, &new_highest)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TokenInfo {
    /// The owner of the newly minted NFT
    pub owner: Addr,
    /// Approvals are stored here, as we clear them all upon transfer and cannot accumulate much
    pub approvals: Vec<Approval>,

    /// Universal resource identifier for this NFT
    /// Should point to a JSON file that conforms to the ERC721
    /// Metadata JSON Schema
    pub token_uri: Option<String>,

    /// You can add any custom metadata here when you extend cw721-base
    pub extension: Metadata,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Metadata {
    pub image: String,
    pub image_data: Option<String>,
    pub external_url: Option<String>,
    pub description: String,
    pub name: String,
    pub attributes: Vec<Trait>,
    pub background_color: Option<String>,
    pub animation_url: Option<String>,
    pub youtube_url: Option<String>,
}

impl Metadata {
    /// For easier testing, generate a value with dummy fields
    pub fn new_test() -> Self {
        Metadata {
            image: "ipfs://deadbeef".to_owned(),
            image_data: None,
            external_url: None,
            description: "some desc".to_owned(),
            name: "some name".to_owned(),
            attributes: Vec::new(),
            background_color: None,
            animation_url: None,
            youtube_url: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct Trait {
    pub display_type: Option<String>,
    pub trait_type: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Approval {
    /// Account that can transfer/send the token
    pub spender: Addr,
    /// When the Approval expires (maybe Expiration::never)
    pub expires: Expiration,
}

impl Approval {
    pub fn is_expired(&self, block: &BlockInfo) -> bool {
        self.expires.is_expired(block)
    }
}

pub struct TokenIndexes<'a> {
    // pk goes to second tuple element
    pub owner: MultiIndex<'a, (Addr, Vec<u8>), TokenInfo>,
}

impl<'a> IndexList<TokenInfo> for TokenIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<TokenInfo>> + '_> {
        let v: Vec<&dyn Index<TokenInfo>> = vec![&self.owner];
        Box::new(v.into_iter())
    }
}

pub fn token_owner_idx(d: &TokenInfo, k: Vec<u8>) -> (Addr, Vec<u8>) {
    (d.owner.clone(), k)
}
