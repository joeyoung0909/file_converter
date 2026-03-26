use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

const NORMAL_TIMEOUT_SECS: u64 = 60;
const LARGE_FILE_TIMEOUT_SECS: u64 = 300;
const LARGE_FILE_SIZE: u64 = 100 * 1024 * 1024;
const MIN_DISK_MULTIPLIER: u64 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConvertTask {
    pub student_name: String,
    pub source_path: String,
    pub output_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConvertResult {
    pub student_name: String,
    pub source_path: String,
    pub output_path: Option<String>,
    pub success: bool,
    pub error: Option<String>,
    pub skipped: bool,
    pub skip_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversionProgress {
    pub current: usize,
    pub total: usize,
    pub current_file: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversionReport {
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub skipped: usize,
    pub results: Vec<ConvertResult>,
}

/// Find the `soffice` binary and verify it actually runs.
/// `bundled_dir` is the app resource directory; if provided, the bundled
/// LibreOffice inside it is checked first.
pub fn find_soffice(bundled_dir: Option<&std::path::Path>) -> Option<String> {
    // Bundled LibreOffice (highest priority — works on all platforms after packaging)
    if let Some(dir) = bundled_dir {
        let bundled = if cfg!(target_os = "macos") {
            dir.join("LibreOffice.app/Contents/MacOS/soffice")
        } else if cfg!(target_os = "windows") {
            dir.join("LibreOffice/program/soffice.exe")
        } else {
            dir.join("LibreOffice/program/soffice")
        };
        if soffice_works(bundled.to_string_lossy().as_ref()) {
            return Some(bundled.to_string_lossy().to_string());
        }
    }

    // System-installed LibreOffice fallback
    let candidates = if cfg!(target_os = "windows") {
        vec![
            r"C:\Program Files\LibreOffice\program\soffice.exe".to_string(),
            r"C:\Program Files (x86)\LibreOffice\program\soffice.exe".to_string(),
        ]
    } else if cfg!(target_os = "macos") {
        vec![
            "/Applications/LibreOffice.app/Contents/MacOS/soffice".to_string(),
            "/usr/local/bin/soffice".to_string(),
            "/opt/homebrew/opt/libreoffice/bin/soffice".to_string(),
            "/opt/homebrew/bin/soffice".to_string(),
        ]
    } else {
        vec![
            "/usr/bin/soffice".to_string(),
            "/usr/local/bin/soffice".to_string(),
            "/opt/libreoffice/program/soffice".to_string(),
        ]
    };

    for path in &candidates {
        if soffice_works(path) {
            return Some(path.clone());
        }
    }

    // Try $PATH
    if let Ok(output) = Command::new("which").arg("soffice").output() {
        let p = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !p.is_empty() && soffice_works(&p) {
            return Some(p);
        }
    }

    None
}

/// Run `soffice --version` to verify the binary actually works (not just a broken wrapper).
fn soffice_works(path: &str) -> bool {
    if !Path::new(path).exists() {
        return false;
    }
    Command::new(path)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Check available disk space at the given path (bytes).
fn available_disk_space(path: &Path) -> u64 {
    // Use df as a cross-platform approach
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if let Ok(meta) = std::fs::metadata(path) {
            let _ = meta.dev();
        }
        // Use statvfs equivalent via std
        if let Ok(output) = Command::new("df")
            .arg("-k")
            .arg(path.to_string_lossy().as_ref())
            .output()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            // df -k output: header + data line, 4th column is available KB
            for line in text.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    if let Ok(avail_kb) = parts[3].parse::<u64>() {
                        return avail_kb * 1024;
                    }
                }
            }
        }
    }
    u64::MAX // If we can't determine, assume enough
}

