use std::{ops::Deref, path::PathBuf};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "zip_dirs", about = "squash things in directories")]
pub(crate) struct Opt {
    /// Input directory path
    #[structopt(parse(from_os_str), default_value = ".")]
    pub(crate) input_dir: PathBuf,

    /// Zip type, optional value is zip or zipper
    #[structopt(short, parse(try_from_str = parse_zip_type), default_value = "zip")]
    pub(crate) zip_type: ZipType,

    /// Exclude dir
    #[structopt(short = "e", long = "exclude-dir", default_value = "")]
    pub(crate) exclude_dir: Dirs,
}

#[derive(Debug)]
pub(crate) enum ZipType {
    AsyncZip,
    Zipper,
    Zip,
}

fn parse_zip_type(src: &str) -> Result<ZipType, anyhow::Error> {
    match src {
        "async_zip" => Ok(ZipType::AsyncZip),
        "self_async_zip" => Ok(ZipType::Zipper),
        "zip" => Ok(ZipType::Zip),
        _ => Err(anyhow::anyhow!("Not support")),
    }
}

#[derive(Debug)]
pub struct Dirs(Vec<PathBuf>);

impl Deref for Dirs {
    type Target = Vec<PathBuf>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::str::FromStr for Dirs {
    type Err = Box<dyn std::error::Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Dirs(
            s.split(",").map(|x| x.trim().to_owned().into()).collect(),
        ))
    }
}
