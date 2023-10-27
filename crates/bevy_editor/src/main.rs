use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::{env, error::Error, fs, path::PathBuf};

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
        New {
            path: String,

            #[command(flatten)]
            project_opts: ProjectOpts,
        },
        /// init_project_here
        Init {
            #[command(flatten)]
            project_opts: ProjectOpts,
        },
    }
}

#[derive(Clone,ValueEnum)]
enum License {
    other,
    cc0,
    apachev2,
    mit,
    gplv2,
    gplv3,
}


//localize_args!{
#[derive(Default, Args)]
#[group(required = false, multiple = true)]
struct ProjectOpts {
    #[arg(long, short)]
    /// project_name
    name: Option<String>,
    #[arg(long, short)]
    /// project_license
    license: Vec<License>,
}
//}

/// Creates a new project in the current directory.
fn init_project(path: PathBuf, opts: ProjectOpts) -> Result<()> {
    Ok(())
}

/// Creates the given folder and calls `init_project` on that folder.
fn new_project(path: PathBuf, opts: ProjectOpts) -> Result<()> {
    fs::create_dir(&path).unwrap();
    init_project(path, opts)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();
    match args.command {
        Commands::New { path, project_opts } => {
            new_project((&path).into(), ProjectOpts::default())
                .with_context(|| format!("Could not create `{}`", &path))?;
            Ok(())
        }
        Commands::Init { project_opts } => {
            init_project(env::current_dir()?, ProjectOpts::default())
                .with_context(|| format!("Could not initialize project"))?;
            Ok(())
        }
    }
}