/// Convert a single Word file to PDF using LibreOffice.
fn convert_one(
    soffice: &str,
    task: &ConvertTask,
    cancel: &Arc<AtomicBool>,
) -> ConvertResult {
    if cancel.load(Ordering::Relaxed) {
        return ConvertResult {
            student_name: task.student_name.clone(),
            source_path: task.source_path.clone(),
            output_path: None,
            success: false,
            error: None,
            skipped: true,
            skip_reason: Some("已取消".to_string()),
        };
    }

    let source = Path::new(&task.source_path);
    let output_dir = Path::new(&task.output_dir);

    // Ensure output directory exists
    if let Err(e) = std::fs::create_dir_all(output_dir) {
        return ConvertResult {
            student_name: task.student_name.clone(),
            source_path: task.source_path.clone(),
            output_path: None,
            success: false,
            error: Some(format!("无法创建输出目录：{e}")),
            skipped: false,
            skip_reason: None,
        };
    }

    // Pre-check disk space for large files
    let file_size = std::fs::metadata(source).map(|m| m.len()).unwrap_or(0);
    if file_size >= LARGE_FILE_SIZE {
        let available = available_disk_space(output_dir);
        if available < file_size * MIN_DISK_MULTIPLIER {
            return ConvertResult {
                student_name: task.student_name.clone(),
                source_path: task.source_path.clone(),
                output_path: None,
                success: false,
                error: Some(format!(
                    "磁盘空间不足（需要 {}MB，可用 {}MB）",
                    (file_size * MIN_DISK_MULTIPLIER) / 1024 / 1024,
                    available / 1024 / 1024
                )),
                skipped: false,
                skip_reason: None,
            };
        }
    }

    let timeout = if file_size >= LARGE_FILE_SIZE {
        Duration::from_secs(LARGE_FILE_TIMEOUT_SECS)
    } else {
        Duration::from_secs(NORMAL_TIMEOUT_SECS)
    };

    let start = Instant::now();

    // Run soffice conversion
    let mut child = match Command::new(soffice)
        .args([
            "--headless",
            "--convert-to",
            "pdf",
            &task.source_path,
            "--outdir",
            &task.output_dir,
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            return ConvertResult {
                student_name: task.student_name.clone(),
                source_path: task.source_path.clone(),
                output_path: None,
                success: false,
                error: Some(format!("启动 LibreOffice 失败：{e}")),
                skipped: false,
                skip_reason: None,
            };
        }
    };

    // Poll for completion with timeout check
    loop {
        if cancel.load(Ordering::Relaxed) {
            let _ = child.kill();
            return ConvertResult {
                student_name: task.student_name.clone(),
                source_path: task.source_path.clone(),
                output_path: None,
                success: false,
                error: None,
                skipped: true,
                skip_reason: Some("已取消".to_string()),
            };
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                if status.success() {
                    // Determine output PDF path
                    let stem = source.file_stem().unwrap_or_default();
                    let pdf_name = format!("{}.pdf", stem.to_string_lossy());
                    let pdf_path = output_dir.join(&pdf_name);

                    // Check PDF was actually created and is non-empty
                    let pdf_size = std::fs::metadata(&pdf_path).map(|m| m.len()).unwrap_or(0);
                    if pdf_size == 0 {
                        return ConvertResult {
                            student_name: task.student_name.clone(),
                            source_path: task.source_path.clone(),
                            output_path: Some(pdf_path.to_string_lossy().to_string()),
                            success: false,
                            error: Some("转换异常，生成的 PDF 为空".to_string()),
                            skipped: false,
                            skip_reason: None,
                        };
                    }

                    return ConvertResult {
                        student_name: task.student_name.clone(),
                        source_path: task.source_path.clone(),
                        output_path: Some(pdf_path.to_string_lossy().to_string()),
                        success: true,
                        error: None,
                        skipped: false,
                        skip_reason: None,
                    };
                } else {
                    let stderr = child.stderr.take().map(|mut s| {
                        let mut buf = String::new();
                        use std::io::Read;
                        let _ = s.read_to_string(&mut buf);
                        buf
                    });
                    return ConvertResult {
                        student_name: task.student_name.clone(),
                        source_path: task.source_path.clone(),
                        output_path: None,
                        success: false,
                        error: Some(stderr.unwrap_or_else(|| "转换失败".to_string())),
                        skipped: false,
                        skip_reason: None,
                    };
                }
            }
            Ok(None) => {
                // Still running
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    return ConvertResult {
                        student_name: task.student_name.clone(),
                        source_path: task.source_path.clone(),
                        output_path: None,
                        success: false,
                        error: Some("转换超时".to_string()),
                        skipped: false,
                        skip_reason: None,
                    };
                }
                std::thread::sleep(Duration::from_millis(200));
            }
            Err(e) => {
                return ConvertResult {
                    student_name: task.student_name.clone(),
                    source_path: task.source_path.clone(),
                    output_path: None,
                    success: false,
                    error: Some(format!("进程错误：{e}")),
                    skipped: false,
                    skip_reason: None,
                };
            }
        }
    }
}

/// Compute default concurrency: min(cores/2, 4), or 1 if low RAM.
pub fn default_concurrency() -> usize {
    // Check available memory via system command (cross-platform fallback)
    let low_ram = check_low_ram();
    if low_ram {
        return 1;
    }

    let cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(2);

    ((cores / 2).max(1)).min(4)
}

