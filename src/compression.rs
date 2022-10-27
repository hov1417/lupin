use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use eyre::{bail, Context};
use tar::Builder;
use tokio::task::spawn_blocking;
use zstd::Encoder;

const BUF_SIZE: usize = 5 * 1024 * 1024;

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
            Encoder::new(BufWriter::with_capacity(BUF_SIZE, file), 10)
                .context("could not create zst encoder")?;
        writer.multithread(100)?;
        {
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
        }
        Ok(())
    })
    .await?
}
