use sfx_ll::{self, embedder, windows::Win32::Foundation::HANDLE};
use std::{
    fs::{self, File},
    io::{BufReader, BufWriter, Error, ErrorKind, Read, Write},
    path::{self, Path, PathBuf},
};
use structopt::{self, StructOpt};
use walkdir::WalkDir;
use zip::{self, write::FileOptions};

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
        #[structopt(short = "t", long)]
        temp_file_name: PathBuf,
        #[structopt(short = "z", long)]
        temp_zip_file_name: PathBuf,
        #[structopt(short = "e", long)]
        entry_point: Option<PathBuf>,
    },
    Extract {},
}

fn main() {
    let opt = Opt::from_args();

    match &opt.subcommand {
        Subcommand::Archive {
            source,
            destination,
            temp_file_name,
            temp_zip_file_name,
            entry_point,
        } => {
            let mut errors: Vec<std::io::Error> = vec![];

            if !source.exists() {
                errors.push(Error::new(
                    ErrorKind::Other,
                    "source does not exist. source must exist.",
                ));
            }

            if temp_file_name.exists() {
                errors.push(Error::new(
                    ErrorKind::Other,
                    "temp_file_name already exists. temp_file_name cannot exist.",
                ));
            }

            if temp_zip_file_name.exists() {
                errors.push(Error::new(
                    ErrorKind::Other,
                    "temp_zip_file_name already exists. temp_zip_file_name cannot exist.",
                ));
            }

            if !source.is_dir() {
                errors.push(Error::new(
                    ErrorKind::Other,
                    "source is not a directory. source must be directory.",
                ));
            }

            if destination.exists() {
                errors.push(Error::new(
                    ErrorKind::Other,
                    "destination already exists. destination cannot exist.",
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
                return;
            }

            // Make zip file
            {
                let walkdir = WalkDir::new(source);
                let walkdir_iter = walkdir.into_iter();
                let zip_options = FileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated)
                    .unix_permissions(0o755);
                let zip_file = File::create(temp_zip_file_name).unwrap();
                let mut zip = zip::ZipWriter::new(zip_file);

                for entry in walkdir_iter {
                    let entry = entry.unwrap();
                    let source_file_path = entry.path();
                    let key = source_file_path.strip_prefix(Path::new(source)).unwrap();

                    // Write file or directory explicitly
                    // Some unzip tools unzip files with directory paths correctly, some do not!
                    if source_file_path.is_file() {
                        let mut buffer = Vec::new();
                        println!("adding file {:?} as {:?} ...", source_file_path, key);
                        #[allow(deprecated)]
                        zip.start_file_from_path(key, zip_options).unwrap();

                        let mut source_file = File::open(source_file_path).unwrap();
                        source_file.read_to_end(&mut buffer).unwrap();

                        zip.write_all(&*buffer).unwrap();
                    } else if !key.as_os_str().is_empty() {
                        // Only if not root! Avoids path spec / warning
                        // and mapname conversion failed error on unzip
                        println!("adding dir {:?} as {:?} ...", source_file_path, key);
                        #[allow(deprecated)]
                        zip.add_directory(key.to_str().unwrap(), zip_options)
                            .unwrap();
                    }
                }
            } // drop zip and zip file here

            fs::copy(std::env::current_exe().unwrap(), temp_file_name).unwrap();

            embedder::with_resource_update_handle(
                destination.as_path(),
                Box::new(|handle| {
                    let block_count =
                        embedder::embed_binary_as_archive(handle, &temp_zip_file_name.as_path())
                            .unwrap();
                    let _success = embedder::embed_block_count(handle, &block_count).is_ok();

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
        }
        Subcommand::Extract {} => {}
    }
}
