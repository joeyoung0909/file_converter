use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::SystemTime;
use walkdir::WalkDir;

/// Information about a single Word/WPS file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub name: String,
    pub size: u64,
    /// Last modified timestamp in seconds since Unix epoch
    pub modified: u64,
    pub is_large: bool,
}

/// A student subfolder with discovered Word/WPS files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudentFolder {
    pub name: String,
    pub path: String,
    pub files: Vec<FileInfo>,
}

const WORD_EXTENSIONS: &[&str] = &["doc", "docx", "wps"];
const LARGE_FILE_THRESHOLD: u64 = 100 * 1024 * 1024; // 100 MB

/// Scan a root directory for student subfolders and their Word/WPS files.
/// `depth` controls how many levels below the root to search (default 2).
pub fn scan_root(root: &Path, depth: u8) -> Vec<StudentFolder> {
    if !root.is_dir() {
        return vec![];
    }

    let mut folders: Vec<StudentFolder> = Vec::new();

    // Each immediate child directory is treated as a student folder
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let student_path = entry.path();
            if !student_path.is_dir() {
                continue;
            }
            let student_name = student_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            // Walk within the student folder up to `depth` levels
            let files = collect_word_files(&student_path, depth as usize);
            folders.push(StudentFolder {
                name: student_name,
                path: student_path.to_string_lossy().to_string(),
                files,
            });
        }
    }

    // Sort by student name for consistent display
    folders.sort_by(|a, b| a.name.cmp(&b.name));
    folders
}

fn collect_word_files(dir: &Path, max_depth: usize) -> Vec<FileInfo> {
    let mut files = Vec::new();

    let walker = WalkDir::new(dir)
        .max_depth(max_depth)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok());

    for entry in walker {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        if !WORD_EXTENSIONS.contains(&ext.as_str()) {
            continue;
        }

        let metadata = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        let size = metadata.len();
        let modified = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        files.push(FileInfo {
            path: path.to_string_lossy().to_string(),
            name: path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            size,
            modified,
            is_large: size >= LARGE_FILE_THRESHOLD,
        });
    }

    files
}
