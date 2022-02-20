use tokio::sync::{RwLock, mpsc};
use tokio::io::{BufReader, AsyncBufReadExt};
use std::collections::HashMap;
use std::sync::{Arc};

use crate::line_parser::{DnsmasqLineParser, DnsmasqParsedLine};
use std::net::IpAddr;

#[derive(Debug, Default)]
pub struct CacheHitsRate {
    pub total_reqs: u64,
    pub hits: u64
}

impl CacheHitsRate {
    pub fn get_ratio(&self) -> f64 {
        self.hits as f64 / self.total_reqs as f64
    }

    pub fn hit(&mut self) {
        self.hits += 1;
        self.total_reqs += 1;
    }

    pub fn miss(&mut self) {
        self.total_reqs += 1;
    }
}

#[derive(Debug, Default)]
pub struct DnsmasqState {
    pub state_enum: DnsmasqStateEnum, 
    pub version: Option<String>,
    pub cache_size: Option<u32>,
    pub name_servers: Vec<String>,
    pub addresses: HashMap<IpAddr, Vec<String>>,
    pub query_sources: HashMap<IpAddr, u64>,
    pub query_types: HashMap<String, u64>,
    pub query_domains: HashMap<String, u64>,
    pub hit_rate: CacheHitsRate
}

#[derive(Debug)]
pub enum DnsmasqStateEnum {
    Uninited,
    Active,
    Error(String)
}

impl Default for DnsmasqStateEnum {
    fn default() -> Self {
        Self::Uninited
    }
}

type StateRef = Arc<RwLock<DnsmasqState>>;

pub enum DnsmasqCommand {
    Update
}

#[derive(Debug, Clone)]
pub struct DnsmasqController {
    pub state: StateRef,
    pub commands: mpsc::Sender<Box<DnsmasqCommand>>,
    task_handle: Arc<tokio::task::JoinHandle<()>>
}


impl DnsmasqController {
    pub fn init() -> Self {
        let state = Arc::new(RwLock::new(DnsmasqState::default()));
        let (cmd_tx, cmd_rx) = mpsc::channel(16);

        let controller_state = state.clone();

        let task_handle = Arc::new(tokio::spawn(async move {
            dnsmasq_ctrl(controller_state, cmd_rx).await
        }));

        return DnsmasqController {
            state,
            task_handle,
            commands: cmd_tx
        };
    }


}

async fn dnsmasq_ctrl(state: StateRef, _cmd_rx: mpsc::Receiver<Box<DnsmasqCommand>>) {
    
    let port: Option<usize> = std::env::var("DNSMASQ_PORT").ok().and_then(
        |ps| str::parse::<usize>(&ps).ok()
    );
    
println!("starting dnsmasq on {:?}", port);

    
    let mut command = tokio::process::Command::new("dnsmasq");
    command.arg("--log-queries=extra");
    command.arg("--keep-in-foreground");
    command.arg("--bind-interfaces");
    command.arg("--log-facility=-");
    command.stderr(std::process::Stdio::piped());
    command.kill_on_drop(true);

    if let Some(p) = port {
        println!("custom port from DNSMASQ_PORT={}", p);
        command.arg(format!("--port={}", p));
    }

    let proc = match 
        command.spawn() {
        Ok(p) => std::sync::Arc::new(tokio::sync::RwLock::new(p)),
        Err(e) => {
            eprintln!("Error starting dnsmasq: {:?}", e);
            state.write().await.state_enum = DnsmasqStateEnum::Error(e.to_string());
            return;
        }
    };

    let p1 = proc.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        let mut proc = p1.write().await;
        match proc.try_wait() {
            Ok(None) => {
                println!("Terminating dnsmasq");
                proc.kill().await.unwrap();
                println!("dnsmasq terminated");
            },
            Ok(Some(e)) => {
                println!("dnsmasq already terminated ({:?})", e.code());
            }
            e => {
                println!("{:?}", e);
            }
        }
        std::process::exit(0);
    });

    let mut stderr = BufReader::new(proc.write().await.stderr.take().unwrap()).lines();

    let parser = DnsmasqLineParser::new().unwrap();

    while let Ok(Some(line)) = stderr.next_line().await {
        println!("[dnsmasq] {}", line);

        match parser.parse_line(&line) {
            Some(DnsmasqParsedLine::Start { version, cache_size }) => {
                let mut w_state = state.write().await;
                w_state.cache_size = Some(cache_size);
                w_state.version = Some(version);
                w_state.state_enum = DnsmasqStateEnum::Active;
            },
            Some(DnsmasqParsedLine::NameServer(server)) => {
                let mut w_state = state.write().await;
                w_state.name_servers.push(server);
            },
            Some(DnsmasqParsedLine::ReadHosts{ path, .. }) => {
                // read data
                eprintln!("parsing readHosts {:?} not implemented", path);
            },
            Some(DnsmasqParsedLine::Query{ from, domain, source, query, ..}) => {
                let mut w_state = state.write().await;
                w_state.query_domains.entry(domain).and_modify(|c| *c += 1).or_insert(1);
                w_state.query_sources.entry(from).and_modify(|c| *c += 1).or_insert(1);
                w_state.query_types.entry(query).and_modify(|c| *c += 1).or_insert(1);
            },
            Some(DnsmasqParsedLine::Reply{ cached, ..}) => {
                let mut w_state = state.write().await;
                if cached {
                    w_state.hit_rate.hit();
                } else {
                    w_state.hit_rate.miss();
                }
            },
            None => {
                eprintln!("unhandled line");
            }
        }
    }
    
}
