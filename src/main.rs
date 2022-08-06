use anyhow::{anyhow, Result};
use clap::Parser;
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Deserialize;
use simple_logger::SimpleLogger;

use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

#[derive(Debug, Deserialize)]
struct BackupRule {
    file_path: String,
    backup_dir_path: String,
}

#[derive(Debug, Deserialize)]
struct ConfigFile {
    rules: Vec<BackupRule>,
}

fn parse_config_file(config_file: &File) -> HashMap<PathBuf, PathBuf> {
    let config: ConfigFile =
        serde_yaml::from_reader(config_file).expect("Failed to parse config file.");

    config
        .rules
        .into_iter()
        .map(|rule| {
            (
                Path::new(&rule.file_path)
                    .canonicalize()
                    .expect("Failed to canonicalize file path."),
                Path::new(&rule.backup_dir_path)
                    .canonicalize()
                    .expect("Failed to canonicalize directory path."),
            )
        })
        .collect::<HashMap<_, _>>()
}

fn start_file_watcher(
    parsed_config: &HashMap<PathBuf, PathBuf>,
) -> (RecommendedWatcher, Receiver<DebouncedEvent>) {
    let (tx, rx) = mpsc::channel();
    let mut watcher =
        notify::watcher(tx, Duration::from_secs(5)).expect("Failed to create file change watcher.");

    for (file_path, backup_dir_path) in parsed_config {
        if !file_path.is_file() {
            log::error!("Can't monitor {file_path:?}. Does not exist or is not a file.");
            continue;
        }

        if backup_dir_path.exists() && !backup_dir_path.is_dir() {
            log::error!("Can't store backups to {backup_dir_path:?}. The path already exists and is not a directory.");
            continue;
        }

        if let Err(e) = std::fs::create_dir_all(&backup_dir_path) {
            log::error!("Failed to create backup location {backup_dir_path:?}. {e:?}");
        }

        log::debug!("Starting watch on file {file_path:?}. Saving backups to {backup_dir_path:?}.");
        watcher
            .watch(file_path, RecursiveMode::NonRecursive)
            .expect("Failed to add file path to watcher.");
    }

    (watcher, rx)
}

fn process_write_event(
    changed_path: &Path,
    output_dir_lookup: &HashMap<PathBuf, PathBuf>,
) -> Result<()> {
    let canonical_path = changed_path.canonicalize()?;

    let backup_dir = output_dir_lookup
        .get(&canonical_path)
        .ok_or_else(|| anyhow!("Don't have a backup rule for file at {canonical_path:?}."))?;

    let curr_time = chrono::Utc::now();

    let backup_file_name = format!(
        "{}-{}",
        curr_time.format("%Y%m%d-%H%M%S-%6f"),
        canonical_path
            .file_name()
            .ok_or_else(|| anyhow!(
                "The path the write event happened at is not a file: {canonical_path:?}"
            ))?
            .to_str()
            .ok_or_else(|| anyhow!("Failed to format final backup file name."))?
    );

    log::debug!("Backing up {canonical_path:?} to {backup_dir:?}/{backup_file_name}.");
    std::fs::copy(
        canonical_path,
        backup_dir.join(Path::new(&backup_file_name)),
    )?;

    Ok(())
}

/// Simple file backup tool.
#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct CliArgs {
    /// The YAML configuration file.
    config_file: String,
}

fn main() {
    SimpleLogger::new().init().unwrap();

    let args = CliArgs::parse();

    let config_file = File::open(args.config_file)
        .unwrap_or_else(|e| panic!("The specified configuration file can't be opened. {e:?}"));

    let output_dir_lookup = parse_config_file(&config_file);

    let (_watcher, event_receiver) = start_file_watcher(&output_dir_lookup);

    loop {
        match event_receiver.recv().unwrap() {
            DebouncedEvent::NoticeWrite(path_buf) => {
                log::debug!("NoticeWrite event: Something is happening with {path_buf:?}.");
            }
            DebouncedEvent::NoticeRemove(path_buf) => {
                log::debug!("NoticeRemove event: {path_buf:?} is being removed.");
            }
            DebouncedEvent::Create(path_buf) => {
                log::debug!("Create event: {path_buf:?} was just created.");
            }
            DebouncedEvent::Write(path_buf) => {
                log::debug!("Write event: {path_buf:?} was just written to.");

                if let Err(e) = process_write_event(&path_buf, &output_dir_lookup) {
                    log::error!("Error while processing write event in file: {path_buf:?}: {e:?}");
                }
            }
            DebouncedEvent::Chmod(path_buf) => {
                log::debug!("Chmod event: The attributes of {path_buf:?} just changed.");
            }
            DebouncedEvent::Remove(path_buf) => {
                log::debug!("Remove event: {path_buf:?} removed.");
            }
            DebouncedEvent::Rename(old_path_buf, new_path_buf) => {
                log::debug!(
                    "Rename event: Old path_buf {old_path_buf:?} renamed to {new_path_buf:?}."
                );
            }
            DebouncedEvent::Rescan => {
                log::debug!("Rescan event.");
            }
            DebouncedEvent::Error(err, path_buf_opt) => {
                log::error!(
                    "Error event: Encountered error {err} while watching {path_buf_opt:?}."
                );
            }
        }
    }
}
