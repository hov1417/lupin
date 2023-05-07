use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use async_compression::tokio::write::ZstdEncoder;
use eyre::{bail, Context};
use tar::Builder;
use tokio::io::AsyncWriteExt;
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

impl Compressor {
    pub async fn new(out_file: &Path) -> eyre::Result<Self> {
        let file = tokio::fs::File::create(out_file)
            .await
            .with_context(|| format!("could not create file {:?}", out_file))?;
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

    pub async fn add_file(
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

pub async fn compress_files(
    files: &[(String, Vec<u8>)],
    out_file: &Path,
) -> eyre::Result<()> {
    let mut compressor = Compressor::new(out_file).await?;
    for (path, data) in files {
        compressor.add_file(path, data).await?;
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
