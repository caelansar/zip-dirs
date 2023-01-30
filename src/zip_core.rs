use anyhow::{anyhow, bail, Result};
use futures::{Future, Stream, StreamExt};
use std::fs::File as StdFile;
use std::io;
use std::path::{Path, PathBuf};
use tokio::fs::{read_dir, DirEntry, ReadDir};
use tokio::io::AsyncWriteExt;
use tokio::{
    fs::File,
    io::AsyncReadExt,
    sync::mpsc::{channel, Receiver, Sender},
};
use tokio_stream::wrappers::ReadDirStream;
use zip::ZipWriter;
use zip_extensions::write::ZipWriterExtensions;

use crate::{async_zip, is_exclude};
use ::async_zip as az;

pub trait ZipCore {
    async fn zip_entry(&self, path: impl AsRef<Path>) -> Result<()>;
}

type ZipStream = impl Stream<Item = io::Result<DirEntry>> + Unpin;

pub trait ZipEngine: ZipCore {
    fn skip(&self, dir: DirEntry) -> bool;

    async fn get_stream(&self) -> ZipStream;

    async fn do_zip(&self) -> Result<()> {
        let mut stream = self.get_stream().await;

        while let Some(Ok(entry)) = stream.next().await {
            let filename = entry.file_name().into_string().unwrap();
            let directory = entry.path();
            // skip hidden directory and excluded directory
            if !self.skip(entry) {
                println!("filename: {}, dir: {:?}", filename, directory);
                self.zip_entry(directory).await?
            } else {
                println!("skip {}", filename)
            }
        }

        Ok(())
    }
}

pub struct DirsZipEngine<T: ZipCore> {
    inner: T,
    path: PathBuf,
    excluded: Vec<PathBuf>,
}

impl<T: ZipCore> DirsZipEngine<T> {
    pub fn new(inner: T, path: impl AsRef<Path>, excluded: Vec<PathBuf>) -> Self {
        Self {
            inner,
            path: path.as_ref().to_path_buf(),
            excluded,
        }
    }
}

impl<T: ZipCore> ZipCore for DirsZipEngine<T> {
    async fn zip_entry(&self, path: impl AsRef<Path>) -> Result<()> {
        self.inner.zip_entry(path).await
    }
}

impl<T: ZipCore> ZipEngine for DirsZipEngine<T> {
    async fn get_stream(&self) -> ZipStream {
        ReadDirStream::new(read_dir(&self.path).await.unwrap())
    }

    fn skip(&self, dir: DirEntry) -> bool {
        let filename = dir.file_name().into_string().unwrap();
        let directory = dir.path();

        filename.starts_with(".")
            || directory.is_file()
            || is_exclude(
                Some(dir.path().as_ref()),
                &self.excluded,
                directory.as_path(),
            )
    }
}

pub struct AsyncZip;

impl AsyncZip {
    async fn handle_directory(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<Receiver<(String, Vec<u8>)>> {
        let entries = self.walk_directory(path).await?;

        let (tx, rx) = channel(1024);

        for entry_path_buf in entries {
            let tx = tx.clone();

            tokio::spawn(async move {
                Self::write_entry(entry_path_buf, tx).await.unwrap();
            });
        }

        Ok(rx)
    }

    async fn write_entry(
        input_path: impl AsRef<Path>,
        tx: Sender<(String, Vec<u8>)>,
    ) -> Result<()> {
        let mut input_file = File::open(input_path.as_ref()).await?;
        let input_file_size = input_file.metadata().await?.len() as usize;

        // read file data to buffer
        let mut buffer = Vec::with_capacity(input_file_size);
        input_file.read_to_end(&mut buffer).await?;

        let filename = input_path
            .as_ref()
            .file_name()
            .ok_or(anyhow!("Directory file path not valid UTF-8."))?
            .to_str()
            .ok_or(anyhow!("Directory file path not valid UTF-8."))?;

        if let Err(e) = tx.send((filename.to_string(), buffer)).await {
            bail!("Failed to send, err {}", e);
        };

        Ok(())
    }

    async fn walk_directory(&self, path: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
        let mut dirs = vec![path.as_ref().to_owned()];
        let mut files = vec![];

        while !dirs.is_empty() {
            let mut dir_iter = tokio::fs::read_dir(dirs.remove(0)).await?;

            while let Some(entry) = dir_iter.next_entry().await? {
                let entry_path_buf = entry.path();

                if entry_path_buf.is_dir() {
                    dirs.push(entry_path_buf);
                } else {
                    files.push(entry_path_buf);
                }
            }
        }

        Ok(files)
    }
}

impl ZipCore for AsyncZip {
    async fn zip_entry(&self, path: impl AsRef<Path>) -> Result<()> {
        println!("output {:?}", path.as_ref().with_extension("zip"));
        let archive = File::create(path.as_ref().with_extension("zip"))
            .await
            .unwrap();

        let mut writer = az::write::ZipFileWriter::new(archive);

        let mut rx = self.handle_directory(path).await?;

        while let Some(data) = rx.recv().await {
            let builder = az::ZipEntryBuilder::new(data.0, az::Compression::Deflate);
            writer.write_entry_whole(builder, &data.1).await.unwrap();
        }
        Ok(())
    }
}

pub struct Zip;

impl ZipCore for Zip {
    async fn zip_entry(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref().to_owned().clone();
        tokio::task::spawn_blocking(move || {
            let file = StdFile::create(path.with_extension("zip")).unwrap();
            let mut zip = ZipWriter::new(file);
            zip.create_from_directory(&path.to_path_buf()).unwrap();
        });
        Ok(())
    }
}

pub struct Zipper;

impl ZipCore for Zipper {
    async fn zip_entry(&self, path: impl AsRef<Path>) -> Result<()> {
        let z = async_zip::Zipper::from_directory(path.as_ref()).await?;
        let mut chunks = z.zipped_stream();

        println!("output {:?}", path.as_ref().with_extension("zip"));
        let mut f = File::create(path.as_ref().with_extension("zip")).await?;

        while let Some(chunk) = chunks.next().await {
            f.write_all(&chunk.expect("invalid zip read")).await?
        }
        Ok(())
    }
}
