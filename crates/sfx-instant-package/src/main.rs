use std::{
    fs,
    io::Error,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use sfx_zip::sfx_ll::{embedder, extractor};
use structopt::StructOpt;
use uuid::{self, Uuid};

#[derive(Debug, StructOpt)]
struct Opt {
    // Create archive file
    #[structopt(subcommand)]
    archive: Option<Subcommand>,
}

#[derive(Debug, StructOpt, Clone)]
enum Subcommand {
    Archive {
        #[structopt(short = "a", long)]
        app_id: String,
        #[structopt(short = "s", long)]
        source: PathBuf,
        #[structopt(short = "w", long)]
        workspace: PathBuf,
        #[structopt(short = "d", long)]
        destination: PathBuf,
        #[structopt(short = "f", long)]
        force: bool,
    },
}

const FLAG_IS_ARCHIVE: &str = "SFX_INSTANT_FLAG__PACKAGE_ARCHIVE";
const FLAG_APP_ID: &str = "SFX_INSTANT_FLAG__APP_ID";
const INSTALLER_WORKSPACE_SUBPATH: &str = ".sfx_instant_installer_workspace";
const APP_DIR_SUBPATH: &str = ".sfx_app";

fn main() {
    let opt = Opt::from_args();

    match opt.archive {
        Some(archive_opt) => handle_archive(archive_opt),
        None => handle_extract(),
    }
}

fn handle_extract() {
    let flag_is_archive_in_this_exe =
        sfx_zip::sfx_ll::extractor::read_custom_string(&String::from(FLAG_IS_ARCHIVE));
    if let None = flag_is_archive_in_this_exe {
        eprintln!("Cannot extract. Not archive");
        return;
    }
    let app_id_in_this_exe =
        sfx_zip::sfx_ll::extractor::read_custom_string(&String::from(FLAG_APP_ID));

    let app_id_in_this_exe = match app_id_in_this_exe {
        Some(x) => x,
        None => {
            eprintln!("Cannot extract. No app_id");
            return;
        }
    };

    let local_app_data_path = match std::env::var("LOCALAPPDATA") {
        Ok(string) => PathBuf::from(string),
        Err(_) => {
            eprintln!("Cannot install. LOCALAPPDATA is not defined");
            return;
        }
    };

    let root_app_path = local_app_data_path.join(app_id_in_this_exe);
    let installer_workspace_path = root_app_path.join(INSTALLER_WORKSPACE_SUBPATH);
    let app_dir_path = root_app_path.join(APP_DIR_SUBPATH);

    // TEST installer_path first

    ensure_gone(&app_dir_path);
    ensure_gone(&installer_workspace_path);
    fs::create_dir_all(&installer_workspace_path).unwrap();

    let zip_file_path = installer_workspace_path.join({
        let mut path = PathBuf::from(Uuid::new_v4().to_string());
        path.set_extension("zip");
        path
    });

    extractor::extract_binary(&zip_file_path).unwrap();
    sfx_zip::zip_fns::extract(&zip_file_path, &app_dir_path);
    ensure_gone(&installer_workspace_path);
}

fn handle_archive(opt: Subcommand) {
    let Subcommand::Archive {
        app_id,
        source,
        workspace,
        destination,
        force,
    } = opt.clone();

    let mut errors: Vec<std::io::Error> = vec![];

    let flag_is_archive_in_this_exe =
        sfx_zip::sfx_ll::extractor::read_custom_string(&String::from(FLAG_IS_ARCHIVE));

    if let Some(_) = flag_is_archive_in_this_exe {
        eprintln!("Cannot use this exe to archive. This exe is already an archive");
        return;
    }

    if !source.is_dir() {
        errors.push(Error::new(ErrorKind::Other, "Source is not directory"));
    }

    if !workspace.is_dir() {
        errors.push(Error::new(ErrorKind::Other, "Workspace is not directory"));
    }

    if destination.exists() && !force {
        errors.push(Error::new(ErrorKind::Other, "Destination already exist"));
    }

    if errors.len() > 0 {
        eprintln!("{} error occured", errors.len());
        errors.iter().for_each(|error| {
            eprintln!("error: {}", error);
        });
        eprintln!("parameters {:?}", &opt);
        return;
    }

    let temp_zip_path = workspace.join({
        let unlucky_limit = 15;
        let mut index: usize = 0;
        loop {
            let mut temp_file_path = PathBuf::from(Uuid::new_v4().to_string());
            temp_file_path.set_extension("zip");
            if !temp_file_path.exists() {
                break temp_file_path;
            }

            index += 1;
            if index > unlucky_limit {
                eprintln!("Unlucky, all filename guesses are already used");
                return;
            }
        }
    });

    sfx_zip::zip_fns::archive(source, temp_zip_path.clone());

    // TODO: kill app-id first
    // TODO: code signing

    ensure_gone(&destination);
    fs::copy(std::env::current_exe().unwrap(), &destination).unwrap();

    sfx_zip::sfx_ll::embedder::with_resource_update_handle(
        &destination,
        Box::new(|handle| {
            embedder::embed_binary_as_archive(handle, &temp_zip_path.as_path()).unwrap();

            embedder::embed_custom_string(
                handle,
                &String::from(FLAG_IS_ARCHIVE),
                &String::from(FLAG_IS_ARCHIVE),
            );

            embedder::embed_custom_string(
                handle,
                &String::from(FLAG_APP_ID),
                &String::from(app_id),
            );
        }),
    );
    ensure_gone(temp_zip_path);
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
