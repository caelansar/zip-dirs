#![feature(is_some_and)]
#![feature(result_option_inspect)]
#![feature(option_result_contains)]

mod async_zip;

use async_walkdir::{Filtering, WalkDir};
use futures::StreamExt;
use std::{env, path::Path};
use tokio::{fs::File, io::AsyncWriteExt};

use crate::async_zip::Zipper;

#[tokio::main]
async fn main() {
    let dir = env::args().nth(1).expect("expect directory param");

    println!("directory is: {}", dir);

    walk_dir(dir).await;
}

async fn zip(dir: impl AsRef<Path>) {
    let z = Zipper::from_directory(dir.as_ref())
        .await
        .expect("failed to list directory");
    let mut chunks = z.zipped_stream();

    let mut f = File::create(dir.as_ref().with_extension("zip"))
        .await
        .expect("failed to create zip file");

    while let Some(chunk) = chunks.next().await {
        f.write_all(&chunk.expect("invalid zip read"))
            .await
            .expect("failed to write file")
    }
}

async fn walk_dir(dir: impl AsRef<Path>) {
    let entries = WalkDir::new(dir);

    entries
        .filter(|f| async move {
            if f.file_type().await.is_ok_and(|f| !f.is_dir()) {
                Filtering::Ignore
            } else {
                Filtering::Continue
            }
        })
        .for_each_concurrent(2, |f| async move {
            if f.is_ok() {
                let filename = f.as_ref().unwrap().file_name().into_string().unwrap();
                let directory = f.as_ref().unwrap().path();
                println!("filename: {}, dir: {:?}", filename, directory);
                zip(directory).await;
            } else {
                println!("err {}", f.err().unwrap())
            }
        })
        .await;
}
