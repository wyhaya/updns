#![feature(const_vec_new)]

#[macro_use]
extern crate lazy_static;

mod config;
mod lib;
mod watch;

use ace::App;
use async_std::io;
use async_std::net::UdpSocket;
use async_std::task;
use config::{Config, Hosts, Invalid, InvalidType};
use dirs;
use lib::*;
use std::env;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};
use watch::Watch;

const CONFIG_NAME: &str = ".updns";
const DEFAULT_BIND: &str = "0.0.0.0:53";
const DEFAULT_PROXY: [&str; 2] = ["8.8.8.8:53", "114.114.114.114:53"];
const PROXY_TIMEOUT: u64 = 2000;
const WATCH_INTERVAL: u64 = 3000;
static mut PROXY: Vec<SocketAddr> = Vec::new();
static mut HOSTS: Option<Hosts> = None;

macro_rules! log {
    ($($arg:tt)*) => {
        println!($($arg)*);
    };
}

macro_rules! warn {
    ($($arg:tt)*) => {
        print!("\x1B[{}m{}\x1B[0m", "1;33", "warning: ");
        println!($($arg)*);
    };
}

macro_rules! error {
    ($($arg:tt)*) => {
        eprint!("\x1B[{}m{}\x1B[0m", "1;31", "error: ");
        eprintln!($($arg)*);
    };
}

macro_rules! exit {
    ($($arg:tt)*) => {
        {
            eprint!("\x1B[{}m{}\x1B[0m", "1;31", "error: ");
            eprintln!($($arg)*);
            std::process::exit(1)
        }
    };
}

fn main() {
    let app = App::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
        .cmd("add", "Add a DNS record")
        .cmd("rm", "Remove a DNS record")
        .cmd("ls", "Print all configured DNS records")
        .cmd("config", "Call vim to edit the configuration file")
        .cmd("path", "Print related directories")
        .cmd("help", "Print help information")
        .cmd("version", "Print version information")
        .opt("-c", "Specify a config file");

    let config_path = match app.value("-c") {
        Some(values) => {
            if values.is_empty() {
                exit!("'-c' value: [CONFIG]");
            }
            PathBuf::from(values[0])
        }
        None => match dirs::home_dir() {
            Some(p) => p.join(CONFIG_NAME),
            None => exit!("Can't get home directory"),
        },
    };

    if let Some(cmd) = app.command() {
        match cmd.as_str() {
            "add" => {
                let values = app.value("add").unwrap_or(vec![]);
                if values.len() != 2 {
                    exit!("'add' value: [DOMAIN] [IP]");
                }
                let mut config = match Config::new(&config_path) {
                    Ok(c) => c,
                    Err(err) => exit!("Failed to read config file: {:?}\n{:?}", &config_path, err),
                };
                if let Err(err) = config.add(&values[0], &values[1]) {
                    exit!("Add record failed\n{:?}", err);
                }
            }
            "rm" => {
                if let Some(value) = app.value("rm") {
                    if value.is_empty() {
                        exit!("'rm' value: [DOMAIN | IP]");
                    }
                }
            }
            "ls" => {
                let (_, _, _, mut hosts) = config_parse(&config_path);
                let mut n = 0;
                for (reg, _) in hosts.iter() {
                    if reg.as_str().len() > n {
                        n = reg.as_str().len();
                    }
                }
                for (domain, ip) in hosts.iter() {
                    println!("{:domain$}    {}", domain.as_str(), ip, domain = n);
                }
            }
            "config" => {
                let cmd = Command::new("vim").arg(&config_path).status();
                match cmd {
                    Ok(status) => {
                        if status.success() {
                            config_parse(&config_path);
                        } else {
                            warn!("Non-zero state exit\n{:?}", status);
                        }
                    }
                    Err(err) => exit!("Call vim command failed\n{:?}", err),
                }
            }
            "path" => {
                let binary = match env::current_exe() {
                    Ok(p) => p.display().to_string(),
                    Err(err) => exit!("Failed to get directory\n{:?}", err),
                };
                println!("Binary: {}\nConfig: {:?}", binary, config_path);
            }
            "help" => {
                app.help();
            }
            "version" => {
                app.version();
            }
            _ => {
                app.error_try("help");
            }
        }
        return;
    }

    let (_, mut binds, proxys, hosts) = config_parse(&config_path);
    if binds.is_empty() {
        warn!("Will bind the default address '{}'", DEFAULT_BIND);
        binds.push(DEFAULT_BIND.parse().unwrap());
    }
    if proxys.is_empty() {
        warn!(
            "Will use the default proxy address '{}'",
            DEFAULT_PROXY.join(", ")
        );
    }
    update_config(proxys, hosts);

    task::spawn(watch_config(config_path));
    task::block_on(run_server(binds));
}

fn config_parse(file: &PathBuf) -> (Config, Vec<SocketAddr>, Vec<SocketAddr>, Hosts) {
    let mut config = match Config::new(file) {
        Ok(c) => c,
        Err(err) => exit!("Failed to read config file: {:?}\n{:?}", file, err),
    };

    let (binds, proxys, hosts, errors) = match config.parse() {
        Ok(d) => d,
        Err(err) => exit!("Parsing config file failed\n{:?}", err),
    };
    output_invalid(errors);

    (config, binds, proxys, hosts)
}

