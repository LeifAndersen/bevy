use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use directories::ProjectDirs;
use prettytable::{format::FormatBuilder, row, Table};
use rust_embed::RustEmbed;
use rust_i18n::t;
use std::{env, fs, fs::File, path::PathBuf, vec::Vec};
use tera::Tera;

rust_i18n::i18n!();

//
// Command Line Parsing
//

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
    #[command(about=t!("uninstall_template"))]
    Uninstall { template: String },
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

impl ToString for License {
    fn to_string(&self) -> String {
        String::from(match self {
            License::ApacheV2 => "Apache-2.0",
            License::Mit => "MIT",
            _ => "Other",
        })
    }
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

//
// Template Management
//

#[derive(RustEmbed)]
#[folder = "assets/default_template/"]
struct DefaultTemplate;

/// Returns a Tera object with the default template loaded.
///
/// Note that binary files are NOT loaded!
fn default_template_tera() -> Result<Tera> {
    let mut tera = Tera::default();
    for filename in DefaultTemplate::iter() {
        let filename = filename.to_string();
        let file = DefaultTemplate::get(&filename)
            .with_context(|| format!("{}: {}", t!("err_no_read"), &filename))?;
        if let Ok(file) = String::from_utf8(file.data.as_ref().to_vec()) {
            tera.add_raw_template(&filename, &file)?;
        }
    }
    Ok(tera)
}

/// Returns the path to the templates directory, creates it if it doesn't exist.
fn templates_dir() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("org", "Bevy Engine", "Bevy").context(t!("err_no_data"))?;
    let templates = dirs.data_dir().join("templates");
    fs::create_dir_all(&templates)?;
    Ok(templates)
}

//
// Project Management
//

/// Creates a new project in the given directory.
fn init_project(path: &PathBuf, opts: &ProjectOpts) -> Result<()> {
    // Complete and validate opts
    let mut ctx = tera::Context::new();
    let name = match &opts.name {
        Some(name) => name as &str,
        None => path
            .file_name()
            .context(t!("err_invalid_dir_name"))?
            .to_str()
            .context(t!("err_invalid_dir_name"))?,
    };
    ctx.insert("name", name);
    let license = if opts.license.is_empty() {
        vec![License::ApacheV2, License::Mit]
    } else if opts.license.contains(&License::NoLicense) {
        vec![]
    } else {
        opts.license.clone()
    };
    ctx.insert(
        "license",
        &license
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<String>>()
            .join(" OR "),
    );

    // Pick appropriate template
    let template = default_template_tera()?;

    // Write files
    for filename_str in template.get_template_names() {
        let filename = path.join(filename_str);
        let folder = filename
            .parent()
            .with_context(|| format!("{}: {}", t!("err_folder_for"), filename_str))?;
        fs::create_dir_all(folder)?;
        let mut file = File::create(filename)?;
        template.render_to(filename_str, &ctx, &mut file)?;
    }
    for filename_str in DefaultTemplate::iter() {
        let filename = path.join(filename_str.to_string());
        let folder = filename
            .parent()
            .with_context(|| format!("{}: {}", t!("err_folder_for"), filename_str))?;
        fs::create_dir_all(folder)?;
        let file = DefaultTemplate::get(&filename_str)
            .with_context(|| format!("{}: {}", t!("err_no_read"), &filename_str))?;
        fs::write(filename, file.data)?;
    }
    Ok(())
}

/// Creates the given folder and calls `init_project` on that folder.
fn new_project(path: &PathBuf, opts: &ProjectOpts) -> Result<()> {
    fs::create_dir(&path)?;
    init_project(path, opts)
}

/// Main entry point for the Bevy CLI.
pub fn cli() -> Result<()> {
    let args = Cli::parse();
    match args.command {
        Commands::New { path, project_opts } => new_project(&PathBuf::from(&path), &project_opts)
            .with_context(|| format!("Could not create `{}`", &path)),

        Commands::Init { project_opts } => init_project(&env::current_dir()?, &project_opts)
            .with_context(|| format!("Could not initialize project")),

        Commands::Templates { command } => match command {
            TemplateCommands::List { .. } => {
                println!("{}:\n", t!("installed_templates"));
                let mut table = Table::new();
                table.set_format(FormatBuilder::new().padding(1, 1).build());
                for template in fs::read_dir(templates_dir()?)? {
                    let template = template?;
                    if template.path().is_dir() {
                        table.add_row(row!["🦀", template.file_name().to_str().context("TODO")?]);
                    }
                }
                table.printstd();
                Ok(())
            }
            TemplateCommands::Uninstall { .. } => todo!(),
        },
    }
}