/// Returns true if available RAM is less than 1 GB.
fn check_low_ram() -> bool {
    #[cfg(target_os = "macos")]
    {
        // vm_stat gives page counts; page size is 4096 bytes on macOS
        if let Ok(output) = std::process::Command::new("vm_stat").output() {
            let text = String::from_utf8_lossy(&output.stdout);
            let mut free_pages: u64 = 0;
            for line in text.lines() {
                if line.starts_with("Pages free:") || line.starts_with("Pages speculative:") {
                    let num: u64 = line
                        .split_whitespace()
                        .last()
                        .and_then(|s| s.trim_end_matches('.').parse().ok())
                        .unwrap_or(0);
                    free_pages += num;
                }
            }
            let free_bytes = free_pages * 4096;
            return free_bytes < 1024 * 1024 * 1024;
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(contents) = std::fs::read_to_string("/proc/meminfo") {
            for line in contents.lines() {
                if line.starts_with("MemAvailable:") {
                    let kb: u64 = line
                        .split_whitespace()
                        .nth(1)
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(u64::MAX);
                    return kb < 1024 * 1024; // less than 1 GB in KB
                }
            }
        }
    }
    false // Assume enough RAM if we can't determine
}

/// Run the batch conversion.
/// Normal files are processed in a thread pool; large files go through a sequential queue.
pub fn run_batch(
    app: AppHandle,
    soffice: String,
    tasks: Vec<ConvertTask>,
    concurrency: usize,
    cancel: Arc<AtomicBool>,
) -> ConversionReport {
    let total = tasks.len();
    let (normal_tasks, large_tasks): (Vec<_>, Vec<_>) = tasks.into_iter().partition(|t| {
        let size = std::fs::metadata(&t.source_path)
            .map(|m| m.len())
            .unwrap_or(0);
        size < LARGE_FILE_SIZE
    });

    let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let results = Arc::new(std::sync::Mutex::new(Vec::<ConvertResult>::new()));

    let emit_progress = |current: usize, file: &str, status: &str| {
        let _ = app.emit(
            "conversion-progress",
            ConversionProgress {
                current,
                total,
                current_file: file.to_string(),
                status: status.to_string(),
            },
        );
    };

    // Configure rayon thread pool for normal files
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(concurrency)
        .build()
        .unwrap_or_else(|_| rayon::ThreadPoolBuilder::new().build().unwrap());

    let normal_results: Vec<ConvertResult> = pool.install(|| {
        normal_tasks
            .par_iter()
            .map(|task| {
                let result = convert_one(&soffice, task, &cancel);
                let idx = counter.fetch_add(1, Ordering::Relaxed) + 1;
                let status = if result.success {
                    "success"
                } else if result.skipped {
                    "skipped"
                } else {
                    "error"
                };
                emit_progress(idx, &task.source_path, status);
                result
            })
            .collect()
    });

    for r in normal_results {
        results.lock().unwrap().push(r);
    }

    // Process large files sequentially
    for task in &large_tasks {
        if cancel.load(Ordering::Relaxed) {
            results.lock().unwrap().push(ConvertResult {
                student_name: task.student_name.clone(),
                source_path: task.source_path.clone(),
                output_path: None,
                success: false,
                error: None,
                skipped: true,
                skip_reason: Some("已取消".to_string()),
            });
            continue;
        }
        let result = convert_one(&soffice, task, &cancel);
        let idx = counter.fetch_add(1, Ordering::Relaxed) + 1;
        let status = if result.success {
            "success"
        } else if result.skipped {
            "skipped"
        } else {
            "error"
        };
        emit_progress(idx, &task.source_path, status);
        results.lock().unwrap().push(result);
    }

    let all_results = results.lock().unwrap().clone();
    let succeeded = all_results.iter().filter(|r| r.success).count();
    let failed = all_results.iter().filter(|r| !r.success && !r.skipped).count();
    let skipped = all_results.iter().filter(|r| r.skipped).count();

    ConversionReport {
        total,
        succeeded,
        failed,
        skipped,
        results: all_results,
    }
}

/// Build output directory for a task based on the output config.
pub fn resolve_output_dir(
    source_path: &str,
    student_name: &str,
    output_config: &OutputConfig,
) -> PathBuf {
    match output_config.mode.as_str() {
        "custom" => {
            if let Some(ref base) = output_config.custom_dir {
                Path::new(base).join(student_name)
            } else {
                Path::new(source_path).parent().unwrap_or(Path::new(".")).to_path_buf()
            }
        }
        _ => Path::new(source_path)
            .parent()
            .unwrap_or(Path::new("."))
            .to_path_buf(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputConfig {
    /// "same" = same directory as source, "custom" = custom_dir
    pub mode: String,
    pub custom_dir: Option<String>,
    /// "overwrite" | "skip" | "rename"
    pub existing_pdf_strategy: String,
}
