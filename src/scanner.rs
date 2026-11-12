use std::{io::ErrorKind, path::Path};
use crate::snapshots::{Snapshot, Snapshots};

pub struct Scanner;

impl Scanner {
    pub fn new() -> Self {
        Self
    }

    pub fn scan<T: AsRef<Path>>(&self, root: T, current_depth: usize, max_depth: usize) -> Result<Snapshots, std::io::Error> {
        if current_depth > max_depth {
            return Ok(Snapshots::new(vec![]));
        }
        
        let mut snapshots = Vec::new();
        let mut unreadable_dirs = 0;

        let root = root.as_ref();
        let entries = match std::fs::read_dir(root) {
            Ok(entries) => entries,
            Err(e) if e.kind() == ErrorKind::PermissionDenied => {
                let mut result = Snapshots::new(vec![]);
                result.unreadable_dirs = 1;
                return Ok(result);
            }
            Err(e) => return Err(e),
        };

        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };

            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(_) => continue,
            };

            if file_type.is_symlink() {
                continue;
            }

            let metadata = match entry.metadata() {
                Ok(metadata) => metadata,
                Err(e) => {
                    if file_type.is_dir() && e.kind() == ErrorKind::PermissionDenied {
                        unreadable_dirs += 1;
                    }
                    continue;
                }
            };

            let path = entry.path();
            let parent_path = path.parent();

            let mut size = metadata.len();

            let children = if metadata.is_dir() {
                match self.scan(&path, current_depth + 1, max_depth) {
                    Ok(children) => {
                        unreadable_dirs += children.unreadable_dirs;
                        Some(children)
                    }
                    Err(e) if e.kind() == ErrorKind::PermissionDenied => {
                        unreadable_dirs += 1;
                        None
                    }
                    Err(e) => return Err(e),
                }
            } else {
                None
            };

            // accumulate size for each folders
            if metadata.is_dir() {
                if let Some(children) = &children {
                    for ss in children {
                        size += ss.size;
                    }
                }
            }
            
            if let Some(parent_path) = parent_path {
                let snapshot = Snapshot::new(
                    &path, size, metadata.is_dir(), Some(parent_path.to_owned()), children);
                snapshots.push(snapshot);
            } else {
                let snapshot = Snapshot::new(
                    &path, size, metadata.is_dir(), None, children);
                snapshots.push(snapshot);
            }
        }

        let mut result = Snapshots::new(snapshots);
        result.unreadable_dirs = unreadable_dirs;
        Ok(result)
    }

    pub fn scan_from_home(&self, current_depth: usize, max_depth: usize) -> Result<Snapshots, std::io::Error> {
        let home = home::home_dir().expect("You have no home directory.");
        self.scan(home, current_depth, max_depth)
    }
}