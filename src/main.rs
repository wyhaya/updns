#[macro_use]
extern crate lazy_static;

mod config;
mod lib;
mod watch;

use ace::App;
use config::{Config, Hosts, Invalid, Parser};
use dirs;
use lib::*;
use regex::Regex;
use std::{
    env,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    process::Command,
    time::Duration,
};
use tokio::{
    io::{Error, ErrorKind, Result},
    net::UdpSocket,
    prelude::*,
    timer::Timeout,
};
use watch::Watch;

const CONFIG_FILE: [&str; 2] = [".updns", "config"];

const DEFAULT_BIND: &str = "0.0.0.0:53";
const DEFAULT_PROXY: [&str; 2] = ["8.8.8.8:53", "1.1.1.1:53"];
const DEFAULT_TIMEOUT: u64 = 2000;

const WATCH_INTERVAL: u64 = 5000;

static mut PROXY: Vec<SocketAddr> = Vec::new();
static mut HOSTS: Option<Hosts> = None;
static mut TIMEOUT: u64 = DEFAULT_TIMEOUT;

macro_rules! exit {
    ($($arg:tt)*) => {
        {
            eprintln!($($arg)*);
            std::process::exit(1)
        }
    };
}
macro_rules! error {
    ($($arg:tt)*) => {
        eprint!("{} ERROR ", time::now().strftime("[%Y-%m-%d %H:%M:%S]").unwrap());
        eprintln!($($arg)*);
    };
}
macro_rules! info {
    ($($arg:tt)*) => {
        print!("{} INFO ", time::now().strftime("[%Y-%m-%d %H:%M:%S]").unwrap());
        println!($($arg)*);
    };
}
macro_rules! warn {
    ($($arg:tt)*) => {
        print!("{} WARN ", time::now().strftime("[%Y-%m-%d %H:%M:%S]").unwrap());
        println!($($arg)*);
    };
}

#[tokio::main]
async fn main() {
    let app = App::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
        .cmd("add", "Add a DNS record")
        .cmd("ls", "Print all configured DNS records")
        .cmd("config", "Call 'vim' to edit the configuration file")
        .cmd("path", "Print related directories")
        .cmd("help", "Print help information")
        .cmd("version", "Print version information")
        .opt("-c", "Specify a config file")
        .opt("-w", "Check the interval of the configuration file (ms)");

    let config_path = match app.value("-c") {
        Some(values) => {
            if values.is_empty() {
                exit!("'-c' value: [CONFIG]");
            }
            PathBuf::from(values[0])
        }
        None => match dirs::home_dir() {
            Some(p) => p.join(CONFIG_FILE[0]).join(CONFIG_FILE[1]),
            None => exit!("Can't get home directory"),
        },
    };

    // Check profile interval
    let watch_interval = match app.value("-w") {
        Some(values) => {
            if values.is_empty() {
                exit!("'-w' value: [ms]");
            }
            values[0]
                .parse::<u64>()
                .unwrap_or_else(|_| exit!("Cannot resolve '{}' to number", &values[0]))
        }
        None => WATCH_INTERVAL,
    };

    if let Some(cmd) = app.command() {
        match cmd.as_str() {
            "add" => {
                let values = app.value("add").unwrap_or_default();
                if values.len() != 2 {
                    exit!("'add' value: [DOMAIN] [IP]");
                }

                // Check is positive
                if let Err(err) = Regex::new(values[0]) {
                    exit!(
                        "Cannot resolve '{}' to regular expression\n{:?}",
                        values[0],
                        err
                    );
                }
                if values[1].parse::<IpAddr>().is_err() {
                    exit!("Cannot resolve '{}' to ip address", values[1]);
                }

                let mut parser = Parser::new(&config_path).await.unwrap_or_else(|err| {
                    exit!("Failed to read config file {:?}\n{:?}", &config_path, err)
                });

                if let Err(err) = parser.add(values[0], values[1]).await {
                    exit!("Add record failed\n{:?}", err);
                }
            }
            "ls" => {
                let mut config = config_parse(&config_path).await;

                let n = config
                    .hosts
                    .iter()
                    .map(|(r, _)| r.as_str().len())
                    .fold(0, |a, b| a.max(b));

                for (host, ip) in config.hosts.iter() {
                    println!("{:domain$}    {}", host.as_str(), ip, domain = n);
                }
            }
            "config" => {
                let status = Command::new("vim")
                    .arg(&config_path)
                    .status()
                    .unwrap_or_else(|err| exit!("Call 'vim' command failed\n{:?}", err));

                if status.success() {
                    config_parse(&config_path).await;
                } else {
                    println!("'vim' exits with a non-zero status code: {:?}", status);
                }
            }
            "path" => {
                let binary = env::current_exe()
                    .unwrap_or_else(|err| exit!("Failed to get directory\n{:?}", err));

                println!(
                    "Binary: {}\nConfig: {}",
                    binary.display(),
                    config_path.display()
                );
            }
            "help" => app.help(),
            "version" => app.version(),
            _ => app.error_try("help"),
        }
        return;
    }

    let mut config = config_parse(&config_path).await;
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

    update_config(config.proxy, config.hosts, config.timeout);

    // Run server
    for addr in config.bind {
        tokio::spawn(run_server(addr));
    }
    // watch config
    watch_config(config_path, watch_interval).await;
}

