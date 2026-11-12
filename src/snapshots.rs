// take this and compare it to the new and current thing
// let user set snapshot

use std::{collections::HashMap, fs::File, io::{BufWriter, Error, ErrorKind}, path::{Path, PathBuf}, vec};

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

/// returns (is_neg, str)
pub fn ibytes_to_units(bytes: i64) -> (bool, String) {
    let is_neg = bytes.is_negative();
    let bytes = bytes.abs();
    let mut final_bytes = bytes;
    let mut div_count = 0;
    let units = ["B", "KB", "MB", "GB", "TB"];

    while final_bytes >= 1024 {
        final_bytes /= 1024;
        div_count+=1;
    }

    (is_neg, format!("{}{}", final_bytes, units[div_count]))
}

/// size in bytes
/// theoretically, they should all have the same parent
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
    pub path: Option<PathBuf>,
    pub size: u64,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub unreadable_dirs: usize,
}

impl Snapshots {
    pub fn new(snapshots: Vec<Snapshot>) -> Self {
        let size: u64 = snapshots.iter().map(|ss| ss.size).sum();
        let path: Option<PathBuf> = {
            if snapshots.is_empty() {
                None
            } else {
                snapshots[0].parent_path.clone()
            }
        };
        Self {
            snapshots,
            path,
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

        serde_json::to_writer_pretty(writer, &self)?;
        Ok(())
    }

    /// file_path, as in File Path, dont mistake it for input directory.
    pub fn load<T: AsRef<Path>>(&self, file_path: T) -> Result<Self, std::io::Error> {
        let file_path = file_path.as_ref();
        if file_path.is_dir() { 
            return Err(
                Error::new(ErrorKind::Unsupported, "File couldn't be loaded cuz it was a direcotry")
            ); 
        }
        let file = File::open(file_path)?;
        let snapshots: Snapshots = serde_json::from_reader(file)
            .map_err(|e| Error::new(ErrorKind::InvalidData, e))?;

        Ok(snapshots)
    }

    /// finds the difference between the stored and current snapshot
    /// old_snapshot_file can be found in ~/.ddisk/
    pub fn compare_growth(&self, old_snapshots: &Snapshots) -> Result<HashMap<String, i64>, std::io::Error> {
        let mut differences: HashMap<String, i64> = HashMap::new();
        for (ss, old_ss)
            in self.snapshots.iter().zip(old_snapshots) {
            // wont compare if it isnt the same snapshot
            // WARNING: subject to change, names arent reliable for file identity 
            if ss.name != old_ss.name { continue }
            let diff = (ss.size as i128) - (old_ss.size as i128);
            let diff = diff as i64;
            differences.insert(ss.name.clone(), diff);
        }

        Ok(differences)
    }

    pub fn compare_growth_to_rows(&self, old_snapshots: &Snapshots) -> Result<Vec<Row<'_>>, std::io::Error> {
        let diffs = self.compare_growth(old_snapshots)?;
        let mut rows = Vec::new();
        for (path, diff) in diffs {
            let row = Row::new(vec![
                Cell::from(path),
                Cell::from(diff.to_string()),
            ]);
            rows.push(row);
        }
        
        Ok(rows)
    }

    pub fn compare_growth_to_rows_unit(&self, old_snapshots: &Snapshots) -> Result<Vec<Row<'_>>, std::io::Error> {
        let diffs = self.compare_growth(old_snapshots)?;
        let mut rows = Vec::new();
        for (path, diff) in diffs {
            let (is_diff_neg, mut diff) = ibytes_to_units(diff);
            if diff == "0B" { diff = "No changes".to_string(); }
            if is_diff_neg { diff = format!("-{}", diff); }

            let row = Row::new(vec![
                Cell::from(path),
                Cell::from(diff.to_string()),
            ]);
            rows.push(row);
        }
        
        Ok(rows)
    }

    pub fn to_rows(&self) -> Vec<Row<'_>> {
        self.snapshots
            .iter()
            .map(|ss| {
                Row::new(vec![
                    Cell::from(ss.name.clone()),
                    Cell::from(ss.size.to_string())
                ])
            })
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