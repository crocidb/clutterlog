use clap::{Parser, Subcommand};

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
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::New { site_name } => {
            println!("Creating new site: {}", site_name);
        }
    }
}
