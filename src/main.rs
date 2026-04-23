use std::env;
use std::path::PathBuf;

use clap::Parser;
use typssg::{compile_article, compile_all};

#[derive(Parser)]
struct Args {
    #[arg(default_value = ".")]
    dir: PathBuf,

    #[arg(long)]
    prepend: Option<PathBuf>,

    #[arg(short)]
    recursive: bool,
}

fn main() {
    match env::current_dir() {
        Ok(path) => println!("Current working directory: {}", path.display()),
        Err(e) => eprintln!("Error getting current directory: {}", e),
    }

    let args = Args::parse();

    let result = if args.recursive {
        compile_all(&args.dir, &args.prepend)
    } else {
        compile_article(&args.dir, &args.prepend)
    };

    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
