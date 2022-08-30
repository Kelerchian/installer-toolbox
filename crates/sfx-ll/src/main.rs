pub mod common;
pub mod embedder;
pub mod extractor;
use std::fs;
use std::path::PathBuf;
use structopt::StructOpt;
use windows::Win32::Foundation::HANDLE;

#[derive(Debug, StructOpt)]
struct Opt {
    // Create archive file
    #[structopt(subcommand)]
    archive: Subcommand,
}

#[derive(Debug, StructOpt)]
enum Subcommand {
    Archive {
        #[structopt(short, long)]
        destination: PathBuf,
        #[structopt(short, long)]
        source: PathBuf,
    },
    Extract {
        #[structopt(short, long)]
        destination: PathBuf,
    },
}

fn main() {
    let opt = Opt::from_args();
    match &opt.archive {
        Subcommand::Archive {
            destination,
            source,
        } => {
            if destination.exists() {
                eprintln!(
                    "Destination at {:?} exists. Choose another filename!",
                    &destination
                );
                return;
            }

            if !source.exists() {
                eprintln!(
                    "Source file at {:?} does not exist. Choose another filename!",
                    &source
                );
                return;
            }

            fs::copy(std::env::current_exe().unwrap(), destination).unwrap();

            embedder::with_resource_update_handle(
                destination.as_path(),
                Box::new(|update_resource_handle: &HANDLE| {
                    let block_count =
                        embedder::embed_binary_as_archive(update_resource_handle, source.as_path())
                            .unwrap();
                    let _success =
                        embedder::embed_block_count(update_resource_handle, &block_count).is_ok();
                }),
            );
        }
        Subcommand::Extract { destination } => {
            let block_count = extractor::read_block_count().unwrap();
            extractor::extract_binary(destination, &block_count);
        }
    }
}
