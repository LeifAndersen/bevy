use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand, ValueEnum};
use directories::ProjectDirs;
use prettytable::{format::FormatBuilder, row, Table};
use rust_embed::RustEmbed;
use rust_i18n::t;
use std::{env, fs, fs::File, io, io::Read, path::PathBuf, vec::Vec};
use tera::Tera;
use zip::ZipArchive;

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
        path: Option<String>,

        #[command(flatten)]
        project_opts: ProjectOpts,
    },
    #[command(about=t!("manage_templates"))]
    Templates {
        #[command(subcommand)]
        command: TemplateCommands,
    },
}

#[derive(Subcommand)]
enum TemplateCommands {
    #[command(about=t!("install_template"))]
    Install { name: String },
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
            License::Cc0 => "CC0-1.0",
            _ => "Other",
        })
    }
}

#[derive(Clone, PartialEq, Eq, ValueEnum)]
enum ContinuousIntegration {
    Github,
    None,
}

impl ContinuousIntegration {
    fn paths(&self) -> (PathBuf, PathBuf) {
        match self {
            ContinuousIntegration::Github => ("github".into(), ".github".into()),
            _ => ("".into(), "".into()),
        }
    }
}

#[derive(Default, Args)]
#[group(required = false, multiple = true)]
struct ProjectOpts {
    #[arg(long, short, help=t!("project_name"))]
    name: Option<String>,
    #[arg(long, short, help=t!("project_license"))]
    license: Vec<License>,
    #[arg(long, short, help=t!("continuous_integration"))]
    continuous_integration: Vec<ContinuousIntegration>,
    #[arg(long, short, help=t!("project_template"))]
    template: Option<String>,
}

//
// Template Management
//

#[derive(RustEmbed)]
#[folder = "assets/licenses/"]
struct LicenseFles;

#[derive(RustEmbed)]
#[folder = "assets/default_template/"]
struct DefaultTemplate;

#[derive(RustEmbed)]
#[folder = "assets/ci"]
struct CIFiles;

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

pub const BEVY_TEMPLATE_DIR: &str = "BEVY_TEMPLATE_DIR";

/// Returns the path to the templates directory, creates it if it doesn't exist.
fn templates_dir() -> Result<PathBuf> {
    let templates = match env::var(BEVY_TEMPLATE_DIR) {
        Ok(dir) => PathBuf::from(dir),
        Err(_) => {
            let dirs =
                ProjectDirs::from("org", "Bevy Engine", "Bevy").context(t!("err_no_data"))?;
            dirs.data_dir().join("templates")
        }
    };
    fs::create_dir_all(&templates)?;
    Ok(templates)
}

/// Creates a tera object from the files in a template
///
/// Note that binary files are NOT loaded!
fn template_zip_terra<T: io::Read + io::Seek>(zip: &mut ZipArchive<T>) -> Result<Tera> {
    let mut tera = Tera::default();
    for i in 0..zip.len() {
        let mut file = zip.by_index(i)?;
        if file.is_file() {
            let mut data = String::new();
            if let Ok(_) = file.read_to_string(&mut data) {
                let filename = file.enclosed_name().context(t!("err_no_read"))?;
                let filename = filename.to_str().context(t!("err_no_read"))?;
                tera.add_raw_template(filename, &data)?;
            }
        }
    }
    Ok(tera)
}

/// Creates a tera object for all CI files.
///
/// Note: These are expected to be entirely text files.
fn continuous_integration_tera() -> Result<Tera> {
    let mut tera = Tera::default();
    for filename in CIFiles::iter() {
        let filename = filename.to_string();
        let file = CIFiles::get(&filename)
            .with_context(|| format!("{}: {}", t!("err_no_read"), &filename))?;
        if let Ok(file) = String::from_utf8(file.data.as_ref().to_vec()) {
            tera.add_raw_template(&filename, &file)?;
        }
    }
    Ok(tera)
}

//
// Project Management
//

