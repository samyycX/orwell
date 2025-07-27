use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::Mutex;

use lazy_static::lazy_static;
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::{
    fmt::{format::Writer, time::FormatTime, FmtContext, FormatEvent, FormatFields},
    layer::{Layer, SubscriberExt},
    registry::LookupSpan,
    EnvFilter, Registry,
};

lazy_static! {
    static ref LOG_FILE: Mutex<File> = Mutex::new(
        OpenOptions::new()
            .create(true)
            .append(true)
            .truncate(false)
            .open("latest.log")
            .expect("Failed to open log file")
    );
}

struct LocalTime;

impl FormatTime for LocalTime {
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        let now = chrono::Local::now();
        write!(w, "{}", now.format("%Y-%m-%d %H:%M:%S%.3f"))
    }
}

struct FileFormatter;

impl<S, N> FormatEvent<S, N> for FileFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> std::fmt::Result {
        let metadata = event.metadata();

        // 时间戳
        LocalTime.format_time(&mut writer)?;
        write!(writer, " ")?;

        // 日志级别
        let level = metadata.level();
        let level_str = match *level {
            Level::ERROR => "ERROR",
            Level::WARN => "WARN",
            Level::INFO => "INFO",
            Level::DEBUG => "DEBUG",
            Level::TRACE => "TRACE",
        };
        write!(writer, "[{}] ", level_str)?;

        // 目标模块
        if let Some(target) = metadata.target() {
            write!(writer, "{}: ", target)?;
        }

        // 日志内容
        ctx.format_fields(writer.by_ref(), event)?;

        writeln!(writer)
    }
}

pub fn init_logger() {
    // 创建文件日志层
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(|| LogFileWriter)
        .event_format(FileFormatter)
        .with_ansi(false)
        .with_filter(
            EnvFilter::from_default_env()
                .add_directive("orwell_client=debug".parse().unwrap())
                .add_directive("debug".parse().unwrap()),
        );

    // 创建控制台日志层
    let console_layer = tracing_subscriber::fmt::layer()
        .with_timer(LocalTime)
        .with_target(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_ansi(true)
        .with_filter(
            EnvFilter::from_default_env()
                .add_directive("orwell_client=info".parse().unwrap())
                .add_directive("info".parse().unwrap()),
        );

    // 注册日志订阅器
    let subscriber = Registry::default().with(file_layer).with(console_layer);

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global default subscriber");
}

struct LogFileWriter;

impl Write for LogFileWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut file = LOG_FILE.lock().unwrap();
        file.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut file = LOG_FILE.lock().unwrap();
        file.flush()
    }
}
