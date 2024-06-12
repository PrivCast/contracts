use std::collections::HashMap;
use secret_toolkit::storage::Item;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use cosmwasm_std::{Addr, Binary, Timestamp};

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct Poll{
    pub id: u64,
    pub uri: String,
    pub created_at: Timestamp,
    pub validity: u64,  
    pub votes: HashMap<u64, u64>,
    pub has_voted: HashMap<u64, bool>,  
    pub vote_count: u64,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Polls{
    pub polls: Vec<Poll>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Gateway {
    pub gateway_address: Addr,
    pub gateway_hash: String,
    pub gateway_key: Binary,
}

pub static POLL_COUNT: Item<u64> = Item::new(b"poll_count");
pub static POLLS: Item<Polls> = Item::new(b"polls");
pub static CONFIG: Item<Gateway> = Item::new(b"config");