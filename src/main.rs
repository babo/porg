use chrono::{DateTime, Datelike, Utc};
use std::collections::HashSet;
use std::fs;
use structopt::StructOpt;

#[derive(StructOpt)]
struct Cli {
    #[structopt(short, long)]
    dry: bool,

    #[structopt(short, long)]
    create: bool,

    #[structopt(parse(from_os_str))]
    from: std::path::PathBuf,

    #[structopt(parse(from_os_str))]
    to: std::path::PathBuf,
}

struct Config {
    destination: std::path::PathBuf,
    dry: bool,
    image_extensions: HashSet<String>,
    other_extensions: HashSet<String>,
    folders_to_skip: HashSet<String>,
    min_size: u64,
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

    if args.to.exists() {
        if !args.to.is_dir() {
            panic!("Destination is not a directory {}", args.to.display())
        }
    } else {
        if args.create == false {
            panic!(
                "Destination doesn't exist and create is not enabled {}",
                args.to.display()
            );
        }
        if !args.dry {
            fs::create_dir_all(args.to.as_path())
                .expect(&format!("Unable to create {}", args.to.display()));
        }
    }

    let config = Config::new(
        args.to,
        "afphoto|awf|arw|bmp|cr2|dng|heic|jpg|jpeg|mov|mp4|mts|nef|png|raf|rw2|srw|tif|tiff|x3f",
        "comask|exposurex6|cocatalogdb|backup|backup 1",
        "com.apple.mediaanalysisd|caches|database|com.apple.photoanalysisda|Cache|Thumbnails|resources|com.apple.mediaanalysisd|resources|private",
        args.dry,
        40960,
    );

    process(args.from, &config);
}

fn process(source: std::path::PathBuf, config: &Config) {
    if !source.exists() {
        panic!("Source doesn't exist: {}", source.display())
    }

    if source.is_dir() {
        for entry in source
            .read_dir()
            .expect(&format!("read_dir failed at {}", source.display()))
        {
            if let Ok(entry) = entry {
                let filename = entry.file_name().to_str().unwrap().to_string();
                if !filename.starts_with(".") && !config.folders_to_skip.contains(&filename) {
                    process(entry.path(), config)
                } else {
                    println!("Skip {}", &filename)
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
            let ext = String::from(ext.unwrap());

            if config.image_extensions.contains(&ext) {
                let size_of = source
                    .metadata()
                    .expect(&format!("Len expected {}", source.display()))
                    .len();

                if size_of < config.min_size {
                    println!("Size is too small {} {}", source.display(), size_of);
                    return;
                }
                if let Ok(created) = source.metadata().and_then(|x| x.created()) {
                    let dt = DateTime::<Utc>::from(created);
                    let mut pathname = config.destination.clone();
                    pathname.push(dt.year().to_string());
                    pathname.push(format!("{:02}", dt.month()));
                    pathname.push(format!("{:02}", dt.day()));
                    if !pathname.exists() && !config.dry {
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
                            println!("copy {}", source.display())
                        }
                    } else {
                        println!("skip {}", source.display())
                    }
                }
            } else if config.other_extensions.contains(&ext) == false {
                println!("Unknown {}", ext)
            }
        }
    }
}
