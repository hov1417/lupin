use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use async_compression::tokio::write::ZstdEncoder;
use eyre::{bail, Context};
use tar::Builder;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use tokio::task::spawn_blocking;
use tracing::{error, warn};
use zstd::Encoder;

pub async fn compress_directory(
    src_dir: PathBuf,
    out_file: PathBuf,
) -> eyre::Result<()> {
    spawn_blocking(move || {
        if !src_dir.is_dir() {
            bail!("Source with path {:?} is not a directory", src_dir)
        }
        let file = File::create(&out_file)
            .with_context(|| format!("could not create file {:?}", out_file))?;
        let mut writer =
            Encoder::new(file, 10).context("could not create zst encoder")?;
        writer.multithread(100)?;
        let mut tar_builder = Builder::new(writer);
        tar_builder
            .append_dir_all(".", src_dir)
            .context("could not create archive")?;
        let writer =
            tar_builder.into_inner().context("error writing archive")?;
        writer
            .finish()
            .context("could not compress archive")?
            .flush()
            .context("could not flush buffer")?;
        Ok(())
    })
    .await?
}

pub struct Compressor {
    writer: ZstdEncoder<tokio::fs::File>,
    closed: bool,
}

impl Debug for Compressor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Compressor")
            .field("closed", &self.closed)
            .finish()
    }
}

impl Compressor {
    #[tracing::instrument]
    pub async fn new(out_file: &Path) -> eyre::Result<Self> {
        let file = tokio::fs::File::create(out_file)
            .await
            .context("could not create file")?;
        let writer = ZstdEncoder::with_quality_and_params(
            file,
            async_compression::Level::Precise(10),
            &[async_compression::zstd::CParameter::nb_workers(100)],
        );
        Ok(Self {
            writer,
            closed: false,
        })
    }

    #[tracing::instrument]
    pub async fn add_file(
        &mut self,
        name: &str,
        path: &Path,
    ) -> eyre::Result<()> {
        let metadata = tokio::fs::metadata(path)
            .await
            .context("Cannot open the file")?;

        self.write_tar_header(name, metadata.len())
            .await
            .context("could not write header")?;
        let mut file = tokio::fs::File::open(path)
            .await
            .context("Cannot open file")?;
        tokio::io::copy(&mut file, &mut self.writer)
            .await
            .context("Cannot copy file")?;

        self.write_tar_footer(metadata.len())
            .await
            .context("could not write header")?;

        Ok(())
    }

    #[tracing::instrument]
    pub async fn add_file_with_data(
        &mut self,
        name: &str,
        data: &[u8],
    ) -> eyre::Result<()> {
        self.write_tar_header(name, data.len() as u64)
            .await
            .context("could not write header")?;

        self.writer
            .write_all(data)
            .await
            .context("could not write file")?;

        Ok(())
    }

    pub async fn add_chunked_file(
        this: Arc<Mutex<Self>>,
        name: &str,
        size: u64,
    ) -> eyre::Result<ChunkedCompressor> {
        this.lock()
            .await
            .write_tar_header(name, size)
            .await
            .context("could not write header")?;

        Ok(ChunkedCompressor {
            compressor: this,
            remaining: size,
        })
    }

    pub async fn finish(&mut self) -> eyre::Result<()> {
        self.finish_tar_archive().await?;
        self.writer
            .flush()
            .await
            .context("could not flush buffer")?;
        self.writer
            .shutdown()
            .await
            .context("could not compress archive")?;

        self.closed = true;
        Ok(())
    }

    async fn write_tar_header(
        &mut self,
        path: &str,
        data_len: u64,
    ) -> eyre::Result<()> {
        let mut header = tar::Header::new_gnu();
        header
            .set_path(&path)
            .context("could not set path for file")?;
        header.set_mode(0o644);
        header.set_mtime(get_unix_epoch());
        header.set_size(data_len);
        header.set_cksum();

        self.writer
            .write_all(header.as_bytes())
            .await
            .context("could not write header")?;
        Ok(())
    }

    async fn write_tar_footer(&mut self, data_len: u64) -> eyre::Result<()> {
        // Pad with zeros if necessary.
        let buf = [0; 512];
        let remaining = 512 - (data_len % 512);
        if remaining < 512 {
            self.writer
                .write_all(&buf[..remaining as usize])
                .await
                .context("could not write file")?;
        }

        Ok(())
    }

    async fn finish_tar_archive(&mut self) -> eyre::Result<()> {
        self.writer
            .write_all(&[0; 1024])
            .await
            .context("could not write file")
    }
}

impl Drop for Compressor {
    fn drop(&mut self) {
        if !self.closed {
            error!("Compressor was not closed");
        }
    }
}

pub struct ChunkedCompressor {
    compressor: Arc<Mutex<Compressor>>,
    remaining: u64,
}

impl ChunkedCompressor {
    pub async fn add_chunk(&mut self, data: &[u8]) -> eyre::Result<()> {
        self.compressor
            .lock()
            .await
            .writer
            .write_all(data)
            .await
            .context("could not write file")?;
        self.remaining -= data.len() as u64;
        Ok(())
    }
}

impl Drop for ChunkedCompressor {
    fn drop(&mut self) {
        if self.remaining > 0 {
            error!("ChunkedCompressor was not closed");
        }
    }
}

pub async fn compress_files(
    files: &[(String, Vec<u8>)],
    out_file: &Path,
) -> eyre::Result<()> {
    let mut compressor = Compressor::new(out_file).await?;
    for (path, data) in files {
        compressor.add_file_with_data(path, data).await?;
    }
    compressor.finish().await?;
    Ok(())
}

fn get_unix_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| {
            warn!("Time went backwards");
            Duration::new(0, 0)
        })
        .as_secs()
}
