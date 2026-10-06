//! Timed backups of a project with unsaved changes.

use std::io;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use super::Session;

/// Time between two backups of a project that has unsaved changes.
pub const AUTOSAVE_INTERVAL: Duration = Duration::from_secs(5 * 60);

/// The local time as it goes into a backup's file name, such as
/// `2026-10-06 18-13-05`. Every value has the same length and they sort in
/// time order, which is what pruning old backups relies on.
pub fn backup_timestamp() -> String {
    chrono::Local::now().format("%Y-%m-%d %H-%M-%S").to_string()
}

impl Session {
    /// Starts the thread that backs the project up every `interval` while
    /// it has a file and unsaved changes. A backup that fails is logged and
    /// tried again next time. The thread stops when the last handle to the
    /// session is dropped.
    pub fn spawn_autosave(&self, interval: Duration) -> io::Result<JoinHandle<()>> {
        let session = self.downgrade();
        thread::Builder::new()
            .name("windfall-autosave".to_owned())
            .spawn(move || {
                loop {
                    thread::sleep(interval);
                    let Some(session) = session.upgrade() else {
                        break;
                    };
                    match session.write_backup(&backup_timestamp()) {
                        Ok(Some(backup)) => log::info!("backed up to {}", backup.display()),
                        Ok(None) => {}
                        Err(error) => log::warn!("the backup failed: {error}"),
                    }
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_timestamp_can_be_part_of_a_file_name_and_sorts_by_time() {
        let stamp = backup_timestamp();
        assert_eq!(stamp.len(), "2026-10-06 18-13-05".len());
        let shape: String = stamp
            .chars()
            .map(|c| if c.is_ascii_digit() { '0' } else { c })
            .collect();
        assert_eq!(shape, "0000-00-00 00-00-00");
    }
}
