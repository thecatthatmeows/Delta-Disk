use std::path::Path;
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

        let root = root.as_ref();
        for entry in std::fs::read_dir(root)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let path = entry.path();
            let parent_path = root.parent();

            let name = entry.file_name().into_string().unwrap();
            let mut size = metadata.len();

            let children = if metadata.is_dir() {
                Some(self.scan(&path, current_depth + 1, max_depth)?)
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
                    name, size, metadata.is_dir(), Some(parent_path.to_owned()), children);
                snapshots.push(snapshot);
            } else {
                let snapshot = Snapshot::new(
                    name, size, metadata.is_dir(), None, children);
                snapshots.push(snapshot);
            }
        }

        Ok(Snapshots::new(snapshots))
    }

    pub fn scan_from_home(&self, current_depth: usize, max_depth: usize) -> Result<Snapshots, std::io::Error> {
        let home = home::home_dir().expect("You have no home directory.");
        self.scan(home, current_depth, max_depth)
    }
}