use flexi_logger::{
    detailed_format, Cleanup, Criterion, Duplicate, FileSpec, FlexiLoggerError, LogSpecification,
    Logger, LoggerHandle, Naming, WriteMode,
};
use std::backtrace::Backtrace;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Initializes ordinary `log` logging. Keep the returned handle alive until exit.
pub fn init(directory: &Path) -> Result<LoggerHandle, FlexiLoggerError> {
    let panic_path = directory.join("n_music_panic.log");
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let thread = std::thread::current();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let report = format!(
            "\n=== n_music {} panic; {}/{}; unix_ms={timestamp} ===\nThread: {} ({:?})\n{info}\n{}\n",
            env!("CARGO_PKG_VERSION"),
            std::env::consts::OS,
            std::env::consts::ARCH,
            thread.name().unwrap_or("unnamed"),
            thread.id(),
            Backtrace::force_capture(),
        );
        // Do not call the logger here: the panicking thread may already hold its lock.
        let saved = (|| -> io::Result<()> {
            let mut options = OpenOptions::new();
            options.create(true).append(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options.open(&panic_path)?;
            file.write_all(report.as_bytes())?;
            file.sync_data()
        })();
        if let Err(error) = saved {
            let _ = writeln!(
                io::stderr(),
                "Cannot save panic report to {}: {error}\n{report}",
                panic_path.display()
            );
        }
        previous_hook(info);
    }));

    let spec = LogSpecification::parse(
        std::env::var("N_MUSIC_LOG").unwrap_or_else(|_| "info,zbus=warn".into()),
    )?;
    let logger = || {
        Logger::with(spec.clone())
            .format(detailed_format)
            .use_utc()
            .write_mode(WriteMode::Direct)
            .panic_if_error_channel_is_broken(false)
    };
    let handle = logger()
        .log_to_file(FileSpec::default().directory(directory).basename("n_music"))
        .append()
        .duplicate_to_stderr(Duplicate::All)
        .rotate(
            Criterion::Size(5 * 1024 * 1024),
            Naming::Numbers,
            Cleanup::KeepLogFiles(1),
        )
        .cleanup_in_background_thread(false)
        .start()
        .or_else(|error| {
            let handle = logger().log_to_stderr().start()?;
            log::error!(
                "Could not initialize file logging in {}: {error}; using stderr",
                directory.display()
            );
            Ok::<_, FlexiLoggerError>(handle)
        })?;
    log::info!(
        "n_music {} starting on {}/{}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    Ok(handle)
}
