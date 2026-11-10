// take this and compare it to the new and current thing
// let user set snapshot

use std::{collections::HashMap, fs::File, io::BufWriter, path::{Path, PathBuf}, vec};

use chrono::{DateTime, Utc};
use ratatui::{style::{Color, Style}, text::Span, widgets::{Cell, Row}};
use serde::{Deserialize, Serialize};

pub fn bytes_to_units(bytes: u64) -> String {
    let mut final_bytes = bytes;
    let mut div_count = 0;
    let units = ["B", "KB", "MB", "GB", "TB"];

    while final_bytes >= 1024 {
        final_bytes /= 1024;
        div_count+=1;
    }

    format!("{}{}", final_bytes, units[div_count])
}

/// size in bytes
#[derive(Clone, Deserialize, Serialize)]
pub struct Snapshot {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
    pub is_dir: bool,
    pub parent_path: Option<PathBuf>,
    pub children: Option<Snapshots>
}

impl Snapshot {
    /// Size in bytes.
    /// NOTE: The snapshot is currently used by the Scanner in scanner.rs
    pub fn new<T: AsRef<Path>>(path: T, size: u64, is_dir: bool, parent_path: Option<PathBuf>, children: Option<Snapshots>) -> Self {
        Self {
            path: path.as_ref().to_owned(),
            name: path.as_ref().file_name().unwrap().to_str().unwrap().to_owned(),
            size,
            is_dir,
            parent_path,
            children,
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Snapshots {
    pub snapshots: Vec<Snapshot>,
    pub size: u64,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub unreadable_dirs: usize,
}

impl Snapshots {
    pub fn new(snapshots: Vec<Snapshot>) -> Self {
        let size: u64 = snapshots.iter().map(|ss| ss.size).sum();
        Self {
            snapshots,
            size,
            created_at: Utc::now(),
            unreadable_dirs: 0,
        }
    }

    /// dir_path, as in Directory Path, dont mistake it for output file.
    pub fn save<T: AsRef<Path>>(&self, dir_path: T) -> Result<(), std::io::Error> {
        let dir_path = dir_path.as_ref();
        if !std::fs::exists(&dir_path)? {
            std::fs::create_dir(&dir_path)?;
        }
        let datetime = self.created_at.to_string();
        let out_file = format!("{}{}.json", datetime, "_checkpoint");
        let file = std::fs::File::create(&dir_path.join(out_file))?;
        let writer = BufWriter::new(file);

        let mut saved = HashMap::new();
        for ss in &self.snapshots {
            saved.insert(ss.name.clone(), ss);
        }
        serde_json::to_writer_pretty(writer, &saved)?;
        Ok(())
    }

    pub fn load<T: AsRef<Path>>(&self, dir_path: T) -> Result<Self, std::io::Error> {
        let dir_path = dir_path.as_ref();
        let file = File::open(dir_path)?;
        let snapshots: Snapshots = serde_json::from_reader(file)?;
        
        Ok(snapshots)
    }

    /// old_snapshot_file can be found in ~/.ddisk/
    pub fn compare_growth<T: AsRef<Path>>(&self, old_snapshot_file: T) -> Result<HashMap<PathBuf, i64>, std::io::Error> {
        let old_snapshots = self.load(old_snapshot_file)?;

        let mut comparisons: HashMap<PathBuf, i64> = HashMap::new();
        for (ss, old_ss)
            in self.snapshots.iter().zip(old_snapshots) {
            // wont compare if it isnt the same snapshot
            // WARNING: subject to change, names arent reliable for file identity 
            if ss.name != old_ss.name {
                continue
            }
            let comparison = (ss.size as i128) - (old_ss.size as i128);
            let comparison = comparison as i64;
            comparisons.insert(ss.path.clone(), comparison);
        }

        Ok(comparisons)
    }

    pub fn to_rows(&self) -> Vec<Row<'_>> {
        self.snapshots
            .iter()
            .map(|ss|
                Row::new(vec![
                    Cell::from(ss.name.clone()),
                    Cell::from(ss.size.to_string())
                ])
            )
            .collect()
    }

    pub fn to_rows_unit(&self) -> Vec<Row<'_>> {
        self.snapshots
            .iter()
            .map(|ss| {
                let name = if ss.is_dir {
                    Span::styled(&ss.name, Style::default().fg(Color::Blue))
                } else {
                    Span::styled(&ss.name, Style::default().fg(Color::White))
                };
                let size = bytes_to_units(ss.size);

                Row::new(vec![
                    Cell::from(name),
                    Cell::from(size)
                ])
            })
            .collect()
    }
}

impl IntoIterator for Snapshots {
    type Item = Snapshot;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.snapshots.into_iter()
    }
}

impl<'a> IntoIterator for &'a Snapshots {
    type Item = &'a Snapshot;
    type IntoIter = std::slice::Iter<'a, Snapshot>;

    fn into_iter(self) -> Self::IntoIter {
        self.snapshots.iter()
    }
}