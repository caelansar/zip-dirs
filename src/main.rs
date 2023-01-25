#![feature(is_some_and)]
#![feature(result_option_inspect)]
#![feature(option_result_contains)]

mod async_zip;
mod option;

use futures::StreamExt;
use option::{Opt, ZipType};
use std::path::{Path, PathBuf};
use structopt::StructOpt;
use tokio::{
    fs::{read_dir, File},
    io::{AsyncReadExt, AsyncWriteExt},
    sync::mpsc::{channel, Sender},
};

use ::async_zip as az;
use anyhow::{anyhow, bail, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let opt = Opt::from_args();
    let dir = opt.input_dir.as_path();

    if !dir.exists() {
        bail!("Directory does not exist");
    }

    println!(
        "input directory is: {:?}, zip_type is: {:?}",
        dir, opt.zip_type
    );

    zip_dir(&dir, opt.zip_type).await
}

struct Zipper;

struct Zip;

impl Zipper {
    async fn zip(path: impl AsRef<Path>) -> Result<()> {
        zip(path).await
    }
}

impl Zip {
    async fn zip(path: impl AsRef<Path>) -> Result<()> {
        zip1(path).await
    }
}

async fn zip1(dir: impl AsRef<Path>) -> Result<()> {
    println!("output {:?}", dir.as_ref().with_extension("zip"));
    let archive = File::create(dir.as_ref().with_extension("zip"))
        .await
        .unwrap();

    let writer = az::write::ZipFileWriter::new(archive);

    handle_directory(dir, writer).await
}

async fn handle_directory(
    input_path: impl AsRef<Path>,
    mut writer: az::write::ZipFileWriter<File>,
) -> Result<()> {
    let entries = walk_directory(input_path.as_ref().into()).await?;

    let (tx, mut rx) = channel(1024);

    for entry_path_buf in entries {
        let tx = tx.clone();

        tokio::spawn(async move {
            write_entry(entry_path_buf, tx).await.unwrap();
        });
    }
    drop(tx);

    while let Some(data) = rx.recv().await {
        let builder = az::ZipEntryBuilder::new(data.0, az::Compression::Deflate);
        writer.write_entry_whole(builder, &data.1).await.unwrap();
    }

    Ok(())
}

async fn write_entry(input_path: impl AsRef<Path>, tx: Sender<(String, Vec<u8>)>) -> Result<()> {
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

async fn walk_directory(dir: PathBuf) -> Result<Vec<PathBuf>> {
    let mut dirs = vec![dir];
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

async fn zip(dir: impl AsRef<Path>) -> Result<()> {
    let z = async_zip::Zipper::from_directory(dir.as_ref()).await?;
    let mut chunks = z.zipped_stream();

    println!("output {:?}", dir.as_ref().with_extension("zip"));
    let mut f = File::create(dir.as_ref().with_extension("zip")).await?;

    while let Some(chunk) = chunks.next().await {
        f.write_all(&chunk.expect("invalid zip read")).await?
    }
    Ok(())
}

async fn zip_dir(dir: impl AsRef<Path>, zip_type: ZipType) -> Result<()> {
    let mut entries = read_dir(dir.as_ref()).await?;
    while let Some(f) = entries.next_entry().await? {
        let filename = f.file_name().into_string().unwrap();
        let directory = f.path();
        // ignore hidden dir and excluded dir
        if !filename.starts_with(".") && directory.is_dir() {
            println!("filename: {}, dir: {:?}", filename, directory);
            match zip_type {
                ZipType::Zipper => Zipper::zip(directory).await?,
                ZipType::Zip => Zip::zip(directory).await?,
            }
        }
    }
    Ok(())
    // let entries = WalkDir::new(dir);

    // entries
    //     .filter(|f| async move {
    //         if f.file_type().await.is_ok_and(|f| !f.is_dir()) {
    //             Filtering::Ignore
    //         } else {
    //             Filtering::Continue
    //         }
    //     })
    //     .for_each_concurrent(2, |f| async move {
    //         if f.is_ok() {
    //             let filename = f.as_ref().unwrap().file_name().into_string().unwrap();
    //             let directory = f.as_ref().unwrap().path();
    //             println!("filename: {}, dir: {:?}", filename, directory);
    //             zip(directory).await;
    //         } else {
    //             println!("err {}", f.err().unwrap())
    //         }
    //     })
    //     .await;
}

#[cfg(test)]
mod test {
    use path_absolutize::*;
    use std::path::Path;

    #[test]
    fn test_path() {
        let path2 = Path::new("~/Rust/zip_dirs/src/async_zip");
        // let path1 = Path::new("./src/async_zip");
        //
        println!("{:?}", path2.iter().last());

        // let pb1 = path1.absolutize().unwrap();
        let pb2 = path2.absolutize().unwrap();

        // println!("{}", pb1.to_str().unwrap());
        println!("{}", pb2.to_str().unwrap());
        // println!("{}", pb1 == pb2);
    }
}
