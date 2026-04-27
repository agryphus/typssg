use std::env;
use std::path::PathBuf;

use clap::Parser;
use typssg::{compile_all, compile_article};
use log::{info, error};

#[derive(Parser)]
struct Args {
    #[arg(default_value = ".")]
    dir: PathBuf,

    #[arg(long)]
    prepend: Option<PathBuf>,

    #[arg(long, value_delimiter = ',')]
    plugin: Vec<String>,

    #[arg(short)]
    recursive: bool,

    #[arg(long)]
    include_title_in_outline: bool,
}

fn main() {
    env_logger::init();

    match env::current_dir() {
        Ok(path) => info!("Starting in working directory: {}", path.display()),
        Err(e) => error!("Error getting current directory: {}", e),
    }

    let args = Args::parse();

    let result = if args.recursive {
        compile_all(
            &args.dir,
            &args.prepend,
            &args.plugin,
            args.include_title_in_outline,
        )
    } else {
        compile_article(
            &args.dir,
            &args.prepend,
            &args.plugin,
            args.include_title_in_outline,
        )
    };

    if let Err(e) = result {
        error!("{e}");
        std::process::exit(1);
    }
}