fn output_invalid(errors: Vec<Invalid>) {
    if !errors.is_empty() {
        for invalid in errors {
            let msg = match invalid.err {
                InvalidType::SocketAddr => "Cannot parse socket addr",
                InvalidType::IpAddr => "Cannot parse ip addr",
                InvalidType::Regex => "Cannot parse Regular expression",
                InvalidType::Other => "Invalid line",
            };
            warn!("{}", msg);
            log!("Line {}: {}", invalid.line, invalid.source);
        }
    }
}

async fn watch_config(p: PathBuf) {
    let mut watch = Watch::new(p, WATCH_INTERVAL);
    watch
        .change(|c| {
            log!("Reload the configuration file: {:?}", &c);
            if let Ok(mut config) = Config::new(c) {
                if let Ok((_, proxy, hosts, errors)) = config.parse() {
                    update_config(proxy, hosts);
                    output_invalid(errors);
                }
            }
        })
        .await;
}

fn update_config(mut proxy: Vec<SocketAddr>, hosts: Hosts) {
    if proxy.is_empty() {
        proxy = DEFAULT_PROXY
            .iter()
            .map(|p| p.parse().unwrap())
            .collect::<Vec<SocketAddr>>();
    }
    unsafe {
        PROXY = proxy;
        HOSTS = Some(hosts);
    };
}

async fn run_server(binds: Vec<SocketAddr>) {
    let mut tasks = vec![];
    for addr in binds {
        let task = task::spawn(async move {
            let socket = match UdpSocket::bind(&addr).await {
                Ok(socket) => {
                    log!("Start listening to '{}'", addr);
                    socket
                }
                Err(err) => exit!("Binding '{}' failed\n{:?}", addr, err),
            };
            loop {
                let mut req = BytePacketBuffer::new();
                match socket.recv_from(&mut req.buf).await {
                    Ok((len, src)) => {
                        let res = match handle(req, len).await {
                            Ok(data) => data,
                            Err(err) => {
                                error!("Processing request failed\n{:?}", err);
                                continue;
                            }
                        };
                        if let Err(err) = socket.send_to(&res, &src).await {
                            error!("Replying to '{}' failed\n{:?}", &src, err);
                        }
                    }
                    Err(err) => {
                        error!("Failed to receive message\n{:?}", err);
                    }
                }
            }
        });
        tasks.push(task);
    }
    for task in tasks {
        task.await;
    }
}

async fn proxy(buf: &[u8]) -> io::Result<Vec<u8>> {
    let proxy = unsafe { &PROXY };

    for addr in proxy.iter() {
        let socket = UdpSocket::bind(("0.0.0.0", 0)).await?;

        let data = io::timeout(Duration::from_millis(PROXY_TIMEOUT), async {
            socket.send_to(&buf, addr).await?;
            let mut res = [0; 512];
            let len = socket.recv(&mut res).await?;
            Ok(res[..len].to_vec())
        })
        .await;

        match data {
            Ok(data) => {
                return Ok(data);
            }
            Err(err) => {
                error!("Agent request to {}\n{:?}", addr, err);
            }
        }
    }

    Err(io::Error::new(
        io::ErrorKind::Other,
        "Proxy server failed to proxy request",
    ))
}

fn get_answer(domain: &str, query: QueryType) -> Option<DnsRecord> {
    let hosts = unsafe { HOSTS.as_ref().unwrap() };
    if let Some(ip) = hosts.get(domain) {
        match query {
            QueryType::A => {
                if let IpAddr::V4(addr) = ip {
                    return Some(DnsRecord::A {
                        domain: domain.to_string(),
                        addr: addr.clone(),
                        ttl: 3600,
                    });
                }
            }
            QueryType::AAAA => {
                if let IpAddr::V6(addr) = ip {
                    return Some(DnsRecord::AAAA {
                        domain: domain.to_string(),
                        addr: addr.clone(),
                        ttl: 3600,
                    });
                }
            }
            _ => {}
        }
    }
    None
}

async fn handle(mut req: BytePacketBuffer, len: usize) -> io::Result<Vec<u8>> {
    let mut request = DnsPacket::from_buffer(&mut req)?;

    let query = match request.questions.get(0) {
        Some(q) => q,
        None => return proxy(&req.buf[..len]).await,
    };

    log!("Query: {} Type: {:?}", query.name, query.qtype);

    if let Some(answer) = get_answer(&query.name, query.qtype) {
        request.header.recursion_desired = true;
        request.header.recursion_available = true;
        request.header.response = true;
        request.answers.push(answer);
        let mut res_buffer = BytePacketBuffer::new();
        request.write(&mut res_buffer)?;
        let len = res_buffer.pos();
        let data = res_buffer.get_range(0, len)?;
        Ok(data.to_vec())
    } else {
        proxy(&req.buf[..len]).await
    }
}
