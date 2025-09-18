// src/model.rs

use std::collections::{HashMap, BTreeMap};

/// Uniquely identifies a committer
pub type CommitterId = usize;

/// Uniquely identifies a file across renames
pub type FileId = usize;

/// Represents a single change to a line in a file
#[derive(Debug, Clone, Copy)]
pub struct LineChange {
    pub timestamp: i64,
    pub committer_id: CommitterId,
}

/// Stores the entire history of changes for a specific line
pub type LineHistory = Vec<LineChange>;

/// Maps a (FileId, line_number) pair to its change history
pub type ChangeMap = HashMap<(FileId, usize), LineHistory>;

/// Information about a file's lifecycle and properties
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub id: FileId,
    pub path: String, // The most recent path
    pub birth_time: i64,
    pub death_time: Option<i64>,
    /// Maps timestamp to line count, to know file size over time
    pub line_counts: BTreeMap<i64, usize>,
}

/// The complete results of the repository analysis
#[derive(Debug)]
pub struct AnalysisResult {
    pub files: Vec<FileInfo>,
    pub changes: ChangeMap,
    pub committers: Vec<String>,
    pub start_time: i64,
    pub end_time: i64,
    pub commits: Vec<(git2::Oid, i64)>,
}
