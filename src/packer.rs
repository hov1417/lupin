use std::cmp::Ordering;
use std::collections::binary_heap::BinaryHeap;
use std::path::Path;
use std::sync::Arc;

use eyre::bail;
use tokio::sync::{Mutex, RwLock};
use tracing::{info, warn};

use crate::compression::{ChunkedCompressor, Compressor};

enum QueueItem {
    Sized {
        path: String,
        file_packer: FilePacker,
        size: u64,
    },
    Unsized {
        path: String,
        file_packer: FilePacker,
    },
}

impl QueueItem {
    fn priority(&self) -> u64 {
        match self {
            QueueItem::Sized { size, .. } => *size,
            QueueItem::Unsized { .. } => u64::MAX,
        }
    }
}

impl Ord for QueueItem {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority().cmp(&other.priority())
    }
}

impl PartialOrd for QueueItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(&other))
    }
}

impl PartialEq for QueueItem {
    fn eq(&self, other: &Self) -> bool {
        self.priority() == other.priority()
    }
}

impl Eq for QueueItem {}

#[derive(Clone)]
struct Packer {
    queue: Arc<RwLock<BinaryHeap<QueueItem>>>,
    compressor: Arc<Mutex<Compressor>>,
}

impl Packer {
    pub async fn new(out: &Path) -> eyre::Result<Self> {
        Ok(Packer {
            queue: Arc::new(RwLock::new(BinaryHeap::new())),
            compressor: Arc::new(Mutex::new(Compressor::new(out).await?)),
        })
    }

    pub async fn new_file(
        &self,
        name: String,
        size: Option<u64>,
    ) -> FilePacker {
        let file_packer = FilePacker {
            name: name.clone(),
            compressor: self.compressor.clone(),
            buffer: Arc::new(Mutex::new(Vec::new())),
            chunked_compressor: Arc::new(Mutex::new(None)),
            finished: Arc::new(Mutex::new(false)),
        };
        match size {
            Some(size) => {
                self.queue.write().await.push(QueueItem::Sized {
                    path: name,
                    file_packer: file_packer.clone(),
                    size,
                });
            }
            None => {
                warn!("unsized file: {:?}", name);
                self.queue.write().await.push(QueueItem::Unsized {
                    path: name,
                    file_packer: file_packer.clone(),
                });
            }
        }

        file_packer
    }
}

// make enum
#[derive(Clone)]
struct FilePacker {
    name: String,
    compressor: Arc<Mutex<Compressor>>,
    buffer: Arc<Mutex<Vec<u8>>>,
    chunked_compressor: Arc<Mutex<Option<ChunkedCompressor>>>,
    finished: Arc<Mutex<bool>>,
}

impl FilePacker {
    pub async fn write(&mut self, buf: &[u8]) -> eyre::Result<()> {
        let mut buffer = self.buffer.lock().await;
        if let Some(chunked_compressor) =
            self.chunked_compressor.lock().await.as_mut()
        {
            if !buffer.is_empty() {
                chunked_compressor.add_chunk(&buffer).await?;
                buffer.clear();
            }
            chunked_compressor.add_chunk(buf).await?;
        } else {
            buffer.extend_from_slice(buf);
        }
        Ok(())
    }

    pub async fn finish(&mut self) -> eyre::Result<()> {
        let buffer = self.buffer.lock().await;
        let mut guard = self.finished.lock().await;
        if *guard {
            return Ok(());
        }
        *guard = true;
        if self.chunked_compressor.lock().await.is_none() {
            if buffer.is_empty() {
                info!("empty file: {:?}", self.name);
            }
            let mut compressor = self.compressor.lock().await;
            compressor.add_file_with_data(&self.name, &buffer).await?;
        }
        Ok(())
    }

    async fn activate_when_finished(
        &self,
        compressor: &mut Compressor,
    ) -> eyre::Result<()> {
        let buffer = self.buffer.lock().await;
        compressor.add_file_with_data(&self.name, &buffer).await?;
        Ok(())
    }

    async fn activate_when_unfinished(
        &self,
        compressor: Arc<Mutex<Compressor>>,
        size: u64,
    ) -> eyre::Result<()> {
        let mut self_chunked_compressor = self.chunked_compressor.lock().await;
        if self_chunked_compressor.is_some() {
            bail!("file already activated");
        }
        let mut chunked_compressor =
            Compressor::add_chunked_file(compressor, &self.name, size).await?;

        let mut buffer = self.buffer.lock().await;
        if !buffer.is_empty() {
            chunked_compressor.add_chunk(&buffer).await?;
            *buffer = Vec::new();
        }

        *self_chunked_compressor = Some(chunked_compressor);

        Ok(())
    }
}
