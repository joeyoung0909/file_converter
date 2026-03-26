use serde::{Deserialize, Serialize};

use crate::scanner::FileInfo;

/// Identification status for a student folder
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum IdentifyStatus {
    /// Exactly one thesis found with confidence
    Identified,
    /// Multiple candidates — all shown, user should pick
    MultipleCandidates,
    /// No Word files at all
    NotFound,
    /// Word files exist but none matched keywords; largest shown as suggestion
    Unconfirmed,
}

/// Result of running the identification rule engine on a student folder
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentifyResult {
    pub status: IdentifyStatus,
    /// All candidate files (may be >1 for MultipleCandidates)
    pub candidates: Vec<FileInfo>,
    /// The file pre-selected by the engine (index into candidates)
    pub selected_index: Option<usize>,
}

/// User-configurable keywords (loaded from settings)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeywordConfig {
    pub include_keywords: Vec<String>,
    pub exclude_keywords: Vec<String>,
}

impl Default for KeywordConfig {
    fn default() -> Self {
        Self {
            include_keywords: DEFAULT_INCLUDE.iter().map(|s| s.to_string()).collect(),
            exclude_keywords: DEFAULT_EXCLUDE.iter().map(|s| s.to_string()).collect(),
        }
    }
}

const DEFAULT_INCLUDE: &[&str] = &[
    "毕业论文",
    "毕业设计",
    "学位论文",
    "毕设",
    "论文终稿",
    "论文定稿",
    "最终版",
    "thesis",
    "dissertation",
    "final_paper",
];

const DEFAULT_EXCLUDE: &[&str] = &[
    "开题报告",
    "任务书",
    "文献综述",
    "文献翻译",
    "外文翻译",
    "中期报告",
    "答辩",
    "ppt",
    "幻灯片",
    "成绩",
];

/// Final-version keywords (preferred when multiple candidates)
const FINAL_KEYWORDS: &[&str] = &["终稿", "定稿", "最终", "final"];

/// Run the rule engine on a list of files from one student folder.
pub fn identify(files: &[FileInfo], config: &KeywordConfig) -> IdentifyResult {
    if files.is_empty() {
        return IdentifyResult {
            status: IdentifyStatus::NotFound,
            candidates: vec![],
            selected_index: None,
        };
    }

    // Step 1: Apply include keyword filter (case-insensitive)
    let mut matched: Vec<&FileInfo> = files
        .iter()
        .filter(|f| {
            let name_lower = f.name.to_lowercase();
            config
                .include_keywords
                .iter()
                .any(|kw| name_lower.contains(&kw.to_lowercase()))
        })
        .collect();

    // Step 2: Remove files matching exclude keywords
    matched.retain(|f| {
        let name_lower = f.name.to_lowercase();
        !config
            .exclude_keywords
            .iter()
            .any(|kw| name_lower.contains(&kw.to_lowercase()))
    });

    // Single file in folder — directly select it
    if files.len() == 1 && matched.is_empty() {
        return IdentifyResult {
            status: IdentifyStatus::Identified,
            candidates: vec![files[0].clone()],
            selected_index: Some(0),
        };
    }

    if matched.is_empty() {
        // No keyword match — pick largest file as unconfirmed suggestion
        let mut sorted = files.to_vec();
        sorted.sort_by(|a, b| b.size.cmp(&a.size));
        return IdentifyResult {
            status: IdentifyStatus::Unconfirmed,
            candidates: sorted,
            selected_index: Some(0),
        };
    }

    if matched.len() == 1 {
        return IdentifyResult {
            status: IdentifyStatus::Identified,
            candidates: vec![matched[0].clone()],
            selected_index: Some(0),
        };
    }

    // Multiple matches — try to narrow down with final-version keywords
    let finals: Vec<&FileInfo> = matched
        .iter()
        .filter(|f| {
            let name_lower = f.name.to_lowercase();
            FINAL_KEYWORDS
                .iter()
                .any(|kw| name_lower.contains(*kw))
        })
        .cloned()
        .collect();

    if finals.len() == 1 {
        let all: Vec<FileInfo> = matched.iter().map(|f| (*f).clone()).collect();
        let selected_idx = all.iter().position(|f| f.path == finals[0].path);
        return IdentifyResult {
            status: IdentifyStatus::Identified,
            candidates: all,
            selected_index: selected_idx,
        };
    }

    // Still multiple — pick newest modification time as default selection
    let pool = if !finals.is_empty() { &finals } else { &matched };
    let newest = pool
        .iter()
        .max_by_key(|f| f.modified)
        .map(|f| f.path.clone());

    let all: Vec<FileInfo> = matched.iter().map(|f| (*f).clone()).collect();
    let selected_idx = newest.and_then(|p| all.iter().position(|f| f.path == p));

    IdentifyResult {
        status: IdentifyStatus::MultipleCandidates,
        candidates: all,
        selected_index: selected_idx,
    }
}
