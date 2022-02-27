use std::{collections::HashMap, net::IpAddr};

use serde_derive::Serialize;

use crate::dnsmasq::TimeBucket;


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
pub struct DynStateResponse<'a> {
    pub num_hits: u64,
    pub num_total: u64,
    pub percent_from_cache: f64,
    pub top_query_domains: &'a HashMap<String, u64>,
    pub top_query_types: &'a HashMap<String, u64>,
    pub top_query_sources: &'a HashMap<IpAddr, u64>,
    pub unknown_domains: &'a HashMap<String, u64>,
    pub lookup_timeline: &'a Vec<TimeBucket>
}