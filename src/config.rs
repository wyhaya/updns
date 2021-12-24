use crate::matcher::Matcher;
use futures_util::future::{BoxFuture, FutureExt};
use logs::error;
use std::{
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
    result,
    slice::Iter,
    time::Duration,
};
use tokio::{
    fs,
    fs::{File, OpenOptions},
    io::{AsyncReadExt, AsyncWriteExt, Result},
};

// Parse time format into Duration
pub fn try_parse_duration(text: &str) -> result::Result<Duration, ()> {
    let numbers = "0123456789.".chars().collect::<Vec<char>>();
    let i = text
        .chars()
        .position(|ch| !numbers.contains(&ch))
        .ok_or(())?;

    let (time, unit) = text.split_at(i);
    if time.is_empty() {
        return Err(());
    }
    let n = time.parse::<f32>().map_err(|_| ())?;
    let ms = match unit {
        "d" => Ok(24. * 60. * 60. * 1000. * n),
        "h" => Ok(60. * 60. * 1000. * n),
        "m" => Ok(60. * 1000. * n),
        "s" => Ok(1000. * n),
        "ms" => Ok(n),
        _ => Err(()),
    }? as u64;

    if ms == 0 {
        Err(())
    } else {
        Ok(Duration::from_millis(ms))
    }
}

#[derive(Debug)]
pub struct Invalid {
    pub line: usize,
    pub source: String,
    pub kind: InvalidType,
}

pub trait MultipleInvalid {
    fn print(&self);
}

impl MultipleInvalid for Vec<Invalid> {
    fn print(&self) {
        for invalid in self {
            error!(
                "[line:{}] {} `{}`",
                invalid.line,
                invalid.kind.description(),
                invalid.source
            );
        }
    }
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
    pub fn description(&self) -> &str {
        match self {
            InvalidType::SocketAddr => "Cannot parse socket address",
            InvalidType::IpAddr => "Cannot parse ip address",
            InvalidType::Regex => "Cannot parse regular expression",
            InvalidType::Timeout => "Cannot parse timeout",
            InvalidType::Other => "Invalid line",
        }
    }
}

#[derive(Debug)]
pub struct Hosts {
    record: Vec<(Matcher, IpAddr)>,
}

impl Hosts {
    pub fn new() -> Hosts {
        Hosts { record: Vec::new() }
    }

    fn push(&mut self, record: (Matcher, IpAddr)) {
        self.record.push(record);
    }

    fn extend(&mut self, hosts: Hosts) {
        self.record.extend(hosts.record);
    }

    pub fn iter(&mut self) -> Iter<(Matcher, IpAddr)> {
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

#[derive(Debug)]
pub struct Config {
    pub bind: Vec<SocketAddr>,
    pub proxy: Vec<SocketAddr>,
    pub hosts: Hosts,
    pub timeout: Option<Duration>,
    pub invalid: Vec<Invalid>,
}

impl Config {
    fn new() -> Config {
        Config {
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
pub struct Parser {
    path: PathBuf,
    file: File,
}

impl Parser {
    pub async fn new<P: AsRef<Path>>(path: P) -> Result<Parser> {
        let path = path.as_ref();

        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).await?;
        }

        Ok(Parser {
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

    fn split(text: &str) -> Option<(&str, &str)> {
        let mut text = text.split_ascii_whitespace();

        if let (Some(left), Some(right)) = (text.next(), text.next()) {
            if text.next().is_none() {
                return Some((left, right));
            }
        }

        None
    }

    // match host
    // example.com 0.0.0.0  or  0.0.0.0 example.com
    fn record(left: &str, right: &str) -> result::Result<(Matcher, IpAddr), InvalidType> {
        // ip domain
        if let Ok(ip) = right.parse() {
            return Matcher::new(left)
                .map(|host| (host, ip))
                .map_err(|_| InvalidType::Regex);
        }

        // domain ip
        if let Ok(ip) = left.parse() {
            return Matcher::new(right)
                .map(|host| (host, ip))
                .map_err(|_| InvalidType::Regex);
        }

        Err(InvalidType::IpAddr)
    }

    pub fn parse(mut self) -> BoxFuture<'static, Result<Config>> {
        async move {
            let content = self.read_to_string().await?;
            let mut config = Config::new();

            for (i, mut line) in content.lines().enumerate() {
                if line.is_empty() {
                    continue;
                }
                // remove comment
                // example # ... -> example
                if let Some(pos) = line.find('#') {
                    line = &line[0..pos];
                }

                line = line.trim();

                if line.is_empty() {
                    continue;
                }

                macro_rules! invalid {
                    ($type: expr) => {{
                        config.invalid.push(Invalid {
                            line: i + 1,
                            source: line.to_string(),
                            kind: $type,
                        });
                        continue;
                    }};
                }

                let (key, value) = match Self::split(line) {
                    Some(d) => d,
                    None => invalid!(InvalidType::Other),
                };

                match key {
                    "bind" => match value.parse::<SocketAddr>() {
                        Ok(addr) => config.bind.push(addr),
                        Err(_) => invalid!(InvalidType::SocketAddr),
                    },
                    "proxy" => match value.parse::<SocketAddr>() {
                        Ok(addr) => config.proxy.push(addr),
                        Err(_) => invalid!(InvalidType::SocketAddr),
                    },
                    "timeout" => match try_parse_duration(value) {
                        Ok(timeout) => config.timeout = Some(timeout),
                        Err(_) => invalid!(InvalidType::Timeout),
                    },
                    "import" => {
                        let mut path = PathBuf::from(value);
                        if path.is_relative() {
                            if let Some(parent) = self.path.parent() {
                                path = parent.join(path);
                            }
                        }
                        config.extend(Parser::new(path).await?.parse().await?);
                    }
                    _ => match Self::record(key, value) {
                        Ok(record) => config.hosts.push(record),
                        Err(kind) => invalid!(kind),
                    },
                }
            }

            Ok(config)
        }
        .boxed()
    }
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, Ipv6Addr};

    use super::*;

    #[tokio::test]
    async fn parse_config() -> Result<()> {
        let config_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("test_data")
            .join("config");

        let parser = Parser::new(config_path).await?;

        let config = parser.parse().await?;

        assert!(config.invalid.is_empty(), "{:?}", config.invalid);

        assert_eq!(
            config.bind,
            vec![SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 53)]
        );

        assert_eq!(
            config.proxy,
            vec![SocketAddr::new(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)), 53)]
        );

        let ip_addresses: Vec<_> = config.hosts.record.iter().map(|(_, ip)| *ip).collect();
        assert_eq!(
            ip_addresses,
            vec![
                IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)),
                IpAddr::V4(Ipv4Addr::new(2, 2, 2, 2)),
                IpAddr::V4(Ipv4Addr::new(3, 3, 3, 3)),
                IpAddr::V6(Ipv6Addr::UNSPECIFIED),
                IpAddr::V4(Ipv4Addr::new(4, 4, 4, 4)),
            ]
        );

        assert_eq!(config.timeout, Some(Duration::from_secs(2)));

        Ok(())
    }
}
