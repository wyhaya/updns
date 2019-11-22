use futures::future::{BoxFuture, FutureExt};
use regex::Regex;
use std::{
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
    result,
    slice::Iter,
};
use tokio::{
    fs::{create_dir_all, File, OpenOptions},
    io::{AsyncReadExt, AsyncWriteExt, Result},
};

lazy_static! {
    static ref REG_IGNORE: Regex = Regex::new(r#"^\s*(#.*)?$"#).unwrap();
    static ref REG_BIND: Regex = Regex::new(r#"^\s*bind\s+(?P<val>[^\s#]+)"#).unwrap();
    static ref REG_PROXY: Regex = Regex::new(r#"^\s*proxy\s+(?P<val>[^\s#]+)"#).unwrap();
    static ref REG_TIMEOUT: Regex = Regex::new(r#"^\s*timeout\s+(?P<val>[^\s#]+)"#).unwrap();
    // todo
    // The path will also contain '#' and ' '
    static ref REG_IMPORT: Regex = Regex::new(r#"^\s*import\s+(?P<val>(.*))$"#).unwrap();
    static ref REG_DOMAIN_IP: Regex = Regex::new(r#"^\s*(?P<val1>[^\s#]+)\s+(?P<val2>[^\s#]+)"#).unwrap();
}

fn cap_socket_addr(reg: &Regex, text: &str) -> Option<result::Result<SocketAddr, InvalidType>> {
    let cap = match reg.captures(text) {
        Some(cap) => cap,
        None => return None,
    };

    match cap.name("val") {
        Some(m) => match m.as_str().parse() {
            Ok(addr) => Some(Ok(addr)),
            Err(_) => Some(Err(InvalidType::SocketAddr)),
        },
        None => Some(Err(InvalidType::SocketAddr)),
    }
}

fn cap_ip_addr(text: &str) -> Option<result::Result<(Host, IpAddr), InvalidType>> {
    let cap = match (&REG_DOMAIN_IP as &Regex).captures(text) {
        Some(cap) => cap,
        None => return None,
    };

    let (val1, val2) = match (cap.name("val1"), cap.name("val2")) {
        (Some(val1), Some(val2)) => (val1.as_str(), val2.as_str()),
        _ => {
            return Some(Err(InvalidType::Other));
        }
    };

    // ip domain
    if let Ok(ip) = val1.parse() {
        return match Host::new(val2) {
            Ok(host) => Some(Ok((host, ip))),
            Err(_) => Some(Err(InvalidType::Regex)),
        };
    }

    // domain ip
    let ip = match val2.parse() {
        Ok(ip) => ip,
        Err(_) => return Some(Err(InvalidType::IpAddr)),
    };

    let host = match Host::new(val1) {
        Ok(reg) => reg,
        Err(_) => return Some(Err(InvalidType::Regex)),
    };

    Some(Ok((host, ip)))
}

#[derive(Debug)]
pub struct Invalid {
    pub line: usize,
    pub source: String,
    pub kind: InvalidType,
}

#[derive(Debug)]
pub enum InvalidType {
    Regex,
    SocketAddr,
    IpAddr,
    Timeout,
    Other,
}

impl InvalidType {
    pub fn text(&self) -> &str {
        match self {
            InvalidType::SocketAddr => "Cannot parse socket addr",
            InvalidType::IpAddr => "Cannot parse ip addr",
            InvalidType::Regex => "Cannot parse Regular expression",
            InvalidType::Timeout => "Cannot parse timeout",
            InvalidType::Other => "Invalid line",
        }
    }
}

#[derive(Debug)]
pub struct Hosts {
    record: Vec<(Host, IpAddr)>,
}

impl Hosts {
    pub fn new() -> Hosts {
        Hosts { record: Vec::new() }
    }

    fn push(&mut self, host: Host, ip: IpAddr) {
        self.record.push((host, ip));
    }

    fn extend(&mut self, hosts: Hosts) {
        for item in hosts.record {
            self.record.push(item);
        }
    }

    pub fn iter(&mut self) -> Iter<(Host, IpAddr)> {
        self.record.iter()
    }

    pub fn get(&self, domain: &str) -> Option<&IpAddr> {
        for (reg, ip) in &self.record {
            if reg.is_match(domain) {
                return Some(ip);
            }
        }
        None
    }
}

// domain match
#[derive(Debug)]
pub struct Host(MatchMode);

#[derive(Debug)]
enum MatchMode {
    Text(String),
    Regex(Regex),
}

impl Host {
    fn new(domain: &str) -> result::Result<Host, regex::Error> {
        // example.com
        if Self::is_text(domain) {
            return Ok(Host(MatchMode::Text(domain.to_string())));
        }

        // *.example.com
        if Self::is_wildcard(domain) {
            let s = format!(
                "^{}$",
                domain.replace(".", r"\.").replace("*", r"([a-z]|\d|-)+")
            );
            return Ok(Host(MatchMode::Regex(Regex::new(&s)?)));
        }

        // use regex
        Ok(Host(MatchMode::Regex(Regex::new(domain)?)))
    }

    fn is_text(domain: &str) -> bool {
        const ALLOW: &str = "abcdefghijklmnopqrstuvwxyz0123456789-.";
        domain.chars().all(|item| ALLOW.chars().any(|c| item == c))
    }

    fn is_wildcard(domain: &str) -> bool {
        const ALLOW: &str = "abcdefghijklmnopqrstuvwxyz0123456789-.*";
        domain.chars().all(|item| ALLOW.chars().any(|c| item == c))
    }

    pub fn is_match(&self, domain: &str) -> bool {
        match &self.0 {
            MatchMode::Text(text) => text == domain,
            MatchMode::Regex(reg) => reg.is_match(domain),
        }
    }

    pub fn as_str(&self) -> &str {
        match &self.0 {
            MatchMode::Text(text) => text,
            MatchMode::Regex(reg) => reg.as_str(),
        }
    }
}

#[derive(Debug)]
pub struct ParseConfig {
    pub bind: Vec<SocketAddr>,
    pub proxy: Vec<SocketAddr>,
    pub hosts: Hosts,
    pub timeout: Option<u64>,
    pub invalid: Vec<Invalid>,
}

impl ParseConfig {
    fn new() -> ParseConfig {
        ParseConfig {
            hosts: Hosts::new(),
            bind: Vec::new(),
            proxy: Vec::new(),
            invalid: Vec::new(),
            timeout: None,
        }
    }
    fn extend(&mut self, other: Self) {
        self.bind.extend(other.bind);
        self.proxy.extend(other.proxy);
        self.hosts.extend(other.hosts);
        self.invalid.extend(other.invalid);
        if other.timeout.is_some() {
            self.timeout = other.timeout;
        }
    }
}

#[derive(Debug)]
pub struct Config {
    path: PathBuf,
    file: File,
}

impl Config {
    pub async fn new<P: AsRef<Path>>(path: P) -> Result<Config> {
        let path = path.as_ref();

        if let Some(dir) = path.parent() {
            create_dir_all(dir).await?;
        }

        Ok(Config {
            file: OpenOptions::new()
                .read(true)
                .append(true)
                .create(true)
                .open(path)
                .await?,
            path: path.to_path_buf(),
        })
    }

    async fn read_to_string(&mut self) -> Result<String> {
        let mut content = String::new();
        self.file.read_to_string(&mut content).await?;
        Ok(content)
    }

    pub async fn add(&mut self, domain: &str, ip: &str) -> Result<usize> {
        if self.read_to_string().await?.ends_with('\n') {
            self.file
                .write(format!("{}  {}", domain, ip).as_bytes())
                .await
        } else {
            self.file
                .write(format!("\n{}  {}", domain, ip).as_bytes())
                .await
        }
    }

    pub fn parse(mut self) -> BoxFuture<'static, Result<ParseConfig>> {
        async move {
            let mut parse = ParseConfig::new();

            for (n, line) in self.read_to_string().await?.lines().enumerate() {
                // ignore
                if REG_IGNORE.is_match(&line) {
                    continue;
                }

                // bind
                if let Some(addr) = cap_socket_addr(&REG_BIND, &line) {
                    match addr {
                        Ok(addr) => parse.bind.push(addr),
                        Err(kind) => parse.invalid.push(Invalid {
                            line: n + 1,
                            source: line.to_string(),
                            kind,
                        }),
                    }
                    continue;
                }

                // proxy
                if let Some(addr) = cap_socket_addr(&REG_PROXY, &line) {
                    match addr {
                        Ok(addr) => parse.proxy.push(addr),
                        Err(kind) => parse.invalid.push(Invalid {
                            line: n + 1,
                            source: line.to_string(),
                            kind,
                        }),
                    }
                    continue;
                }

                // timeout
                if let Some(cap) = REG_TIMEOUT.captures(&line) {
                    if let Some(time) = cap.name("val") {
                        if let Ok(t) = time.as_str().parse::<u64>() {
                            parse.timeout = Some(t);
                            continue;
                        }
                    }
                    parse.invalid.push(Invalid {
                        line: n + 1,
                        source: line.to_string(),
                        kind: InvalidType::Timeout,
                    });
                    continue;
                }

                // import
                if let Some(cap) = REG_IMPORT.captures(&line) {
                    if let Some(m) = cap.name("val") {
                        let mut p = Path::new(m.as_str()).to_path_buf();

                        if p.is_relative() {
                            if let Some(parent) = self.path.parent() {
                                p = parent.join(p);
                            }
                        }

                        parse.extend(Config::new(p).await?.parse().await?);
                    } else {
                        // todo
                    }
                    continue;
                }

                // host
                if let Some(d) = cap_ip_addr(&line) {
                    match d {
                        Ok((host, ip)) => parse.hosts.push(host, ip),
                        Err(kind) => parse.invalid.push(Invalid {
                            line: n + 1,
                            source: line.to_string(),
                            kind,
                        }),
                    }
                    continue;
                }

                parse.invalid.push(Invalid {
                    line: n + 1,
                    source: line.to_string(),
                    kind: InvalidType::Other,
                });
            }

            Ok(parse)
        }
            .boxed()
    }
}
