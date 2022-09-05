use std::{collections::HashMap, net::IpAddr};

use serde_derive::Serialize;



#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StaticStateResponse<'a> {
    pub version: Option<&'a str>,
    pub cache_size: Option<u32>,
    pub name_servers: &'a [String],
    pub mappings: HashMap<IpAddr, Vec<String>>
}


#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DynStateResponse {
    /// UNIX ms
    pub timestamp: u64,
    /// ms
    pub frame_size: u64,
    pub num_hits: u64,
    pub num_reqs: u64,
    pub num_since_start: u64,
    pub percent_from_cache: f64,
}