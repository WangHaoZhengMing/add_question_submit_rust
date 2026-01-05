use crate::models::question::QuestionPage;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tokio::fs;

/// 从 TOML 文件加载数据并转换为 QuestionPage 对象
pub async fn load_toml_to_question_page(toml_file_path: &Path) -> Result<QuestionPage> {
    let content = fs::read_to_string(toml_file_path)
        .await
        .with_context(|| format!("无法读取TOML文件: {}", toml_file_path.display()))?;

    let mut page: QuestionPage = toml::from_str(&content)
        .with_context(|| format!("无法解析TOML文件: {}", toml_file_path.display()))?;

    // 设置文件路径
    page.file_path = Some(toml_file_path.to_string_lossy().to_string());

    Ok(page)
}

/// 从文件夹中加载所有 TOML 文件并转换为 QuestionPage 对象列表
pub async fn load_all_toml_files(folder_path: &str) -> Result<Vec<QuestionPage>> {
    let folder = PathBuf::from(folder_path);

    if !folder.exists() {
        anyhow::bail!("文件夹不存在: {}", folder_path);
    }

    let mut question_pages = Vec::new();
    let mut entries = fs::read_dir(&folder)
        .await
        .with_context(|| format!("无法读取文件夹: {}", folder_path))?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("toml") {
            tracing::info!(
                "正在加载: {}",
                path.file_name().unwrap_or_default().to_string_lossy()
            );

            match load_toml_to_question_page(&path).await {
                Ok(page) => {
                    let question_count = page.stemlist.len();
                    tracing::info!("成功加载 {} 个题目", question_count);
                    question_pages.push(page);
                }
                Err(e) => {
                    tracing::warn!("加载文件失败 {}: {}", path.display(), e);
                }
            }
        }
    }

    Ok(question_pages)
}

/// 从文件夹中加载单个 TOML 文件（按索引）
pub async fn load_single_toml(folder_path: &str, index: usize) -> Result<Option<QuestionPage>> {
    let folder = PathBuf::from(folder_path);

    if !folder.exists() {
        anyhow::bail!("文件夹不存在: {}", folder_path);
    }

    let mut toml_files = Vec::new();
    let mut entries = fs::read_dir(&folder)
        .await
        .with_context(|| format!("无法读取文件夹: {}", folder_path))?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("toml") {
            toml_files.push(path);
        }
    }

    if toml_files.is_empty() {
        tracing::warn!("在文件夹 {} 中没有找到 TOML 文件", folder_path);
        return Ok(None);
    }

    if index >= toml_files.len() {
        anyhow::bail!("索引 {} 超出范围，共有 {} 个文件", index, toml_files.len());
    }

    let toml_file = &toml_files[index];
    tracing::info!(
        "正在加载第 {} 个文件: {}",
        index + 1,
        toml_file.file_name().unwrap_or_default().to_string_lossy()
    );

    let page_data = load_toml_to_question_page(toml_file).await?;
    tracing::info!("成功加载 {} 个题目", page_data.stemlist.len());

    Ok(Some(page_data))
}
