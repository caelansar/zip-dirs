use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "example", about = "An example of StructOpt usage.")]
pub(crate) struct Opt {
    /// Input directory path
    #[structopt(parse(from_os_str), default_value = ".")]
    pub(crate) input_dir: PathBuf,

    /// Zip type, optional value is zip or zipper
    #[structopt(short, parse(try_from_str = parse_zip_type), default_value = "zipper")]
    pub(crate) zip_type: ZipType,
}

#[derive(Debug)]
pub(crate) enum ZipType {
    Zip,
    Zipper,
}

fn parse_zip_type(src: &str) -> Result<ZipType, anyhow::Error> {
    match src {
        "zip" => Ok(ZipType::Zip),
        "zipper" => Ok(ZipType::Zipper),
        _ => Err(anyhow::anyhow!("Only support zip/zipper")),
    }
}
