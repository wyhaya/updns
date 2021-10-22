mod cli;
mod config;
mod lib;
mod matcher;
mod watch;

use cli::{parse_args, AppRunType};
use config::{Config, Hosts, MultipleInvalid, Parser};
use futures_util::StreamExt;
use lazy_static::lazy_static;
use lib::*;
use logs::{error, info, warn};
use std::{
    env,
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};
use tokio::{
    io::{Error, ErrorKind, Result},
    net::UdpSocket,
    sync::RwLock,
    time::timeout,
};
use watch::Watch;

const CONFIG_FILE: [&str; 2] = [".updns", "config"];
const WATCH_INTERVAL: Duration = Duration::from_millis(5000);
const DEFAULT_BIND: &str = "0.0.0.0:53";
const DEFAULT_PROXY: [&str; 2] = ["8.8.8.8:53", "1.1.1.1:53"];
const DEFAULT_TIMEOUT: Duration = Duration::from_millis(2000);

lazy_static! {
    static ref PROXY: RwLock<Vec<SocketAddr>> = RwLock::new(Vec::new());
    static ref HOSTS: RwLock<Hosts> = RwLock::new(Hosts::new());
    static ref TIMEOUT: RwLock<Duration> = RwLock::new(DEFAULT_TIMEOUT);
}

#[macro_export]
macro_rules! exit {
    ($($arg:tt)*) => {
        {
            logs::error!($($arg)*);
            std::process::exit(1)
        }
    };
}

#[tokio::main]
async fn main() {
    match parse_args() {
        AppRunType::AddRecord { path, ip, host } => {
            let mut parser = Parser::new(&path)
                .await
                .unwrap_or_else(|err| exit!("Failed to read config file {:?}\n{:?}", &path, err));

            if let Err(err) = parser.add(&host, &ip).await {
                exit!("Add record failed\n{:?}", err);
            }
        }
        AppRunType::PrintRecord { path } => {
            let mut config = force_get_config(&path).await;
            let n = config
                .hosts
                .iter()
                .map(|(m, _)| m.to_string().len())
                .fold(0, |a, b| a.max(b));

            for (host, ip) in config.hosts.iter() {
                println!("{:domain$}    {}", host.to_string(), ip, domain = n);
            }
        }
        AppRunType::EditConfig { path } => {
            let status = Command::new("vim")
                .arg(&path)
                .status()
                .unwrap_or_else(|err| exit!("Call 'vim' command failed\n{:?}", err));

            if status.success() {
                force_get_config(&path).await;
            } else {
                exit!("'vim' exits with a non-zero status code: {:?}", status);
            }
        }
        AppRunType::PrintPath { path } => {
            let binary = env::current_exe()
                .unwrap_or_else(|err| exit!("Failed to get directory\n{:?}", err));

            println!("Binary: {}\nConfig: {}", binary.display(), path.display());
        }
        AppRunType::Run { path, duration } => {
            let mut config = force_get_config(&path).await;
            if config.bind.is_empty() {
                warn!("Will bind the default address '{}'", DEFAULT_BIND);
                config.bind.push(DEFAULT_BIND.parse().unwrap());
            }
            if config.proxy.is_empty() {
                warn!(
                    "Will use the default proxy address '{}'",
                    DEFAULT_PROXY.join(", ")
                );
            }

            update_config(config.proxy, config.hosts, config.timeout).await;

            // Run server
            for addr in config.bind {
                tokio::spawn(run_server(addr));
            }
            // watch config
            watch_config(path, duration).await;
        }
    }
}

async fn update_config(mut proxy: Vec<SocketAddr>, hosts: Hosts, timeout: Option<Duration>) {
    if proxy.is_empty() {
        proxy = DEFAULT_PROXY
            .iter()
            .map(|p| p.parse().unwrap())
            .collect::<Vec<SocketAddr>>();
    }

    {
        let mut w = PROXY.write().await;
        *w = proxy;
    }
    {
        let mut w = HOSTS.write().await;
        *w = hosts;
    }
    {
        let mut w = TIMEOUT.write().await;
        *w = timeout.unwrap_or(DEFAULT_TIMEOUT);
    }
}

