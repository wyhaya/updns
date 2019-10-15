use regex::Regex;
use std::fs;
use std::fs::File;
use std::io;
use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::slice::Iter;

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

fn cap_socket_addr(reg: &Regex, text: &str) -> Option<Result<SocketAddr, InvalidType>> {
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

fn cap_ip_addr(text: &str) -> Option<Result<(Regex, IpAddr), InvalidType>> {
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
        return match Regex::new(val2) {
            Ok(reg) => Some(Ok((reg, ip))),
            Err(_) => Some(Err(InvalidType::Regex)),
        };
    }

    // domain ip
    let ip = match val2.parse() {
        Ok(ip) => ip,
        Err(_) => return Some(Err(InvalidType::IpAddr)),
    };

    let reg = match Regex::new(val1) {
        Ok(reg) => reg,
        Err(_) => return Some(Err(InvalidType::Regex)),
    };

    return Some(Ok((reg, ip)));
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
    pub fn as_str(&self) -> &str {
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
pub struct ParseConfig {
    pub bind: Vec<SocketAddr>,
    pub proxy: Vec<SocketAddr>,
    pub hosts: Hosts,
    pub timeout: Option<u64>,
    pub invalid: Vec<Invalid>,
}

#[derive(Debug)]
pub struct Config {
    path: PathBuf,
    file: File,
}

impl Config {
    pub fn new<P: AsRef<Path>>(path: P) -> io::Result<Config> {
        let path = path.as_ref();

        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }

        Ok(Config {
            file: fs::OpenOptions::new()
                .read(true)
                .append(true)
                .create(true)
                .open(path)?,
            path: path.to_path_buf(),
        })
    }

    fn read_to_string(&mut self) -> io::Result<String> {
        let mut content = String::new();
        self.file.read_to_string(&mut content)?;
        Ok(content)
    }

    pub fn add(&mut self, domain: &str, ip: &str) -> io::Result<()> {
        if self.read_to_string()?.ends_with("\n") {
            writeln!(self.file, "{}  {}", domain, ip)
        } else {
            writeln!(self.file, "\n{}  {}", domain, ip)
        }
    }

    pub fn parse(&mut self) -> io::Result<ParseConfig> {
        let mut hosts = Hosts::new();
        let mut bind = Vec::new();
        let mut proxy = Vec::new();
        let mut invalid = Vec::new();
        let mut timeout = None;

        for (n, line) in self.read_to_string()?.lines().enumerate() {
            // ignore
            if REG_IGNORE.is_match(&line) {
                continue;
            }

            // bind
            if let Some(addr) = cap_socket_addr(&REG_BIND, &line) {
                match addr {
                    Ok(addr) => bind.push(addr),
                    Err(kind) => {
                        invalid.push(Invalid {
                            line: n + 1,
                            source: line.to_string(),
                            kind,
                        });
                    }
                }
                continue;
            }

            // proxy
            if let Some(addr) = cap_socket_addr(&REG_PROXY, &line) {
                match addr {
                    Ok(addr) => proxy.push(addr),
                    Err(kind) => {
                        invalid.push(Invalid {
                            line: n + 1,
                            source: line.to_string(),
                            kind,
                        });
                    }
                }
                continue;
            }

            // timeout
            if let Some(cap) = REG_TIMEOUT.captures(&line) {
                if let Some(time) = cap.name("val") {
                    if let Ok(t) = time.as_str().parse::<u64>() {
                        timeout = Some(t);
                        continue;
                    }
                }
                invalid.push(Invalid {
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

                    let config = Config::new(p)?.parse()?;
                    bind.extend(config.bind);
                    proxy.extend(config.proxy);
                    hosts.extend(config.hosts);
                    invalid.extend(config.invalid);
                } else {
                    // todo
                }
                continue;
            }

            // host
            if let Some(d) = cap_ip_addr(&line) {
                match d {
                    Ok((domain, ip)) => hosts.push(domain, ip),
                    Err(kind) => {
                        invalid.push(Invalid {
                            line: n + 1,
                            source: line.to_string(),
                            kind,
                        });
                    }
                }
                continue;
            }

            invalid.push(Invalid {
                line: n + 1,
                source: line.to_string(),
                kind: InvalidType::Other,
            });
        }

        Ok(ParseConfig {
            bind,
            proxy,
            hosts,
            timeout,
            invalid,
        })
    }
}

#[derive(Debug)]
pub struct Hosts {
    record: Vec<(Regex, IpAddr)>,
}

impl Hosts {
    pub fn new() -> Hosts {
        Hosts { record: Vec::new() }
    }

    fn push(&mut self, domain: Regex, ip: IpAddr) {
        self.record.push((domain, ip));
    }

    fn extend(&mut self, hosts: Hosts) {
        for item in hosts.record {
            self.record.push(item);
        }
    }

    pub fn iter(&mut self) -> Iter<(Regex, IpAddr)> {
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
