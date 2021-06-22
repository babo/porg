use chrono::{DateTime, Datelike, Utc};
use std::collections::HashSet;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::time::{Duration, SystemTime};
use structopt::StructOpt;

#[derive(StructOpt)]
struct Cli {
    #[structopt(short, long)]
    dry: bool,

    #[structopt(short, long)]
    create: bool,

    #[structopt(parse(from_os_str))]
    src: std::path::PathBuf,

    #[structopt(parse(from_os_str))]
    dst: std::path::PathBuf,
}

struct Config {
    destination: std::path::PathBuf,
    image_extensions: HashSet<String>,
    other_extensions: HashSet<String>,
    folders_to_skip: HashSet<String>,
    min_size: u64,
    dry: bool,
}

impl Config {
    pub fn new(
        destination: std::path::PathBuf,
        image: &str,
        other: &str,
        folders: &str,
        dry: bool,
        min_size: u64,
    ) -> Self {
        let image_extensions = image.split('|').map(|x| x.to_string()).collect();
        let other_extensions = other.split('|').map(|x| x.to_string()).collect();
        let folders_to_skip = folders.split('|').map(|x| x.to_string()).collect();

        Config {
            destination,
            dry,
            image_extensions,
            other_extensions,
            folders_to_skip,
            min_size,
        }
    }
}

fn main() {
    let args = Cli::from_args();

    if !args.src.exists() {
        panic!("Source doesn't exist: {}", args.src.display())
    }

    if args.dst.exists() {
        if !args.dst.is_dir() {
            panic!("Destination is not a directory {}", args.dst.display())
        }
    } else {
        if args.create == false {
            panic!(
                "Destination doesn't exist and create is not enabled {}",
                args.dst.display()
            );
        }
        if !args.dry {
            fs::create_dir_all(args.dst.as_path())
                .expect(&format!("Unable to create {}", args.dst.display()));
        }
    }

    let config = Config::new(
        args.dst,
        "afphoto|ai|awi|awf|arw|bmp|cr2|dng|heic|jpg|jpeg|mov|mp4|mts|nef|orf|png|psd|raf|rw2|srw|tif|tiff|x3f",
        "comask|exposurex6|cocatalogdb|backup|backup 1|doc|xls",
        "com.apple.mediaanalysisd|caches|database|com.apple.photoanalysisda|Cache|Thumbnails|resources|com.apple.mediaanalysisd|resources|private|Previews",
        args.dry,
        40960,
    );

    process(args.src, &config);
}

fn process(source: std::path::PathBuf, config: &Config) {
    if source.is_dir() {
        for entry in source
            .read_dir()
            .expect(&format!("read_dir failed at {}", source.display()))
        {
            if let Ok(entry) = entry {
                let filename = entry.file_name().to_str().unwrap().to_string();
                if filename.starts_with(".") || config.folders_to_skip.contains(&filename) {
                    println!("Skip {}", &filename)
                } else {
                    process(entry.path(), config)
                }
            }
        }
    } else if source.is_file() {
        let ext = source
            .extension()
            .map(|x| x.to_ascii_lowercase().to_str().map(|x| Some(x.to_string())))
            .flatten()
            .flatten();
        if ext.is_some() {
            let ext = ext.unwrap();

            if config.image_extensions.contains(&ext) {
                let size_of = source
                    .metadata()
                    .expect(&format!("Len expected {}", source.display()))
                    .len();

                if size_of < config.min_size {
                    println!("Size is too small {} {}", source.display(), size_of);
                    return;
                }
                if let Ok(created) = source.metadata().and_then(|x| match x.created() {
                    Ok(created) => Ok(created),
                    _ => Ok(SystemTime::UNIX_EPOCH + Duration::from_secs(x.ctime() as u64)),
                }) {
                    let dt = DateTime::<Utc>::from(created);
                    let mut pathname = config.destination.clone();
                    pathname.push(dt.year().to_string());
                    pathname.push(format!("{:02}", dt.month()));
                    pathname.push(format!("{:02}", dt.day()));
                    if !config.dry && !pathname.exists() {
                        fs::create_dir_all(pathname.as_path())
                            .expect(&format!("Unable to create {}", pathname.display()));
                    }
                    pathname.push(source.file_name().unwrap());
                    if !pathname.exists() {
                        if !config.dry {
                            fs::copy(&source, &pathname).expect(&format!(
                                "Unable to copy {} -> {}",
                                source.display(),
                                pathname.display()
                            ));
                            println!("Copy {} {}", source.display(), pathname.display())
                        }
                    } else {
                        println!("Skip {}", source.display())
                    }
                }
            } else if config.other_extensions.contains(&ext) == false {
                println!("Unknown {}", ext)
            }
        }
    }
}
