#![cfg(test)]

use std::env::temp_dir;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use criterion::{Criterion, criterion_group, criterion_main};
use eyre::Context;
use once_cell::sync::Lazy;
use rand::{Rng, thread_rng};
use rand::distributions::Alphanumeric;
use zstd::Encoder;

use lupin::compression::compress_files;

static FILES: Lazy<Arc<[(String, Vec<u8>)]>> =
    Lazy::new(|| get_files(100, 1_000_000).into());

pub fn finish_archive(writer: &mut impl Write) -> eyre::Result<()> {
    writer.write_all(&[0; 1024]).context("could not write file")
}

pub fn write_header(
    writer: &mut impl Write,
    path: &String,
    data: &Vec<u8>,
) -> eyre::Result<()> {
    let mut header = tar::Header::new_gnu();
    header
        .set_path(&path)
        .context("could not set path for file")?;
    header.set_mode(0o644);
    header.set_size(data.len() as u64);
    header.set_cksum();

    writer
        .write_all(header.as_bytes())
        .context("could not write header")?;

    // Pad with zeros if necessary.
    let buf = [0; 512];
    let remaining = 512 - (data.len() % 512);
    if remaining < 512 {
        writer
            .write_all(&buf[..remaining as usize])
            .context("could not write file")?;
    }

    Ok(())
}

pub fn blocking_compress_files(
    files: &[(String, Vec<u8>)],
    out_file: &Path,
) -> eyre::Result<()> {
    let file = File::create(out_file)
        .with_context(|| format!("could not create file {:?}", out_file))?;

    let mut writer =
        Encoder::new(file, 10).context("could not create zst encoder")?;
    writer.multithread(100)?;
    for (path, data) in files {
        write_header(&mut writer, path, data)
            .context("could not write header")?;

        writer.write_all(data).context("could not write file")?;
    }
    finish_archive(&mut writer)?;
    writer
        .finish()
        .context("could not compress archive")?
        .flush()
        .context("could not flush buffer")?;
    Ok(())
}

fn random_text(len: usize) -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

fn get_files(file: usize, size: usize) -> Vec<(String, Vec<u8>)> {
    let mut files = Vec::new();
    for _ in 0..file {
        files.push((random_text(10), random_text(size).into_bytes()));
    }
    files
}

fn benchmark_blocking_compress_files(c: &mut Criterion) {
    let out_file = temp_dir().join("benchmark_blocking_compress_files.tar.zst");
    c.bench_function("blocking_compress_files 100 files by one mb", |b| {
        b.iter(|| blocking_compress_files(FILES.as_ref(), &out_file).unwrap())
    });
}

fn benchmark_compress_files(c: &mut Criterion) {
    let out_file = temp_dir().join("benchmark_compress_files.tar.zst");
    c.bench_function("compress_files 100 files by one mb", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| compress_files(FILES.as_ref(), &out_file))
    });
}

async fn blocking_compress_files_in_async(
    files: Arc<[(String, Vec<u8>)]>,
    out_file: Arc<PathBuf>,
) {
    tokio::task::spawn_blocking(move || {
        blocking_compress_files(files.as_ref(), out_file.as_ref()).unwrap();
    })
    .await
    .unwrap()
}

fn benchmark_blocking_compress_files_in_async(c: &mut Criterion) {
    let out_file = Arc::new(
        temp_dir().join("benchmark_blocking_compress_files_in_async.tar.zst"),
    );
    c.bench_function(
        "blocking_compress_files_in_async 100 files by one mb",
        |b| {
            b.to_async(tokio::runtime::Runtime::new().unwrap())
                .iter(|| {
                    blocking_compress_files_in_async(
                        Arc::clone(&FILES),
                        Arc::clone(&out_file),
                    )
                })
        },
    );
}

criterion_group!(
    name = bench_blocking_compress_files;
    config = Criterion::default().sample_size(10);
    targets = benchmark_blocking_compress_files
);
criterion_group!(
    name = bench_compress_files;
    config = Criterion::default().sample_size(10);
    targets = benchmark_compress_files
);
criterion_group!(
    name = bench_blocking_compress_files_in_async;
    config = Criterion::default().sample_size(10);
    targets = benchmark_blocking_compress_files_in_async
);

criterion_main!(
    bench_blocking_compress_files,
    bench_blocking_compress_files_in_async,
    bench_compress_files
);
