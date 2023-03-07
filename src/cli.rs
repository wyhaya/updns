use crate::{exit, CONFIG_FILE};
use clap::{crate_name, crate_version, Arg, Command};
use logs::Logs;
use regex::Regex;
use std::{net::IpAddr, path::PathBuf};

pub struct Args {
    pub path: PathBuf,
    pub run: RunType,
}

pub enum RunType {
    Start,
    AddRecord { ip: String, host: String },
    PrintRecord,
    EditConfig,
    PrintPath,
}

pub fn parse_args() -> Args {
    let matches = Command::new(crate_name!())
        .version(crate_version!())
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .takes_value(true)
                .help("Specify a config file"),
        )
        .arg(
            Arg::with_name("log")
                .short('l')
                .long("log")
                .value_name("LEVEL")
                .takes_value(true)
                .possible_values(["trace", "debug", "info", "warn", "error", "off"])
                .default_value("info")
                .help("Set log level"),
        )
        .subcommand(
            Command::new("add")
                .about("Add a DNS record")
                .arg(
                    Arg::with_name("host")
                        .value_name("HOST")
                        .required(true)
                        .help("Domain of the DNS record"),
                )
                .arg(
                    Arg::with_name("ip")
                        .value_name("IP")
                        .required(true)
                        .help("IP of the DNS record"),
                ),
        )
        .subcommand(Command::new("ls").about("Print all configured DNS records"))
        .subcommand(Command::new("edit").about("Call 'vim' to edit the configuration file"))
        .subcommand(Command::new("path").about("Print related directories"))
        .get_matches();

    let level = matches.value_of("log").unwrap();

    Logs::new()
        .target(crate_name!())
        .level_from_str(level)
        .unwrap()
        .init();

    let path = match matches.value_of("config") {
        Some(s) => PathBuf::from(s),
        None => match dirs::home_dir() {
            Some(p) => p.join(CONFIG_FILE[0]).join(CONFIG_FILE[1]),
            None => exit!("Can't get home directory"),
        },
    };

    match matches.subcommand() {
        None => Args {
            path,
            run: RunType::Start,
        },
        Some(("add", matches)) => {
            let host = matches.value_of("host").unwrap();
            let ip = matches.value_of("ip").unwrap();
            // check
            if let Err(err) = Regex::new(host) {
                exit!(
                    "Cannot resolve host '{}' to regular expression\n{:?}",
                    host,
                    err
                );
            }
            if ip.parse::<IpAddr>().is_err() {
                exit!("Cannot resolve '{}' to ip address", ip);
            }
            Args {
                path,
                run: RunType::AddRecord {
                    ip: ip.to_string(),
                    host: host.to_string(),
                },
            }
        }
        Some(("ls", _)) => Args {
            path,
            run: RunType::PrintRecord,
        },
        Some(("edit", _)) => Args {
            path,
            run: RunType::EditConfig,
        },
        Some(("path", _)) => Args {
            path,
            run: RunType::PrintPath,
        },
        _ => unreachable!(),
    }
}
