use chrono::{Utc};
use rusqlite::types::{ToSqlOutput, FromSql, FromSqlError};
use rusqlite::{Connection, params, ToSql};
use tokio::sync::{mpsc, Mutex};
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


#[derive(Debug)]
pub struct DnsmasqState {
    pub state_enum: DnsmasqStateEnum, 
    pub version: Option<String>,
    pub cache_size: Option<u32>,
    pub name_servers: Vec<String>,
    pub addresses: HashMap<IpAddr, Vec<String>>,
    pub sql_conn: Connection
    /*,
    pub query_sources: HashMap<IpAddr, u64>,
    pub query_types: HashMap<String, u64>,
    pub query_domains: HashMap<String, u64>,
    pub hit_rate: CacheHitsRate,
    pub nxdomain_replies: HashMap<String, u64>,
    pub timeline: Vec<TimeBucket> */
}

#[derive(Debug, PartialEq)]
pub enum QueryState {
    RUNNING = 0,
    HIT = 1,
    MISS = 2,
    NX = 3
}

impl ToSql for QueryState {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(*self as u8))
    }
}

impl FromSql for QueryState {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        value.as_i64().and_then(|s| {
            match s {
                0 => Ok(QueryState::RUNNING),
                1 => Ok(QueryState::HIT),
                2 => Ok(QueryState::MISS),
                3 => Ok(QueryState::NX),
                _ => Err(FromSqlError::InvalidType)
            }
        })
    }
}

impl DnsmasqState {
    pub fn empty() -> Self {

        let mut conn = Connection::open_in_memory().unwrap();
        // timestamp UNIX ms, 
        // state 0 = running, 1 = hit, 2 = miss, 3 = nx
        conn.execute("CREATE TABLE dns_queries ( id INTEGER PRIMARY KEY AUTOINCREMENT, timestamp INTEGER NOT NULL, type TEXT NOT NULL, domain TEXT NOT NULL, state INTEGER NOT NULL, source TEXT NOT NULL, duration INTEGER);", params![]).unwrap();

        Self {
            addresses: HashMap::new(),
            cache_size: None,
            name_servers: vec![],
            version: None,
            state_enum: DnsmasqStateEnum::Uninited,
            sql_conn: conn
        }
    }

    pub fn start_query(&self, id: u64, source: String, domain: String, query_type: String) {
        let now = Utc::now().timestamp_millis();
        match self.sql_conn.execute("INSERT INTO dns_queries (id, timestamp, type, domain, state, source) VALUES (?1, ?2, ?3, ?4, ?5, ?6);", params![
            id, now, query_type, domain, QueryState::RUNNING, source
        ]) {
            Ok(_) => {},
            Err(e) => eprintln!("Error while inserting: {:?}", e)
        }
    }

    pub fn finish_query(&self, id: u64, state: QueryState) {
        let tx = self.sql_conn.transaction().unwrap();
        let (timestamp, curr_state) = match tx.query_row("SELECT (id, timestamp, state) FROM dns_queries WHERE id = ?1", params![id], |row| {
            let ts: u64 = row.get(1)?;
            let curr_state: QueryState = row.get(2)?;
            Ok((ts, curr_state))
        }) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Error while finding query {:?} : {:?}", id, e);
                return;
            }
        };

        if curr_state != QueryState::RUNNING {
            eprintln!("query state was not running when finish_query was called");
        }

        let dur = Utc::now().timestamp_millis() as u64 - timestamp;
        tx.execute("UPDATE dns_queries SET state = ?1, duration = ?2 WHERE id = ?3", params![
            state, dur, id
        ]).unwrap();
    }
}

#[derive(Debug)]
pub enum DnsmasqStateEnum {
    Uninited,
    Active,
    Error(String)
}

type StateRef = Arc<Mutex<DnsmasqState>>;

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
        let state = Arc::new(Mutex::new(DnsmasqState::empty()));
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
            state.lock().await.state_enum = DnsmasqStateEnum::Error(e.to_string());
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
                let mut state_guard = state.lock().await;
                state_guard.cache_size = Some(cache_size);
                state_guard.version = Some(version);
                state_guard.state_enum = DnsmasqStateEnum::Active;
            },
            Some(DnsmasqParsedLine::NameServer(server)) => {
                let mut state_guard = state.lock().await;
                state_guard.name_servers.push(server);
            },
            Some(DnsmasqParsedLine::ReadHosts{ path, .. }) => {
                // read data
                eprintln!("parsing readHosts {:?} not implemented", path);
            },
            Some(DnsmasqParsedLine::Query{ id, from, domain, query, source}) => {
                let mut state_guard = state.lock().await;
                state_guard.start_query(id, source, domain, query);

            },
            Some(DnsmasqParsedLine::Reply{ id, domain, cached, result, ..}) => {
                let state_guard = state.lock().await;
                let mut state = QueryState::RUNNING;
                if cached {
                    state = QueryState::HIT;
                } else {
                    state = QueryState::MISS;
                }

                if result.is_none() {
                    state = QueryState::NX;
                }

                state_guard.finish_query(id, state)
            },
            None => {
                eprintln!("unhandled line");
            }
        }
    }
    
}
