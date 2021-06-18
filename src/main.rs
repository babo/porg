use chrono::{DateTime, Datelike, Utc};
use std::fs;
use structopt::StructOpt;
use std::collections::HashSet;

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

    let known_extensions: HashSet<&'static str> = "awf arw bmp cr2 heic jpg jpeg mov mp4 mts nef png raf rw2 srw tif tiff x3f".split(' ').collect();

    process(args.from, &args.to, &known_extensions, &args.dry);
}

fn process(source: std::path::PathBuf, dest: &std::path::PathBuf, known_extensions: &HashSet<&'static str>, dry: &bool) {
    if !source.exists() {
        panic!("Source doesn't exist: {}", source.display())
    }

    if source.is_dir() {
        for entry in source
            .read_dir()
            .expect(&format!("read_dir failed at {}", source.display()))
        {
            if let Ok(entry) = entry {
                if entry
                    .file_name()
                    .to_str()
                    .and_then(|x| Some(x.starts_with(".")))
                    == Some(false)
                {
                    process(entry.path(), dest, known_extensions, dry)
                }
            }
        }
    } else if source.is_file() {
        let extension = source.extension();
        if extension.is_some() {
            extension.map(|x| {
                if let Some(ext) = x.to_ascii_lowercase().to_str() {
                    if !known_extensions.contains(ext) {
                        println!("{}", source.display())
                    }
                }
            });
            if let Ok(created) = source.metadata().and_then(|x| x.created()) {
                let dt = DateTime::<Utc>::from(created);
                let mut pathname = dest.clone();
                pathname.push(dt.year().to_string());
                pathname.push(dt.month().to_string());
                pathname.push(dt.day().to_string());
                if !pathname.exists() && !dry {
                    fs::create_dir_all(pathname.as_path())
                        .expect(&format!("Unable to create {}", pathname.display()));
                }
                pathname.push(source.file_name().unwrap());
                if !pathname.exists() {
                    if !dry {
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
        }
    }
}
