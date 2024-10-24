//! File/tag related functionality
//!
//! This includes functions resolve where to,
//! write to, and read from tag files

use anyhow::{Context, Error, Result};
use app_dirs::{self, AppDataType, AppInfo};
use std::{
    fs::{create_dir_all, read_to_string, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

/// static information about this application
///
/// Used to determine where to put local data on the users filesystem
const APP_INFO: AppInfo = AppInfo {
    name: "evry",
    author: "purarue",
};

/// Keeps track of the user data dir, creates directories if they don't exist
#[derive(Debug, Default)]
pub struct LocalDir {
    pub data_dir: PathBuf,
}

impl LocalDir {
    /// Creates application/data directories if they don't exist
    pub fn new() -> Result<Self, Error> {
        let dir_info: PathBuf = app_dirs::get_app_root(AppDataType::UserData, &APP_INFO)
            .context("Couldn't get user local data directory")?;
        // use EVRY_DIR environment variable, if it exists
        let evry_env = std::env::var("EVRY_DIR");
        let evry_dir: &Path = match evry_env {
            Ok(ref evry_environ) => Path::new(evry_environ),
            Err(_) => dir_info.as_path(),
        };

        // hmm -- not really needed anymore since we don't have any other files there (rollback was
        // removed), but will keep for backwards compatibility
        let data_dir = evry_dir.join("data");
        create_dir_all(&data_dir).context("Could not create evry local directory")?;
        Ok(Self { data_dir })
    }
}

/// read epoch time from a tag file
pub fn read_epoch_millis(filepath: &str) -> Result<u128, Error> {
    let millis_str =
        read_to_string(filepath).context("Could not read tag information from file")?;
    millis_str.trim().parse::<u128>().context(format!(
        "Could not convert tag file contents '{}' to integer for tag '{}'",
        millis_str, filepath
    ))
}

/// A 'tag' is the name of some evry task
///
/// This is used to differentiate
/// different tasks/runs of evry from each other.
///
/// Holds metadata about the tag name,
/// and gives access to the underlying file.
///
/// ```
/// evry 2 months -sometool && run tool
/// evry 10 minutes -requestfile && wget ...
/// ```
#[derive(Debug)]
pub struct Tag {
    /// the name of this tag, like `requestfile`
    pub name: String,
    /// the computed location of this tag, like `~/.local/share/evry/data/requestfile`
    pub path: String,
}

impl Tag {
    /// Creates a new tag, resolves its `path`
    pub fn new(name: String, local_dir: &LocalDir) -> Self {
        let mut buf = local_dir.data_dir.clone();
        buf.push(&name);
        // man, this is ugly
        let path = buf
            .into_os_string()
            .into_string()
            .expect("Could not convert path to string");
        Self { name, path }
    }

    /// Returns whether or not the corresponding tag file exists
    pub fn file_exists(&self) -> bool {
        Path::new(&self.path).exists()
    }

    /// Reads from the tag file, returning when this tag was last run
    pub fn read_epoch_millis(&self) -> Result<u128, Error> {
        read_epoch_millis(&self.path)
    }

    /// Writes a number (epoch datetime) to this tagfile
    pub fn write(&self, time: u128) -> Result<(), Error> {
        let fp = File::create(&self.path).context("Could not create tag file")?;
        let mut writer = BufWriter::new(&fp);
        write!(&mut writer, "{}", time).context("Could not write to file")
    }
}
