use notify::{Watcher, DebouncedEvent, RecursiveMode};
use simple_logger::SimpleLogger;

use std::sync::mpsc;
use std::time::Duration;

fn main() {
    SimpleLogger::new().init().unwrap();

    let (tx, rx) = mpsc::channel();

    let mut watcher = notify::watcher(tx, Duration::from_secs(5)).unwrap();

    watcher.watch("./test.txt", RecursiveMode::NonRecursive).unwrap();

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
            },
            DebouncedEvent::Write(path) => {
                log::debug!("Write event: {path:?} was just written to.")
            },
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
            },
            DebouncedEvent::Error(err, path_opt) => {
                log::error!("Error event: Encountered error {err} while watching {path_opt:?}.");
            }
        }
    }
}
