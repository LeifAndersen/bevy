use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use rust_i18n::t;
use std::{env, fs, path::PathBuf};

rust_i18n::i18n!();

#[derive(Parser)]
#[command(name = t!("bevy_editor"), author, version, about=t!("bevy_command_line_tool"))]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about=t!("start_new_project"))]
    New {
        path: String,

        #[command(flatten)]
        project_opts: ProjectOpts,
    },
    #[command(about=t!("init_project_here"))]
    Init {
        #[command(flatten)]
        project_opts: ProjectOpts,
    },
    #[command(about=t!("manage_template"))]
    Templates {
        #[command(subcommand)]
        command: TemplateCommands,
    },
}

#[derive(Subcommand)]
enum TemplateCommands {
    #[command(about=t!("list_templates"))]
    List {},
}

#[derive(Clone, PartialEq, Eq, ValueEnum)]
enum License {
    Other,
    Cc0,
    ApacheV2,
    Mit,
    GplV2,
    GplV3,
    NoLicense,
}

#[derive(Default, Args)]
#[group(required = false, multiple = true)]
struct ProjectOpts {
    #[arg(long, short, help=t!("project_name"))]
    name: Option<String>,
    #[arg(long, short, help=t!("project_license"))]
    license: Vec<License>,
    #[arg(long, short, help=t!("project_template"))]
    template: Option<String>,
}

/// Creates a new project in the given directory.
fn init_project(path: &PathBuf, opts: &ProjectOpts) -> Result<()> {
    // Complete and validate opts structure
    let _name = match &opts.name {
        Some(name) => name,
        None => path
            .file_name()
            .context(t!("invalid_directory_name"))?
            .to_str()
            .context(t!("invalid_directory_name"))?,
    };
    let _license = if opts.license.is_empty() {
        vec![License::ApacheV2, License::Mit]
    } else if opts.license.contains(&License::NoLicense) {
        vec![]
    } else {
        opts.license.clone()
    };
    Ok(())
}

/// Creates the given folder and calls `init_project` on that folder.
fn new_project(path: &PathBuf, opts: &ProjectOpts) -> Result<()> {
    fs::create_dir(&path).unwrap();
    init_project(path, opts)
}

pub fn cli() -> Result<()> {
    let args = Cli::parse();
    match args.command {
        Commands::New { path, project_opts } => new_project(&PathBuf::from(&path), &project_opts)
            .with_context(|| format!("Could not create `{}`", &path)),
        Commands::Init { project_opts } => init_project(&env::current_dir()?, &project_opts)
            .with_context(|| format!("Could not initialize project")),
        Commands::Templates { .. } => todo!(),
    }
}
