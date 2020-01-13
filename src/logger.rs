use log::{LevelFilter, Metadata, Record, SetLoggerError};

static LOGGER: Logger = Logger;

pub fn init() -> Result<(), SetLoggerError> {
    log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Trace))
}

struct Logger;

impl log::Log for Logger {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        println!(
            "[{}] {:<5} {}",
            time::now().strftime("%F %T").unwrap(),
            record.level(),
            record.args()
        );
    }

    fn flush(&self) {}
}