async fn force_get_config(file: &Path) -> Config {
    let parser = Parser::new(file)
        .await
        .unwrap_or_else(|err| exit!("Failed to read config file {:?}\n{:?}", file, err));

    let config: Config = parser
        .parse()
        .await
        .unwrap_or_else(|err| exit!("Parsing config file failed\n{:?}", err));

    config.invalid.print();
    config
}

async fn watch_config(p: PathBuf, d: Duration) {
    let mut watch = Watch::new(&p, d).await;
    while watch.next().await.is_some() {
        info!("Reload the configuration file: {:?}", &p);
        if let Ok(parser) = Parser::new(&p).await {
            if let Ok(config) = parser.parse().await {
                update_config(config.proxy, config.hosts, config.timeout).await;
                config.invalid.print();
            }
        }
    }
}

async fn run_server(addr: SocketAddr) {
    let socket = match UdpSocket::bind(&addr).await {
        Ok(socket) => {
            info!("Start listening to '{}'", addr);
            socket
        }
        Err(err) => {
            exit!("Binding '{}' failed\n{:?}", addr, err)
        }
    };

    loop {
        let mut req = BytePacketBuffer::new();

        let (len, src) = match socket.recv_from(&mut req.buf).await {
            Ok(r) => r,
            Err(err) => {
                error!("Failed to receive message {:?}", err);
                continue;
            }
        };

        let res = match handle(req, len).await {
            Ok(data) => data,
            Err(err) => {
                error!("Processing request failed {:?}", err);
                continue;
            }
        };

        if let Err(err) = socket.send_to(&res, &src).await {
            error!("Replying to '{}' failed {:?}", &src, err);
        }
    }
}

async fn proxy(buf: &[u8]) -> Result<Vec<u8>> {
    let proxy = PROXY.read().await;
    let duration = *TIMEOUT.read().await;

    for addr in proxy.iter() {
        let socket = UdpSocket::bind(("0.0.0.0", 0)).await?;

        let data: Result<Vec<u8>> = timeout(duration, async {
            socket.send_to(buf, addr).await?;
            let mut res = [0; 512];
            let len = socket.recv(&mut res).await?;
            Ok(res[..len].to_vec())
        })
        .await?;

        match data {
            Ok(data) => {
                return Ok(data);
            }
            Err(err) => {
                error!("Agent request to {} {:?}", addr, err);
            }
        }
    }

    Err(Error::new(
        ErrorKind::Other,
        "Proxy server failed to proxy request",
    ))
}

async fn get_answer(domain: &str, query: QueryType) -> Option<DnsRecord> {
    if let Some(ip) = HOSTS.read().await.get(domain) {
        match query {
            QueryType::A => {
                if let IpAddr::V4(addr) = ip {
                    return Some(DnsRecord::A {
                        domain: domain.to_string(),
                        addr: *addr,
                        ttl: 3600,
                    });
                }
            }
            QueryType::AAAA => {
                if let IpAddr::V6(addr) = ip {
                    return Some(DnsRecord::AAAA {
                        domain: domain.to_string(),
                        addr: *addr,
                        ttl: 3600,
                    });
                }
            }
            _ => {}
        }
    }
    None
}

async fn handle(mut req: BytePacketBuffer, len: usize) -> Result<Vec<u8>> {
    let mut request = DnsPacket::from_buffer(&mut req)?;

    let query = match request.questions.get(0) {
        Some(q) => q,
        None => return proxy(&req.buf[..len]).await,
    };

    info!("{} {:?}", query.name, query.qtype);

    // Whether to proxy
    let answer = match get_answer(&query.name, query.qtype).await {
        Some(record) => record,
        None => return proxy(&req.buf[..len]).await,
    };

    request.header.recursion_desired = true;
    request.header.recursion_available = true;
    request.header.response = true;
    request.answers.push(answer);
    let mut res_buffer = BytePacketBuffer::new();
    request.write(&mut res_buffer)?;

    let data = res_buffer.get_range(0, res_buffer.pos())?;
    Ok(data.to_vec())
}
