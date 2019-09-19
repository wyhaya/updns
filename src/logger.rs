use log::{Level, LevelFilter, Metadata, Record, SetLoggerError};

static LOGGER: Logger = Logger;

pub fn init() -> Result<(), SetLoggerError> {
    log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Trace))
}

struct Logger;

impl log::Log for Logger {
    fn enabled(&self, meta: &Metadata) -> bool {
        meta.level() != Level::Trace
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            println!(
                "{} {}: {}",
                time::now().strftime("[%Y-%m-%d][%H:%M:%S]").unwrap(),
                record.level(),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}