fn update_config(mut proxy: Vec<SocketAddr>, hosts: Hosts, timeout: Option<u64>) {
    if proxy.is_empty() {
        proxy = DEFAULT_PROXY
            .iter()
            .map(|p| p.parse().unwrap())
            .collect::<Vec<SocketAddr>>();
    }
    unsafe {
        PROXY = proxy;
        HOSTS = Some(hosts);
        TIMEOUT = timeout.unwrap_or(DEFAULT_TIMEOUT);
    };
}

async fn config_parse(file: &PathBuf) -> Config {
    let parser = Parser::new(file)
        .await
        .unwrap_or_else(|err| exit!("Failed to read config file {:?}\n{:?}", file, err));

    let config: Config = parser
        .parse()
        .await
        .unwrap_or_else(|err| exit!("Parsing config file failed\n{:?}", err));

    output_invalid(&config.invalid);
    config
}

fn output_invalid(errors: &[Invalid]) {
    for invalid in errors {
        error!(
            "[line:{}] {} `{}`",
            invalid.line,
            invalid.kind.as_str(),
            invalid.source
        );
    }
}

async fn watch_config(p: PathBuf, t: u64) {
    let mut watch = Watch::new(&p, t).await;

    while let Some(_) = watch.next().await {
        info!("Reload the configuration file: {:?}", &p);
        if let Ok(parser) = Parser::new(&p).await {
            if let Ok(config) = parser.parse().await {
                update_config(config.proxy, config.hosts, config.timeout);
                output_invalid(&config.invalid);
            }
        }
    }
}

async fn run_server(addr: SocketAddr) {
    let mut socket = match UdpSocket::bind(&addr).await {
        Ok(socket) => {
            info!("Start listening to '{}'", addr);
            socket
        }
        Err(err) => exit!("Binding '{}' failed\n{:?}", addr, err),
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
    let proxy = unsafe { &PROXY };

    for addr in proxy.iter() {
        let mut socket = UdpSocket::bind(("0.0.0.0", 0)).await?;

        let data: Result<Vec<u8>> = Timeout::new(
            async {
                socket.send_to(&buf, addr).await?;
                let mut res = [0; 512];
                let len = socket.recv(&mut res).await?;
                Ok(res[..len].to_vec())
            },
            Duration::from_millis(unsafe { TIMEOUT }),
        )
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

fn get_answer(domain: &str, query: QueryType) -> Option<DnsRecord> {
    let hosts = unsafe { HOSTS.as_ref().unwrap() };
    if let Some(ip) = hosts.get(domain) {
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
    let answer = match get_answer(&query.name, query.qtype) {
        Some(record) => record,
        None => return proxy(&req.buf[..len]).await,
    };

    request.header.recursion_desired = true;
    request.header.recursion_available = true;
    request.header.response = true;
    request.answers.push(answer);
    let mut res_buffer = BytePacketBuffer::new();
    request.write(&mut res_buffer)?;

    let len = res_buffer.pos();
    let data = res_buffer.get_range(0, len)?;
    Ok(data.to_vec())
}
