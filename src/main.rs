use std::env;
use std::path::PathBuf;

use clap::Parser;
use typssg::compile_article;

#[derive(Parser)]
struct Args {
    #[arg(default_value = ".")]
    dir: PathBuf,

    #[arg(long)]
    prepend: Option<PathBuf>,
}

fn main() {
    match env::current_dir() {
        Ok(path) => println!("Current working directory: {}", path.display()),
        Err(e) => eprintln!("Error getting current directory: {}", e),
    }

    let args = Args::parse();

    if let Err(e) = compile_article(args.dir, args.prepend) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
