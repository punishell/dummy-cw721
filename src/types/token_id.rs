use std::fmt::Display;

use cosmwasm_std::{StdError, StdResult};
use cw_storage_plus::PrimaryKey;
use schemars::{
    schema::{InstanceType, SchemaObject},
    JsonSchema,
};
use serde::{de::Visitor, Deserialize, Serialize};

/// Use numeric token IDs internally
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TokenId {
    value: u64,
    bytes: [u8; 8],
}

impl TokenId {
    pub fn new(value: u64) -> Self {
        TokenId {
            value,
            bytes: value.to_le_bytes(),
        }
    }
}

impl Display for TokenId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl std::fmt::Debug for TokenId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "TokenId({})", self.value)
    }
}

impl<'a> PrimaryKey<'a> for TokenId {
    type Prefix = ();
    type SubPrefix = ();

    fn key(&self) -> Vec<&[u8]> {
        vec![self.to_bytes()]
    }
}

impl Serialize for TokenId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.value.to_string())
    }
}

impl<'a> Deserialize<'a> for TokenId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        deserializer.deserialize_str(TokenIdVisitor)
    }
}

impl From<TokenId> for String {
    fn from(token_id: TokenId) -> Self {
        token_id.value.to_string()
    }
}

struct TokenIdVisitor;

impl<'a> Visitor<'a> for TokenIdVisitor {
    type Value = TokenId;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("Token ID")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        v.parse::<u64>().map_err(E::custom).map(TokenId::new)
    }
}

impl JsonSchema for TokenId {
    fn schema_name() -> String {
        "TokenId".to_owned()
    }

    fn json_schema(_gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        SchemaObject {
            instance_type: Some(InstanceType::String.into()),
            ..Default::default()
        }
        .into()
    }
}

impl TokenId {
    /// Deserialize from the internal representation
    pub fn from_bytes(bytes: &[u8]) -> StdResult<TokenId> {
        match hydrate_trailing_zeros(bytes) {
            None => Err(StdError::serialize_err(
                "Token ID",
                "Tokens must be exactly 8 bytes",
            )),
            Some(arr) => Ok(TokenId::new(u64::from_le_bytes(arr))),
        }
    }

    /// Serialize to the internal representation
    ///
    /// To save storage space, removes trailing zeroes
    pub fn to_bytes(&self) -> &[u8] {
        strip_trailing_zeros(&self.bytes)
    }
}

fn strip_trailing_zeros(mut slice: &[u8]) -> &[u8] {
    while slice.last() == Some(&0) {
        slice = &slice[..slice.len() - 1];
    }
    slice
}

/// Returns `None` if given a slice with more than 8 values
fn hydrate_trailing_zeros(slice: &[u8]) -> Option<[u8; 8]> {
    if slice.len() > 8 {
        None
    } else {
        let mut ret = [0; 8];
        ret[0..slice.len()].copy_from_slice(slice);
        Some(ret)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::quickcheck;

    quickcheck! {
        fn bytes_round_trip(value: u64) -> bool {
            let token = TokenId::new(value);
            let bytes = token.to_bytes();
            let token2 = TokenId::from_bytes(&bytes).unwrap();
            assert_eq!(token, token2);
            true
        }
    }

    #[test]
    fn strip_handles_empty_list() {
        let expected: &[u8] = &[];
        assert_eq!(expected, strip_trailing_zeros(&[]));
    }

    #[test]
    fn strip_handles_gaps() {
        let expected: &[u8] = &[42, 0, 59];
        assert_eq!(expected, strip_trailing_zeros(&[42, 0, 59, 0, 0]));
    }

    quickcheck! {
        fn strip_hydrate_roundtrip(input: u64) -> bool {
            let input = input.to_le_bytes();
            let stripped = strip_trailing_zeros(&input);
            let hydrated = hydrate_trailing_zeros(&stripped).unwrap();
            assert_eq!(input, hydrated);
            true
        }
    }
}