/// Creates a new project in the given directory.
fn init_project(path: &PathBuf, opts: &ProjectOpts) -> Result<()> {

    // Ensure current dir is empty
    if fs::read_dir(path)?.count() != 0 {
        bail!("{}: {}", t!("err_not_empty: {}"), path.to_string_lossy());
    }

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
    let continuous_integration = if opts.continuous_integration.is_empty() {
        vec![ContinuousIntegration::Github]
    } else if opts
        .continuous_integration
        .contains(&ContinuousIntegration::None)
    {
        vec![]
    } else {
        opts.continuous_integration.clone()
    };

    // Pick appropriate template
    let template = match &opts.template {
        Some(template) => {
            let tpath = templates_dir()?.join(format!("{}.zip", template));
            let tfile = File::open(tpath).context(t!("err_no_template"))?;
            template_zip_terra(&mut ZipArchive::new(tfile)?)?
        }
        None => default_template_tera()?,
    };

    // Write template files
    for filename_str in template.get_template_names() {
        let filename = path.join(filename_str);
        let folder = filename
            .parent()
            .with_context(|| format!("{}: {}", t!("err_folder_for"), filename_str))?;
        fs::create_dir_all(folder)?;
        let mut file = File::create(filename)?;
        template.render_to(filename_str, &ctx, &mut file)?;
    }
    if let None = &opts.template {
        // TODO, also need to do this for explicit templates
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
    }

    // Write license files
    for license in license {
        let license = license.to_string();
        let text = LicenseFles::get(&license)
            .with_context(|| format!("{}: {}", t!("err_no_license"), license))?;
        fs::write(path.join(format!("LICENSE-{}", license)), text.data)?;
    }

    // Write CI files
    let ci_template = continuous_integration_tera()?;
    for ci in continuous_integration {
        let (ci, ci_path) = ci.paths();
        for filename in ci_template.get_template_names() {
            let filename_path = PathBuf::from(filename);
            if filename_path.starts_with(&ci) {
                let out_filename = path.join(&ci_path).join(filename_path.strip_prefix(&ci)?);
                fs::create_dir_all(out_filename.parent().unwrap())?;
                let mut file = File::create(out_filename)?;
                ci_template.render_to(filename, &ctx, &mut file)?;
            }
        }
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

        Commands::Init { path, project_opts } => {
            let proot = &env::current_dir()?;
            let proot = match path {
                Some(path) => proot.join(path),
                None => proot.to_owned(),
            };
            init_project(&proot, &project_opts)
                .with_context(|| format!("Could not initialize project"))
        }

        Commands::Templates { command } => match command {
            TemplateCommands::List { .. } => {
                println!("{}:\n", t!("installed_templates"));
                let mut table = Table::new();
                table.set_format(FormatBuilder::new().padding(1, 1).build());
                for template in fs::read_dir(templates_dir()?)? {
                    let template = template?;
                    if template.path().is_file() {
                        table.add_row(row!["🦀", template.file_name().to_str().context("TODO")?]);
                    }
                }
                table.printstd();
                Ok(())
            }
            TemplateCommands::Install { .. } => todo!(),
            TemplateCommands::Uninstall { .. } => todo!(),
        },
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use assert_fs::prelude::*;
    use predicates::prelude::*;

    #[test]
    fn test_default_init_project() {
        let opts = ProjectOpts::default();
        let tempdir = assert_fs::TempDir::new().unwrap();
        let projdir = tempdir.path().join("bobgame");
        assert!(new_project(&projdir, &opts).is_ok());
        tempdir.child("bobgame").assert(predicate::path::is_dir());
        tempdir
            .child("bobgame")
            .child(".github")
            .assert(predicate::path::is_dir());
        tempdir
            .child("bobgame")
            .child("LICENSE-Apache-2.0")
            .assert(predicate::path::is_file());
        tempdir
            .child("bobgame")
            .child("Cargo.toml")
            .assert(predicate::path::is_file());
    }

    #[test]
    fn test_no_extras() {
        let opts = ProjectOpts {
            license: vec![License::NoLicense],
            continuous_integration: vec![ContinuousIntegration::None],
            ..ProjectOpts::default()
        };
        let tempdir = assert_fs::TempDir::new().unwrap();
        let projdir = tempdir.path().join("bobgame");
        assert!(new_project(&projdir, &opts).is_ok());
        tempdir
            .child("bobgame")
            .child(".github")
            .assert(predicate::path::missing());
        tempdir
            .child("bobgame")
            .child("LICENSE-Apache-2.0")
            .assert(predicate::path::missing());
    }

    #[test]
    fn test_build_zip_terra() {
        let template_path = PathBuf::from("assets/tests/simple.zip");
        let template = File::open(template_path).unwrap();
        let mut archive = ZipArchive::new(template).unwrap();
        let tera = template_zip_terra(&mut archive).unwrap();
        assert_eq!(2, tera.get_template_names().count());
    }
}
