use notify::{DebouncedEvent, RecursiveMode, Watcher};
use serde::Deserialize;
use simple_logger::SimpleLogger;

use std::sync::mpsc;
use std::time::Duration;

#[derive(Debug, Deserialize)]
struct ConfigFile {
    file_paths: Vec<String>,
}

fn main() {
    SimpleLogger::new().init().unwrap();

    let config_file = std::fs::File::open("./config.yaml").unwrap();
    let config: ConfigFile = serde_yaml::from_reader(config_file).unwrap();

    let (tx, rx) = mpsc::channel();
    let mut watcher = notify::watcher(tx, Duration::from_secs(5)).unwrap();

    for watch_file_path_str in config.file_paths {
        let path = std::path::Path::new(&watch_file_path_str);

        if path.is_file() {
            log::debug!("Starting watch on file {}", path.display());
            watcher
                .watch(&watch_file_path_str, RecursiveMode::NonRecursive)
                .unwrap();
        } else {
            log::error!("{} does not exist or is not a file.", path.display());
        }
    }

    loop {
        match rx.recv().unwrap() {
            DebouncedEvent::NoticeWrite(path) => {
                log::debug!("NoticeWrite event: Something is happening with {path:?}.");
            }
            DebouncedEvent::NoticeRemove(path) => {
                log::debug!("NoticeRemove event: {path:?} is being removed.");
            }
            DebouncedEvent::Create(path) => {
                log::debug!("Create event: {path:?} was just created.");
            }
            DebouncedEvent::Write(path) => {
                log::debug!("Write event: {path:?} was just written to.")
            }
            DebouncedEvent::Chmod(path) => {
                log::debug!("Chmod event: The attributes of {path:?} just changed.")
            }
            DebouncedEvent::Remove(path) => {
                log::debug!("Remove event: {path:?} removed.");
            }
            DebouncedEvent::Rename(old_path, new_path) => {
                log::debug!("Rename event: Old path {old_path:?} renamed to {new_path:?}.");
            }
            DebouncedEvent::Rescan => {
                log::debug!("Rescan event.")
            }
            DebouncedEvent::Error(err, path_opt) => {
                log::error!("Error event: Encountered error {err} while watching {path_opt:?}.");
            }
        }
    }
}
