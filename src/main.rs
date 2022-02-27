use warp::{Filter};
use std::convert::Infallible;

mod dnsmasq;
mod line_parser;
mod responses;
use crate::dnsmasq::*;


fn with_dns_controller(dns_controller: DnsmasqController) -> impl Filter<Extract = (DnsmasqController,), Error = Infallible> + Clone {
    warp::any().map(move || dns_controller.clone())
}

async fn get_api_static(dns: DnsmasqController) -> Result<impl warp::Reply, Infallible> {
    let state = dns.state.read().await;

    let res = responses::StaticStateResponse {
        cache_size: state.cache_size,
        name_servers: &state.name_servers,
        version: state.version.as_deref(),
        mappings: state.addresses.clone()
    };

    return Ok(warp::reply::json(&res));
}


async fn get_api_dyn(dns: DnsmasqController) -> Result<impl warp::Reply, Infallible> {
    let state = dns.state.read().await;

    let res = responses::DynStateResponse {
        num_hits: state.hit_rate.hits,
        num_total: state.hit_rate.total_reqs,
        percent_from_cache: state.hit_rate.get_ratio(),
        top_query_domains: &state.query_domains,
        top_query_sources: &state.query_sources,
        top_query_types: &state.query_types,
        unknown_domains: &state.nxdomain_replies,
        lookup_timeline: &state.timeline
    };

    return Ok(warp::reply::json(&res));
}

/// mounted under /api
fn build_api(dns_controller: DnsmasqController) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    let state = 
        // GET /api
        warp::path::end().map(|| "api up")
    .or(
        // GET /api/state
        warp::path("static")
        .and(with_dns_controller(dns_controller.clone()))
        .and(warp::get())
        .and_then(get_api_static)
    ).or(
        warp::path("dynamic")
        .and(with_dns_controller(dns_controller.clone()))
        .and(warp::get())
        .and_then(get_api_dyn)
    );
    
    state
}

#[tokio::main]
async fn main() {

    let dns_controller = DnsmasqController::init();
    

    let index = warp::any().and(warp::fs::file("frontend/dist/index.html"));
    // GET /hello/warp => 200 OK with body "Hello, warp!"
    let health = warp::path!("health").map(|| "server active");

    let frontend_assets = warp::path("assets").and(warp::fs::dir("frontend/dist/assets"));


    let api = warp::path("api").and(build_api(dns_controller));

    let port:u16 = std::env::var("WEB_PORT").ok().and_then(
        |ps| str::parse::<u16>(&ps).ok()
    ).unwrap_or(80);

    println!("Running on port {}", port);

    warp::serve(health
        .or(frontend_assets)
        .or(api)
        .or(index)
    )
    .run(([0, 0, 0, 0], port))
    .await;
}
