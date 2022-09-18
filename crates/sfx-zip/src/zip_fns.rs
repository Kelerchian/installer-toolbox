use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};
use walkdir::WalkDir;
use zip::write::FileOptions;

pub fn archive<P>(source: P, destination: P)
where
    P: AsRef<Path>,
{
    let source = source.as_ref();
    let walkdir = WalkDir::new(source);
    let walkdir_iter = walkdir.into_iter();
    let zip_options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);
    let zip_file = File::create(destination).unwrap();
    let mut zip = zip::ZipWriter::new(zip_file);

    for entry in walkdir_iter {
        let entry = entry.unwrap();
        let source_file_path = entry.path();
        let key = source_file_path.strip_prefix(source).unwrap();

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
}

pub fn extract<P>(source: P, destination: P)
where
    P: AsRef<Path>,
{
    let zip_file = File::open(source).unwrap();
    let mut archive = zip::ZipArchive::new(zip_file).unwrap();
    archive.extract(destination).unwrap();
}
