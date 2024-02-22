use std::{
    ffi::OsStr,
    fs::{self, File},
    io::{BufRead, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
};

use globset::{Glob, GlobSet, GlobSetBuilder};
use walkdir::WalkDir;

use crate::Result;

// File Bundler
pub fn bundle_to_file(files: Vec<PathBuf>, dst_file: &Path) -> Result<()> {
    let mut writer = BufWriter::new(File::create(dst_file)?);

    for file in files {
        if !file.is_file() {
            return Err(format!("Cannot Bundle '{:?}' is not a file", file).into());
        }
        let reader = BufReader::new(File::open(&file)?);

        writeln!(writer, "\n// ==== File Path: {}\n", file.to_string_lossy())?;

        for line in reader.lines() {
            let line = line?;
            writeln!(writer, "{}", line)?;
        }
        writeln!(writer, "\n\n")?;
    }
    writer.flush()?;

    Ok(())
}

// File Parser/Writer
pub fn load_from_toml<T>(file: impl AsRef<Path>) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let content = read_to_string(file.as_ref())?;

    Ok(toml::from_str(&content)?)
}

pub fn load_from_json<T>(file: impl AsRef<Path>) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let content = serde_json::from_reader(get_reader(file.as_ref())?)?;

    Ok(content)
}

pub fn save_to_json<T>(file: impl AsRef<Path>, data: &T) -> Result<()>
where
    T: serde::Serialize,
{
    let file = file.as_ref();

    let file =
        File::create(file).map_err(|err| format!("Cannot Create File '{:?}': {}", file, err))?;

    serde_json::to_writer_pretty(file, data)?;

    Ok(())
}

// Dir Utils
// * Returns true if one or more dir was created
pub fn ensure_dir(dir: &Path) -> Result<bool> {
    if dir.is_dir() {
        Ok(false) // Returns false if dir is already present
    } else {
        fs::create_dir_all(dir)?;
        Ok(true)
    }
}

pub fn list_files(
    dir: &Path,
    include_globs: Option<&[&str]>,
    exclude_globs: Option<&[&str]>,
) -> Result<Vec<PathBuf>> {
    let base_dir_exclude = base_dir_exclude_globs()?;

    // Determine Recursive Depth
    // If there is a ** in the glob, make recursion. If there is * in the glob, is a flat file and do not make recursion
    let depth = include_globs
        .map(|globs| globs.iter().any(|&glob| glob.contains("**")))
        .map(|value| if value { 100 } else { 1 }) // TODO: The value 100 and 1 could be in a Constant
        .unwrap_or(1);

    // Prepare Globs
    let include_globs = include_globs.map(get_glob_set).transpose()?;
    let exclude_globs = exclude_globs.map(get_glob_set).transpose()?;

    // Build File Iterator
    let walk_dir_iterator = WalkDir::new(dir)
        .max_depth(depth)
        .into_iter()
        .filter_entry(|entry|
            // if dir, check the dir exclude
            if entry.file_type().is_dir() {
                !base_dir_exclude.is_match(entry.path())
            }
            // else is file, we apply the globs
            else {
                // Evaluate the exclude
                if let Some(exclude_globs) = exclude_globs.as_ref() {
                    if exclude_globs.is_match(entry.path()) {
                        return false;
                    }
                }
                // Otherwise, evaluate the include
                match include_globs.as_ref() {
                    Some(globs) => globs.is_match(entry.path()),
                    None => true,
                }
            }
        )
        .filter_map(|entry| entry.ok().filter(|dir_entry| dir_entry.file_type().is_file()));

    let paths = walk_dir_iterator.map(|entry| entry.into_path());

    Ok(paths.collect())
}

fn base_dir_exclude_globs() -> Result<GlobSet> {
    get_glob_set(&["**/.git", "**/target", "**/.env", "**/.env.sample"])
}

pub fn get_glob_set(globs: &[&str]) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for glob in globs {
        builder.add(Glob::new(glob)?);
    }
    Ok(builder.build()?)
}

// File Utils
fn get_reader(file: &Path) -> Result<BufReader<File>> {
    let Ok(file) = File::open(file) else {
        return Err(format!("File Not Found: {}", file.display()).into());
    };

    Ok(BufReader::new(file))
}

pub fn read_to_string(file: &Path) -> Result<String> {
    if !file.is_file() {
        return Err(format!("File Not Found: {}", file.display()).into());
    }
    let content = fs::read_to_string(file)?;

    Ok(content)
}

// XFile
// Trait that has methods which return the `&str` when Ok, and When None or Err, return ""
pub trait XFile {
    fn x_file_name(&self) -> &str;
    fn x_extension(&self) -> &str;
}

impl XFile for Path {
    fn x_file_name(&self) -> &str {
        self.file_name().and_then(OsStr::to_str).unwrap_or("")
    }

    fn x_extension(&self) -> &str {
        self.extension().and_then(OsStr::to_str).unwrap_or("")
    }
}
