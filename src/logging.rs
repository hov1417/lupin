use std::io;

use eyre::Context;
use indicatif::MultiProgress;
use tracing::level_filters::LevelFilter;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_error::ErrorLayer;
use tracing_subscriber::fmt::{layer, MakeWriter};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub struct LoggerGuard(WorkerGuard);

pub async fn init_trace_logger(
    mpb: MultiProgress,
    verbose: u8,
    quite: u8,
) -> eyre::Result<LoggerGuard> {
    let (non_blocking, guard) = tracing_appender::non_blocking(io::stdout());

    let verbosity = 1 - (quite as i8) + (verbose as i8);
    let fmt_layer = layer()
        .compact()
        .with_writer(WriterWrapper::new(mpb.clone(), non_blocking));

    tracing_subscriber::registry()
        .with(ErrorLayer::default())
        .with(level_filter_from_number(verbosity))
        .with(fmt_layer)
        .try_init()
        .context("Could not init logger")?;
    Ok(LoggerGuard(guard))
}

fn level_filter_from_number(verbosity: i8) -> LevelFilter {
    let level = match verbosity {
        i8::MIN..=-1 => None,
        0 => Some(tracing::Level::ERROR),
        1 => Some(tracing::Level::WARN),
        2 => Some(tracing::Level::INFO),
        3 => Some(tracing::Level::DEBUG),
        4..=i8::MAX => Some(tracing::Level::TRACE),
    };
    LevelFilter::from(level)
}

trait Suspender {
    fn suspend<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R;
}

impl Suspender for MultiProgress {
    fn suspend<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        self.suspend(f)
    }
}

#[derive(Clone)]
struct WriterWrapper<S, W>
where
    S: Suspender,
    W: io::Write,
{
    suspender: S,
    inner: W,
}

impl<S, W> WriterWrapper<S, W>
where
    S: Suspender,
    W: io::Write,
{
    fn new(suspender: S, inner: W) -> WriterWrapper<S, W> {
        WriterWrapper { suspender, inner }
    }
}

impl<F, W> MakeWriter<'_> for WriterWrapper<F, W>
where
    F: Suspender + Clone,
    W: io::Write + Clone,
{
    type Writer = Self;

    fn make_writer(&self) -> Self::Writer {
        self.clone()
    }
}

impl<F, W> io::Write for WriterWrapper<F, W>
where
    F: Suspender + Clone,
    W: io::Write + Clone,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.suspender.suspend(|| {
            let ret = self.inner.write(buf);
            self.inner.flush()?;
            ret
        })
    }

    fn write_vectored(
        &mut self,
        bufs: &[io::IoSlice<'_>],
    ) -> io::Result<usize> {
        self.suspender.suspend(|| {
            let ret = self.inner.write_vectored(bufs);
            self.inner.flush()?;
            ret
        })
    }

    fn flush(&mut self) -> io::Result<()> {
        self.suspender.suspend(|| self.inner.flush())
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.suspender.suspend(|| {
            let ret = self.inner.write_all(buf);
            self.inner.flush()?;
            ret
        })
    }

    fn write_fmt(&mut self, fmt: std::fmt::Arguments<'_>) -> io::Result<()> {
        self.suspender.suspend(|| {
            let ret = self.inner.write_fmt(fmt);
            self.inner.flush()?;
            ret
        })
    }
}
