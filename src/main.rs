use chrono::{DateTime, Datelike, Utc};
use filetime::{set_file_mtime, FileTime};
use std::collections::HashSet;
use std::fs;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::io::BufWriter;
use std::time::{Duration, SystemTime};
use structopt::StructOpt;

#[derive(StructOpt)]
struct Cli {
    #[structopt(short, long)]
    dry: bool,

    #[structopt(short, long)]
    create: bool,

    #[structopt(short, long)]
    overwrite: bool,

    #[structopt(parse(from_os_str))]
    src: Vec<std::path::PathBuf>,

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
    overwrite: bool,
}

impl Config {
    pub fn new(
        destination: std::path::PathBuf,
        image: &str,
        other: &str,
        folders: &str,
        dry: bool,
        overwrite: bool,
        min_size: u64,
    ) -> Self {
        let image_extensions = image.split('|').map(|x| x.to_string()).collect();
        let other_extensions = other.split('|').map(|x| x.to_string()).collect();
        let folders_to_skip = folders.split('|').map(|x| x.to_string()).collect();

        Config {
            destination,
            image_extensions,
            other_extensions,
            folders_to_skip,
            min_size,
            dry,
            overwrite,
        }
    }
}

fn main() {
    let args = Cli::from_args();

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
        "afphoto|ai|arw|awf|awi|bmp|cr2|crw|dng|heic|jpe|jpeg|jpg|mkv|mov|mp4|mrw|mrw2|mts|nef|orf|pef|png|psd|raf|rw2|srw|tif|tiff|x3f",
        "asd|backup|backup 1|cocatalogdb|comask|cos|cue|db|doc|exposurex6|gif|htm|ini|itc|itdb|itl|log|md5|nfo|on1|ovw|pdf|pp3|rtf|sfv|spd|spi|thm|trashed|txt|url|vbe|xls|xml|xmp",
        "Cache|Mobile Applications|Podcasts|Previews|Settings50|Thumbnails|Thumbs.db|caches|com.apple.mediaanalysisd|com.apple.photoanalysisda|database|private|resources",
        args.dry,
        args.overwrite,
        10240,
    );

    for s in args.src {
        if !s.exists() {
            panic!("Source doesn't exist: {}", s.display())
        }
        process(s, &config);
    }
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
                let metadata = source
                    .metadata()
                    .expect(&format!("Len expected {}", source.display()));
                let size_of = metadata.len();

                if size_of < config.min_size {
                    println!("Size is too small {} {}", source.display(), size_of);
                    return;
                }
                let mt = FileTime::from_last_modification_time(&metadata);
                let dt = DateTime::<Utc>::from(
                    SystemTime::UNIX_EPOCH + Duration::from_secs(mt.unix_seconds() as u64),
                );

                let mut pathname = config.destination.clone();
                pathname.push(dt.year().to_string());
                pathname.push(format!("{:02}", dt.month()));
                pathname.push(format!("{:02}", dt.day()));
                if !config.dry && !pathname.exists() {
                    fs::create_dir_all(pathname.as_path())
                        .expect(&format!("Unable to create {}", pathname.display()));
                }
                pathname.push(source.file_name().unwrap());
                let exists = pathname.exists();
                if config.overwrite || !exists {
                    if !config.dry {
                        mycopy(&source, &pathname).expect(&format!(
                            "Unable to copy {} -> {}",
                            source.display(),
                            pathname.display()
                        ));
                        if let Some(e) = set_file_mtime(&pathname, mt).err() {
                            println!("Error while {} {}", source.display(), e);
                        }
                        println!(
                            "{} {} {}",
                            if exists { "Overwrite" } else { "Copy" },
                            source.display(),
                            pathname.display()
                        )
                    } else {
                        println!("Would copy {} {}", source.display(), pathname.display())
                    }
                } else {
                    println!("Skip {}", source.display())
                }
            } else if config.other_extensions.contains(&ext) == false {
                println!("Unknown {}", ext)
            }
        }
    }
}

fn mycopy(from: &std::path::Path, to: &std::path::Path) -> io::Result<usize> {
    let f = File::open(from)?;

    let metadata = f.metadata()?;
    if !metadata.is_file() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Not a file"));
    }

    let mut buffer = Vec::new();
    let mut reader = BufReader::new(f);
    let len = reader.read_to_end(&mut buffer)?;
    let dest = File::create(to)?;
    let mut writer = BufWriter::new(dest);
    writer.write_all(&mut buffer)?;
    writer.flush()?;

    set_file_mtime(to, FileTime::from_last_modification_time(&metadata))?;
    Ok(len)
}
