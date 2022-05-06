use log::{Log, Metadata, Record};

pub struct Logger {}

impl Logger {
    pub const fn new() -> Self {
        Self {}
    }
}

impl Log for Logger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    // todo: this is a work-around for the jlink_rtt crate
    //       currently not supporting non-blocking writes
    #[cfg(debug_assertions)]
    fn log(&self, record: &Record) {
        use log::Level;
        use rtt_target::rprint;
        if !self.enabled(record.metadata()) {
            return;
        }
        match record.level() {
            Level::Error => rprint!("[ERROR "),
            Level::Warn => rprint!("[WARN  "),
            Level::Info => rprint!("[INFO  "),
            Level::Debug => rprint!("[DEBUG "),
            Level::Trace => rprint!("[TRACE "),
        };
        rprint!(record.target());
        rprint!("] ");
        rprint!("{}", *record.args());
        rprint!("\n");
    }

    #[cfg(not(debug_assertions))]
    fn log(&self, _record: &Record) {}

    fn flush(&self) {}
}

use log::LevelFilter;

#[macro_export]
#[cfg(debug_assertions)]
macro_rules! rtt_init_logger {
    ($mode:ident) => {
        use log::LevelFilter;
        use $crate::logger::Logger;
        static mut LOGGER_INSTANCE: Logger = Logger::new();
        ::rtt_target::rtt_init_print!($mode);
        unsafe {
            log::set_logger(&LOGGER_INSTANCE).unwrap();
        }
        log::set_max_level(LevelFilter::Trace);
    };

    () => {
        $crate::rtt_init_logger!(NoBlockSkip);
    };
}

/// Don't create a SEGGER_RTT section in release
#[macro_export]
#[cfg(not(debug_assertions))]
macro_rules! rtt_init_logger {
    ($mode:ident) => {};
    () => {};
}

pub fn set_level(level: LevelFilter) {
    log::set_max_level(level);
}

