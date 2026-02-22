mod website;

use std::path::Path;

use clap::{Parser, Subcommand};
use website::Website;

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new clutterlog site
    New {
        /// Name of the site to create
        site_name: String,
    },
    /// Build the site in the current directory
    Build,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::New { site_name } => {
            let path = Path::new(&site_name);
            match Website::new(path) {
                Ok(website) => {
                    println!(
                        "Created new site '{}' at '{}'",
                        website.info.title,
                        website.path.display()
                    );
                }
                Err(e) => {
                    eprintln!("Error creating site: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Build => {
            let path = Path::new(".");
            match Website::load(path) {
                Ok(website) => match website.build() {
                    Ok(report) => {
                        println!("Site '{}' built successfully\n", website.info.title);
                        println!("{}", report);
                    }
                    Err(e) => {
                        eprintln!("Error building site: {}", e);
                        std::process::exit(1);
                    }
                },
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}
