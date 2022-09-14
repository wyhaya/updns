use crate::{config::try_parse_duration, exit, CONFIG_FILE, WATCH_INTERVAL};
use clap::{crate_name, crate_version, App, AppSettings, Arg, SubCommand};
use logs::Logs;
use regex::Regex;
use std::{net::IpAddr, path::PathBuf, time::Duration};

pub enum AppRunType {
    AddRecord {
        path: PathBuf,
        ip: String,
        host: String,
    },
    PrintRecord {
        path: PathBuf,
    },
    EditConfig {
        path: PathBuf,
    },
    PrintPath {
        path: PathBuf,
    },
    Run {
        path: PathBuf,
        duration: Duration,
    },
}

pub fn parse_args() -> AppRunType {
    let app = App::new(crate_name!())
        .version(crate_version!())
        .global_setting(AppSettings::ColoredHelp)
        .setting(AppSettings::VersionlessSubcommands)
        .subcommand(
            SubCommand::with_name("add")
                .about("Add a DNS record")
                .arg(
                    Arg::with_name("host")
                    .value_name("HOST")
                    .required(true)
                        .help("Domain of the DNS record")
                ).arg(
                    Arg::with_name("ip")
                    .value_name("IP")
                    .required(true)
                        .help("IP of the DNS record")
                )
        )
        .subcommand(
            SubCommand::with_name("ls").about("Print all configured DNS records")
        )
        .subcommand(
            SubCommand::with_name("edit").about("Call 'vim' to edit the configuration file")
        )
        .subcommand(
            SubCommand::with_name("path").about("Print related directories")
        )
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .takes_value(true)
                .help("Specify a config file"),
        )
        .arg(
            Arg::with_name("duration")
                .short("d")
                .long("duration")
                .value_name("TIME")
                .takes_value(true)
                .help("Check the interval time of the configuration file\nformat: 1ms, 1s, 1m, 1h, 1d"),
        )
        .arg(
            Arg::with_name("log")
                .short("l")
                .long("log")
                .value_name("...")
                .takes_value(true)
                .default_value("info")
                .help("Set logs enable"),
        )
        .get_matches();

    Logs::new()
        .target(env!("CARGO_PKG_NAME"))
        .level_from_str(app.value_of("log").unwrap())
        .unwrap_or_else(|msg| exit!("Log value error: {:#?}", msg))
        .init();

    let path = match app.value_of("config") {
        Some(s) => PathBuf::from(s),
        None => match dirs::home_dir() {
            Some(p) => p.join(CONFIG_FILE[0]).join(CONFIG_FILE[1]),
            None => exit!("Can't get home directory"),
        },
    };

    let duration = match app.value_of("duration") {
        Some(s) => try_parse_duration(s).unwrap_or_else(|_| {
            exit!(
                "Cannot resolve '{}' to interval time, format: 1ms, 1s, 1m, 1h, 1d",
                s
            )
        }),
        None => WATCH_INTERVAL,
    };

    if let Some(add) = app.subcommand_matches("add") {
        let host = add.value_of("host").unwrap().to_string();
        let ip = add.value_of("ip").unwrap().to_string();
        // check
        if let Err(err) = Regex::new(&host) {
            exit!(
                "Cannot resolve host '{}' to regular expression\n{:?}",
                host,
                err
            );
        }
        if ip.parse::<IpAddr>().is_err() {
            exit!("Cannot resolve '{}' to ip address", ip);
        }
        return AppRunType::AddRecord { path, ip, host };
    }

    if app.is_present("ls") {
        return AppRunType::PrintRecord { path };
    }

    if app.is_present("edit") {
        return AppRunType::EditConfig { path };
    }

    if app.is_present("path") {
        return AppRunType::PrintPath { path };
    }

    AppRunType::Run { path, duration }
}
