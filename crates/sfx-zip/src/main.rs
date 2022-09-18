mod zip_fns;

use sfx_ll::{self, embedder, extractor};
use std::{
    fs::{self},
    io::{Error, ErrorKind},
    path::{Path, PathBuf},
};
use structopt::{self, StructOpt};
use zip_fns::{archive, extract};

#[derive(Debug, StructOpt)]
struct Opt {
    // Create archive file
    #[structopt(subcommand)]
    subcommand: Subcommand,
}

const ENTRYPOINT_KEY: &str = "entrypoint";

#[derive(Debug, StructOpt)]
enum Subcommand {
    Archive {
        #[structopt(short = "s", long)]
        source: PathBuf,
        #[structopt(short = "d", long)]
        destination: PathBuf,
        #[structopt(short = "z", long)]
        temp_zip_file_name: PathBuf,
        #[structopt(short = "e", long)]
        entry_point: Option<PathBuf>,
        #[structopt(short = "f", long)]
        force: bool,
    },
    Extract {
        #[structopt(short = "d", long)]
        destination: PathBuf,
        #[structopt(short = "t", long)]
        temp_zip_file_name: PathBuf,
        #[structopt(short = "f", long)]
        force: bool,
    },
}

fn ensure_gone<P: AsRef<Path>>(filepath: P) {
    let filepath = filepath.as_ref();

    if !filepath.exists() {
        return;
    }

    if filepath.is_dir() {
        fs::remove_dir_all(filepath).unwrap();
    } else {
        fs::remove_file(filepath).unwrap();
    }
}

fn main() -> Result<(), Option<std::io::Error>> {
    let opt = Opt::from_args();

    match &opt.subcommand {
        Subcommand::Archive {
            source,
            destination,
            temp_zip_file_name,
            entry_point,
            force,
        } => {
            let mut errors: Vec<std::io::Error> = vec![];

            if !source.exists() && !force {
                errors.push(Error::new(
                    ErrorKind::Other,
                    "source does not exist. source must exist.",
                ));
            }

            if temp_zip_file_name.exists() && !force {
                errors.push(Error::new(
                    ErrorKind::Other,
                    "temp_zip_file_name already exists. temp_zip_file_name cannot exist.",
                ));
            }

            if destination.exists() && !force {
                errors.push(Error::new(
                    ErrorKind::Other,
                    "destination already exists. destination cannot exist.",
                ));
            }

            if !source.is_dir() {
                errors.push(Error::new(
                    ErrorKind::Other,
                    "source is not a directory. source must be directory.",
                ));
            }

            if let Some(entry_point) = entry_point {
                if entry_point.is_absolute() {
                    errors.push(Error::new(
                        ErrorKind::Other,
                        "entry_point cannot be absolute path.",
                    ));
                }

                let absolute_entry_point = source.join(entry_point);

                if !absolute_entry_point.exists() {
                    errors.push(Error::new(ErrorKind::Other, "entry_point does not exist"));
                }
            }

            if errors.len() > 0 {
                eprintln!("{} error occured", errors.len());
                errors.iter().for_each(|error| {
                    eprintln!("error: {}", error);
                });
                eprintln!("parameters {:?}", &opt.subcommand);
                return Err(None);
            }

            ensure_gone(destination);
            ensure_gone(temp_zip_file_name);
            ensure_gone(destination);

            // Make zip file
            archive(source, temp_zip_file_name);

            fs::copy(std::env::current_exe().unwrap(), destination).unwrap();

            embedder::with_resource_update_handle(
                destination.as_path(),
                Box::new(|handle| {
                    embedder::embed_binary_as_archive(handle, &temp_zip_file_name.as_path())
                        .unwrap();

                    if let Some(entry_point) = entry_point {
                        let entry_point_str = String::from(entry_point.to_str().unwrap());
                        embedder::embed_custom_string(
                            handle,
                            &String::from(ENTRYPOINT_KEY),
                            &entry_point_str,
                        );
                    }
                }),
            );

            Ok(())
        }
        Subcommand::Extract {
            destination,
            temp_zip_file_name,
            force,
        } => {
            let mut errors: Vec<std::io::Error> = vec![];

            if destination.exists() && !force {
                if !destination.is_dir() {
                    errors.push(Error::new(
                        ErrorKind::Other,
                        "destination is not directory.",
                    ));
                } else if destination.read_dir()?.count() > 0 {
                    errors.push(Error::new(ErrorKind::Other, "destination is not empty."));
                }
            }
            if temp_zip_file_name.exists() && !force {
                errors.push(Error::new(ErrorKind::Other, "tempfile already exist."));
            }

            if errors.len() > 0 {
                eprintln!("{} error occured", errors.len());
                errors.iter().for_each(|error| {
                    eprintln!("error: {}", error);
                });
                eprintln!("parameters {:?}", &opt.subcommand);
                return Err(None);
            }

            // Start
            ensure_gone(destination);
            ensure_gone(temp_zip_file_name);

            fs::create_dir(destination)?;

            // let entry_point = extractor::read_custom_string(ENTRYPOINT_KEY);
            extractor::extract_binary(temp_zip_file_name).unwrap();

            extract(temp_zip_file_name, destination);

            Ok(())
        }
    }
}
