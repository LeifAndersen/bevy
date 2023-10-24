use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use rust_i18n::t;
use std::{error::Error, fs};

mod localize;
use localize::*;

localize! {
    #[derive(Parser)]
    #[command(name = t!("bevy_editor"), author, version)]
    /// bevy_command_line_tool
    struct Cli {
        #[command(subcommand)]
        command: Commands,
    }
    
    #[derive(Subcommand)]
    enum Commands {
        /// start_new_project
        New { name: String },
        /// init_project_here
        Init {},
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();
    match args.command {
        Commands::New { name } => {
            let name = &name;
            fs::create_dir(name).with_context(|| format!("Could not create `{}`", name))?;
            println!("Hello {}", name);
            Ok(())
        }
        Commands::Init {} => {
            println!("Start new project here.");
            Ok(())
        }
    }
}
