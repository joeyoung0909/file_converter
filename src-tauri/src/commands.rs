use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::converter::{
    default_concurrency, find_soffice, resolve_output_dir, run_batch, ConvertTask,
    ConversionReport, OutputConfig,
};
use crate::identifier::{identify, IdentifyResult, KeywordConfig};
use crate::scanner::{scan_root, StudentFolder};

/// Shared application state
pub struct AppState {
    pub cancel_flag: Arc<AtomicBool>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            cancel_flag: Arc::new(AtomicBool::new(false)),
        }
    }
}

/// Entry returned to the frontend combining scan + identification results
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentEntry {
    pub student_name: String,
    pub student_path: String,
    pub identify_result: IdentifyResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConvertRequest {
    pub tasks: Vec<ConvertTask>,
    pub output_config: OutputConfig,
    pub concurrency: Option<usize>,
}

/// Scan a root folder and identify thesis files in each student subfolder.
#[tauri::command]
pub fn scan_folder(
    path: String,
    depth: Option<u8>,
    keyword_config: Option<KeywordConfig>,
) -> Result<Vec<StudentEntry>, String> {
    let root = std::path::Path::new(&path);
    if !root.exists() {
        return Err(format!("文件夹不存在：{path}"));
    }
    if !root.is_dir() {
        return Err(format!("路径不是文件夹：{path}"));
    }

    let depth = depth.unwrap_or(2);
    let config = keyword_config.unwrap_or_default();
    let folders: Vec<StudentFolder> = scan_root(root, depth);

    let entries: Vec<StudentEntry> = folders
        .into_iter()
        .map(|folder| {
            let result = identify(&folder.files, &config);
            StudentEntry {
                student_name: folder.name,
                student_path: folder.path,
                identify_result: result,
            }
        })
        .collect();

    Ok(entries)
}

/// Check if LibreOffice is available and return its path.
#[tauri::command]
pub fn get_libreoffice_path(app: AppHandle) -> Option<String> {
    let resource_dir = app.path().resource_dir().ok();
    find_soffice(resource_dir.as_deref())
}

/// Get the default concurrency for the current machine.
#[tauri::command]
pub fn get_default_concurrency() -> usize {
    default_concurrency()
}

/// Start batch conversion. Emits `conversion-progress` events during processing.
#[tauri::command]
pub async fn start_conversion(
    app: AppHandle,
    state: State<'_, AppState>,
    request: ConvertRequest,
) -> Result<ConversionReport, String> {
    let resource_dir = app.path().resource_dir().ok();
    let soffice = find_soffice(resource_dir.as_deref()).ok_or_else(|| {
        "未找到 LibreOffice，请将 LibreOffice.app 放入 src-tauri/resources/ 目录，或安装系统版 LibreOffice（https://www.libreoffice.org/download/）".to_string()
    })?;

    // Reset cancel flag
    state
        .cancel_flag
        .store(false, std::sync::atomic::Ordering::Relaxed);
    let cancel = state.cancel_flag.clone();

    let concurrency = request
        .concurrency
        .unwrap_or_else(default_concurrency)
        .max(1)
        .min(8);

    // Resolve output directories
    let tasks_with_output: Vec<ConvertTask> = request
        .tasks
        .into_iter()
        .map(|mut task| {
            let outdir = resolve_output_dir(&task.source_path, &task.student_name, &request.output_config);
            task.output_dir = outdir.to_string_lossy().to_string();
            task
        })
        .collect();

    // Run in blocking thread to avoid blocking async executor
    let report = tokio::task::spawn_blocking(move || {
        run_batch(app, soffice, tasks_with_output, concurrency, cancel)
    })
    .await
    .map_err(|e| format!("转换任务出错：{e}"))?;

    Ok(report)
}

/// Cancel any in-progress conversion.
#[tauri::command]
pub fn cancel_conversion(state: State<'_, AppState>) {
    state
        .cancel_flag
        .store(true, std::sync::atomic::Ordering::Relaxed);
}

/// Export the conversion report to a file (txt or csv).
#[tauri::command]
pub fn export_report(
    report: ConversionReport,
    format: String,
    path: String,
) -> Result<(), String> {
    let content = match format.as_str() {
        "csv" => build_csv(&report),
        _ => build_txt(&report),
    };
    std::fs::write(&path, content).map_err(|e| format!("写入文件失败：{e}"))
}

fn build_txt(report: &ConversionReport) -> String {
    let mut lines = vec![
        "===== 转换结果报告 =====".to_string(),
        format!(
            "总计：{}  成功：{}  失败：{}  跳过：{}",
            report.total, report.succeeded, report.failed, report.skipped
        ),
        String::new(),
    ];
    for r in &report.results {
        let status = if r.success {
            "✓ 成功".to_string()
        } else if r.skipped {
            format!("- 跳过（{}）", r.skip_reason.as_deref().unwrap_or(""))
        } else {
            format!("✗ 失败（{}）", r.error.as_deref().unwrap_or(""))
        };
        lines.push(format!("[{}] {} → {}", r.student_name, r.source_path, status));
    }
    lines.join("\n")
}

fn build_csv(report: &ConversionReport) -> String {
    let mut lines =
        vec!["学生文件夹,源文件,状态,输出PDF,错误信息".to_string()];
    for r in &report.results {
        let status = if r.success {
            "成功"
        } else if r.skipped {
            "跳过"
        } else {
            "失败"
        };
        let output = r.output_path.as_deref().unwrap_or("");
        let error = r.error.as_deref().unwrap_or(r.skip_reason.as_deref().unwrap_or(""));
        lines.push(format!(
            "{},{},{},{},{}",
            r.student_name, r.source_path, status, output, error
        ));
    }
    lines.join("\n")
}
