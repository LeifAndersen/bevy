use anyhow::{Context, Result};
use git2::Repository;
use std::{
    collections::HashMap,
    io,
    io::{Read, Write},
    path::PathBuf,
};
use tera::Tera;
use zip::ZipArchive;

/// Compatability layer for tera that _also_ supports bin files.
///
/// Note that this doesn't support all of tera's API,
/// but it should support the whole language.
///
/// We may add support for other templating languages in the future.
#[derive(Default)]
pub struct Templater {
    tera: Tera,
    binaries: HashMap<String, Vec<u8>>,
}

impl Templater {
    /// Creates a tera object from the files in a template
    ///
    /// Note that binary files are NOT loaded!
    pub fn from_zip<T: io::Read + io::Seek>(zip: &mut ZipArchive<T>) -> Result<Self> {
        let mut tera = Templater::default();
        for i in 0..zip.len() {
            let mut file = zip.by_index(i)?;
            if file.is_file() {
                let mut data = String::new();
                if let Ok(_) = file.read_to_string(&mut data) {
                    let filename = file.enclosed_name().context("bad filename")?;
                    let filename = filename
                        .strip_prefix(filename.components().next().context("bad zip")?)
                        .context("bad filename")?
                        .to_str()
                        .context("bad filename")?;
                    tera.add_raw_template(filename, &data)?;
                }
            }
        }
        Ok(tera)
    }

    pub fn from_git(repo: &Repository) -> Result<Self> {
        let mut tera = Templater::default();
        let tree = repo.head()?.peel_to_tree()?;
        tree.walk(
            git2::TreeWalkMode::PreOrder,
            |name, entry| match (|| -> Option<()> {
                if let Some(git2::ObjectType::Blob) = entry.kind() {
                    let path = PathBuf::from(name).join(entry.name().unwrap());
                    let data = entry.to_object(repo).unwrap().peel_to_blob().unwrap();
                    let data = data.content();
                    tera.add_raw_template(&path.to_str()?, &data).ok()?;
                }
                Some(())
            })() {
                Some(_) => git2::TreeWalkResult::Ok,
                None => git2::TreeWalkResult::Abort,
            },
        )?;
        Ok(tera)
    }

    /// Adds a template of given name `name` to the templator.
    ///
    /// Note that if the name ends with .tera, it is assumed to be a tera file, and will
    /// be parsed as such. If you want a literal file to end with .tera, it will need
    /// to have the extension .tera.tera.
    pub fn add_raw_template<T>(&mut self, name: &str, content: &T) -> Result<()>
    where
        Vec<u8>: From<T>,
        T: Clone,
    {
        let content: Vec<u8> = content.clone().into();
        match name.split_once(".tera") {
            Some((name, "")) => {
                match String::from_utf8(content.clone()) {
                    Ok(file) => {
                        self.tera.add_raw_template(&name, &file)?;
                    }
                    _ => {
                        self.binaries.insert(String::from(name), content);
                    }
                };
            }
            _ => {
                self.binaries.insert(String::from(name), content);
            }
        };
        Ok(())
    }

    pub fn get_template_names(&self) -> impl Iterator<Item = &str> {
        self.tera
            .get_template_names()
            .chain(self.binaries.keys().map(|s| s.as_str()))
    }

    pub fn render_to(
        &self,
        template_name: &str,
        context: &tera::Context,
        mut write: impl Write,
    ) -> Result<()> {
        match self.binaries.get(template_name) {
            Some(data) => {
                write.write_all(data)?;
            }
            None => {
                self.tera.render_to(template_name, context, write)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::{collections::HashSet, fs::File, path::PathBuf};

    use super::*;
    #[test]
    fn test_build_zip_tera() {
        let template_path = PathBuf::from("assets/tests/templates/simple.zip");
        let template = File::open(template_path).unwrap();
        let mut archive = ZipArchive::new(template).unwrap();
        let tera = Templater::from_zip(&mut archive).unwrap();
        assert_eq!(4, tera.get_template_names().count());
        assert_eq!(
            tera.get_template_names().collect::<HashSet<&str>>(),
            HashSet::from([
                "Cargo.toml",
                "src/main.rs",
                "src/simple.rs",
                "src/simple_raw.rs"
            ]),
        );
    }
}
