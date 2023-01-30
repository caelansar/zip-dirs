#![feature(is_some_and)]
#![feature(result_option_inspect)]
#![feature(option_result_contains)]
#![feature(async_fn_in_trait)]
#![feature(associated_type_defaults)]
#![feature(type_alias_impl_trait)]

mod async_zip;
mod option;
mod zip_core;

use option::Opt;
use path_absolutize::*;

use std::{
    borrow::Cow,
    env,
    path::{Path, PathBuf},
};
use structopt::StructOpt;

use anyhow::{bail, Result};

use crate::{
    option::ZipType,
    zip_core::{AsyncZip, DirsZipEngine, Zip, ZipEngine, Zipper},
};

#[tokio::main]
async fn main() -> Result<()> {
    let opt = Opt::from_args();
    let dir = opt.input_dir.as_path();

    if !dir.exists() {
        bail!("Directory does not exist");
    }

    println!(
        "input directory is: {:?}, zip_type is: {:?}, exclude_dir is: {:?}",
        dir, opt.zip_type, opt.exclude_dir
    );

    let excluded = opt.exclude_dir.clone();

    match opt.zip_type {
        ZipType::Zip => {
            println!("ok");
            DirsZipEngine::new(Zip {}, &dir, excluded).do_zip().await
        }
        ZipType::Zipper => DirsZipEngine::new(Zipper {}, &dir, excluded).do_zip().await,
        ZipType::AsyncZip => {
            DirsZipEngine::new(AsyncZip {}, &dir, excluded)
                .do_zip()
                .await
        }
    }
}

#[allow(deprecated)]
fn absolute_path<T: AsRef<Path>>(cwd: Option<T>, path: &Path) -> Cow<'_, Path> {
    let home = env::home_dir().unwrap();

    if path.starts_with("~/") {
        let path = path.strip_prefix("~/").unwrap();
        path.absolutize_virtually(home).unwrap()
    } else {
        if let Some(cwd) = cwd {
            path.absolutize_from(&cwd.as_ref().absolutize().unwrap())
                .unwrap()
        } else {
            path.absolutize_virtually(home).unwrap()
        }
    }
}

fn is_exclude(cwd: Option<&Path>, exclude: &Vec<PathBuf>, dir: impl AsRef<Path>) -> bool {
    if exclude.is_empty() {
        return false;
    }
    let exclude: Vec<Cow<Path>> = exclude
        .iter()
        .map(|path| absolute_path(cwd, &path))
        .collect();

    let absolute_dir = absolute_path(cwd, dir.as_ref());

    println!("exclude dirs {:?}, dir {:?}", exclude, absolute_dir);

    exclude.iter().find(|x| **x == absolute_dir).is_some()
}

#[cfg(test)]
mod test {
    use std::{borrow::Borrow, path::Path};

    use path_absolutize::Absolutize;

    use crate::is_exclude;

    #[test]
    fn absolutize_from_should_work() {
        let path1 = Path::new("src/async_zip");
        let path = path1.absolutize_from(Path::new("/ss/bb")).unwrap();
        assert_eq!(
            Path::new("/ss/bb/src/async_zip"),
            Borrow::<Path>::borrow(&path)
        );

        let path1 = Path::new("/src/async_zip");
        let path = path1.absolutize_from(Path::new("/ss/bb")).unwrap();
        assert_eq!(Path::new("/src/async_zip"), Borrow::<Path>::borrow(&path))
    }

    #[test]
    fn is_exclude_should_work() {
        let path1 = Path::new("./src/async_zip");
        let path2 = Path::new("~/Rust/zip_dirs/src/async_zip");
        let path3 = Path::new("../async_zip");
        let path4 = Path::new("./src/async_zip");

        let ok = is_exclude(
            Some(Path::new(".").as_ref()),
            &vec![path1.to_path_buf()],
            path2,
        );
        assert!(ok);

        let ok = is_exclude(
            Some(Path::new(".").as_ref()),
            &vec![path1.to_path_buf()],
            path3,
        );
        assert!(!ok);

        let ok = is_exclude(None, &vec![path1.to_path_buf()], path4);
        assert!(ok);
    }
}
