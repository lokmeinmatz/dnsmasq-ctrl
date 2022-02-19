use std::sync::Arc;

use warp::{Filter};
use tokio::sync::{watch, mpsc};
use tokio::io::{BufReader, AsyncBufReadExt};
use std::convert::Infallible;

#[derive(Debug)]
enum DnsmasqState {
    Uninited,
    Active,
    Error(String)
}

enum DnsmasqCommand {
    Update
}

#[derive(Debug, Clone)]
struct DnsmasqController {
    watch_state: watch::Receiver<Box<DnsmasqState>>,
    commands: mpsc::Sender<Box<DnsmasqCommand>>,
    task_handle: Arc<tokio::task::JoinHandle<()>>
}


async fn dnsmasq_ctrl(state_tx: watch::Sender<Box<DnsmasqState>>, cmd_rx: mpsc::Receiver<Box<DnsmasqCommand>>) {
    
    let port: Option<usize> = std::env::var("DNSMASQ_PORT").ok().and_then(
        |ps| str::parse::<usize>(&ps).ok()
    );
    
    println!("starting dnsmasq");

    
    let mut command = tokio::process::Command::new("dnsmasq");
    command.arg("--log-queries");
    command.stdout(std::process::Stdio::piped());

    if let Some(p) = port {
        println!("custom port from DNSMASQ_PORT={}", p);
        command.arg(format!("--port={}", p));
    }

    let proc = match 
        command.spawn() {
        Ok(p) => std::sync::Arc::new(tokio::sync::RwLock::new(p)),
        Err(e) => {
            eprintln!("Error starting dnsmasq: {:?}", e);
            state_tx.send(Box::new(DnsmasqState::Error(e.to_string()))).unwrap();
            return;
        }
    };

    let p1 = proc.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        let mut proc = p1.write().await;
        if let Ok(None) = proc.try_wait() {
            println!("Terminating dnsmasq");
            proc.kill().await.unwrap();
            println!("dnsmasq terminated");
        }
        std::process::exit(0);
    });

    let mut stdout = BufReader::new(proc.write().await.stdout.take().unwrap());
    
    while let Ok(None) = proc.write().await.try_wait() {
        let mut line = String::new();
        stdout.read_line(&mut line).await.unwrap();
        println!("[dnsmasq] {}", line);
    }
    
}

impl DnsmasqController {
    fn init() -> Self {
        let (state_tx, state_rx) = watch::channel(Box::new(DnsmasqState::Uninited));
        let (cmd_tx, cmd_rx) = mpsc::channel(16);

        let task_handle = Arc::new(tokio::spawn(async move {
            dnsmasq_ctrl(state_tx, cmd_rx).await
        }));

        return DnsmasqController {
            watch_state: state_rx,
            task_handle,
            commands: cmd_tx
        };
    }


}

fn with_dns_controller(dns_controller: DnsmasqController) -> impl Filter<Extract = (DnsmasqController,), Error = Infallible> + Clone {
    warp::any().map(move || dns_controller.clone())
}

async fn get_api_state(dns: DnsmasqController) -> Result<impl warp::Reply, Infallible> {
    Ok("unimplemented")
}


/// mounted under /api
fn build_api(dns_controller: DnsmasqController) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::any()
        .and(with_dns_controller(dns_controller))
        .and(warp::path("state").and(warp::get()).map(|| format!("test"))
    )
}

#[tokio::main]
async fn main() {

    let dns_controller = DnsmasqController::init();
    

    let index = warp::any().and(warp::fs::file("frontend/dist/index.html"));
    // GET /hello/warp => 200 OK with body "Hello, warp!"
    let health = warp::path!("health").map(|| "server active");

    let frontend_assets = warp::path("assets").and(warp::fs::dir("frontend/dist/assets"));


    let api = warp::path("api").and(build_api(dns_controller));

    warp::serve(health
        .or(frontend_assets)
        .or(api)
        .or(index)
    )
    .run(([127, 0, 0, 1], 3030))
    .await;
}
