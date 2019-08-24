use regex::Regex;
use std::fs::File;
use std::io;
use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use std::slice::Iter;

lazy_static! {
    static ref REG_IGNORE: Regex = Regex::new(r#"^\s*(#.*)?$"#).unwrap();
    static ref REG_BIND: Regex = Regex::new(r#"^\s*bind\s+(?P<val>[^\s#]+)"#).unwrap();
    static ref REG_PROXY: Regex = Regex::new(r#"^\s*proxy\s+(?P<val>[^\s#]+)"#).unwrap();
    // todo
    // The path will also contain '#' and ' '
    static ref REG_IMPORT: Regex = Regex::new(r#"\s*import\s+(?P<val>(/.*))"#).unwrap();
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
pub struct Config {
    file: File,
    content: String,
}

#[derive(Debug)]
pub struct Invalid {
    pub line: usize,
    pub source: String,
    pub err: InvalidType,
}

#[derive(Debug)]
pub enum InvalidType {
    Regex,
    SocketAddr,
    IpAddr,
    Other,
}

impl Config {
    pub fn new<P: AsRef<Path>>(path: P) -> io::Result<Config> {
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(path)?;

        let mut content = String::new();
        file.read_to_string(&mut content)?;

        Ok(Config { file, content })
    }

    pub fn add(&mut self, domain: &str, ip: &str) -> std::io::Result<()> {
        if self.content.ends_with("\n") {
            writeln!(self.file, "{}  {}", domain, ip)
        } else {
            writeln!(self.file, "\n{}  {}", domain, ip)
        }
    }

    pub fn parse(&mut self) -> io::Result<(Vec<SocketAddr>, Vec<SocketAddr>, Hosts, Vec<Invalid>)> {
        let mut hosts = Hosts::new();
        let mut binds = Vec::new();
        let mut proxy = Vec::new();
        let mut errors = Vec::new();

        for (n, line) in self.content.lines().enumerate() {
            // ignore
            if REG_IGNORE.is_match(&line) {
                continue;
            }

            // bind
            if let Some(addr) = cap_socket_addr(&REG_BIND, &line) {
                match addr {
                    Ok(addr) => binds.push(addr),
                    Err(err) => {
                        errors.push(Invalid {
                            line: n + 1,
                            source: line.to_string(),
                            err,
                        });
                    }
                }
                continue;
            }

            // proxy
            if let Some(addr) = cap_socket_addr(&REG_PROXY, &line) {
                match addr {
                    Ok(addr) => proxy.push(addr),
                    Err(err) => {
                        errors.push(Invalid {
                            line: n + 1,
                            source: line.to_string(),
                            err,
                        });
                    }
                }
                continue;
            }

            // import
            if let Some(cap) = REG_IMPORT.captures(&line) {
                if let Some(m) = cap.name("val") {
                    let (b, p, h, e) = Config::new(m.as_str())?.parse()?;
                    binds.extend(b);
                    proxy.extend(p);
                    hosts.extend(h);
                    errors.extend(e);
                } else {
                    // todo
                }
                continue;
            }

            // host
            if let Some(d) = cap_ip_addr(&line) {
                match d {
                    Ok((domain, ip)) => hosts.push(domain, ip),
                    Err(err) => {
                        errors.push(Invalid {
                            line: n + 1,
                            source: line.to_string(),
                            err,
                        });
                    }
                }
                continue;
            }

            errors.push(Invalid {
                line: n + 1,
                source: line.to_string(),
                err: InvalidType::Other,
            });
        }

        Ok((binds, proxy, hosts, errors))
    }
}

#[derive(Debug)]
pub struct Hosts {
    list: Vec<(Regex, IpAddr)>,
}

impl Hosts {
    pub fn new() -> Hosts {
        Hosts { list: Vec::new() }
    }

    fn push(&mut self, domain: Regex, ip: IpAddr) {
        self.list.push((domain, ip));
    }

    fn extend(&mut self, hosts: Hosts) {
        for item in hosts.list {
            self.list.push(item);
        }
    }

    pub fn iter(&mut self) -> Iter<(Regex, IpAddr)> {
        self.list.iter()
    }

    pub fn get(&self, domain: &str) -> Option<&IpAddr> {
        for (reg, ip) in &self.list {
            if reg.is_match(domain) {
                return Some(ip);
            }
        }
        None
    }
}
