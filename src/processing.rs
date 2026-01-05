//! 核心业务处理模块
//!
//! 负责试卷和题目的处理流程

use crate::api;
use crate::config::Config;
use crate::models::question::{Question, QuestionPage};
use anyhow::{Context, Result};
use chromiumoxide::Page;
use serde_json::json;
use std::fs;
use std::path::Path;
use tracing::{error, info, warn};

/// 题目处理统计
#[derive(Debug, Default)]
pub struct QuestionStats {
    pub processed: usize,
    pub skipped: usize,
}

/// 处理单个试卷
///
/// # 参数
/// - `page`: 浏览器页面对象
/// - `paper`: 试卷数据
/// - `paper_index`: 试卷索引（用于日志）
/// - `config`: 配置
///
/// # 返回
/// 返回是否成功处理
pub async fn process_paper(
    page: &Page,
    paper: QuestionPage,
    paper_index: usize,
    config: &Config,
) -> Result<bool> {
    let paper_id = paper.page_id.as_ref().context("试卷ID不能为空")?;

    log_paper_start(paper_index, &paper.name, paper_id, paper.stemlist.len());

    let mut stats = QuestionStats::default();
    let mut question_index = 0;

    // 处理所有题目
    for question in paper.stemlist.iter() {
        question_index += 1;
        log_question_start(paper_index, question_index, paper.stemlist.len());

        // 如果是标题，单独处理
        if question.is_title {
            match api::tiku::save_title(page, paper_id, question_index, &question.stem).await {
                Ok(_) => info!("[试卷 {}] ✓ 标题保存成功", paper_index),
                Err(e) => {
                    error!("[试卷 {}] 标题保存失败: {}", paper_index, e);
                    stats.skipped += 1;
                }
            }
            continue;
        }

        // 处理普通题目
        match process_question(
            page,
            question,
            paper_id,
            &paper.subject,
            question_index,
            paper_index,
            config,
        )
        .await
        {
            Ok(true) => {
                stats.processed += 1;
            }
            Ok(false) => {
                stats.skipped += 1;
            }
            Err(e) => {
                error!(
                    "[试卷 {}] 题目 {} 处理失败: {}",
                    paper_index, question_index, e
                );
                stats.skipped += 1;
            }
        }
    }

    // 提交整个试卷
    match api::tiku::submit_paper(page, paper_id).await {
        Ok(_) => info!("[试卷 {}] ✓ 试卷提交成功", paper_index),
        Err(e) => {
            error!("[试卷 {}] 试卷提交失败: {}", paper_index, e);
        }
    }

    // 清理文件
    cleanup_file(paper.file_path.as_deref(), paper_index)?;

    // 输出统计信息
    log_paper_complete(paper_index, &stats, paper.stemlist.len());

    Ok(true)
}

/// 处理单个题目
///
/// # 参数
/// - `page`: 浏览器页面对象
/// - `question`: 题目数据
/// - `paper_id`: 试卷ID
/// - `subject`: 科目
/// - `question_index`: 题目索引
/// - `paper_index`: 试卷索引（用于日志）
/// - `config`: 配置
///
/// # 返回
/// 返回是否成功处理（true=成功，false=跳过）
async fn process_question(
    page: &Page,
    question: &Question,
    paper_id: &str,
    subject: &str,
    question_index: usize,
    paper_index: usize,
    config: &Config,
) -> Result<bool> {
    let stem = &question.stem;

    // 日志：显示题干预览
    log_stem(paper_index, stem);

    // 1. 获取科目代码
    let subject_code = crate::models::subject::Subject::from_str(subject)
        .with_context(|| format!("无法解析科目: {}", subject))?
        .code()
        .to_string();

    // 2. 搜索题库
    info!("[试卷 {}] 🔍 正在题库中搜索...", paper_index);
    let search_results = api::tiku::search_questions_xueku(page, stem, &subject_code, 50).await?;

    info!(
        "[试卷 {}] ✓ 搜索完成，找到 {} 个相似题目",
        paper_index,
        search_results.len()
    );

    if search_results.is_empty() {
        warn!("[试卷 {}] ⚠️ 未找到相似题目，跳过此题", paper_index);
        return Ok(false);
    }

    // 详细日志（如果启用）
    if config.verbose_logging {
        log_search_results(paper_index, &search_results);
    }

    // 3. 选择最佳匹配
    let selected_index = api::llm::find_best_match(
        &search_results,
        stem,
        question.imgs.as_deref(),
        &config.llm_api_key,
        &config.llm_api_base_url,
    )
    .await?;

    info!(
        "[试卷 {}] ✓ 选择了第 {} 个结果",
        paper_index,
        selected_index + 1
    );

    // 4. 构建并提交题目数据
    let question_data =
        build_question_data(&search_results[selected_index], paper_id, question_index);

    api::tiku::save_question(page, &question_data).await?;

    Ok(true)
}

/// 构建题目数据
fn build_question_data(
    search_result: &serde_json::Value,
    paper_id: &str,
    question_index: usize,
) -> serde_json::Value {
    let mut data = search_result.clone();
    data["addFlag"] = json!(1);
    data["paperId"] = json!(paper_id);
    data["sysCode"] = json!(1);
    data["questionType"] = json!("1");
    data["relationType"] = json!(1);
    data["inputType"] = json!(1);
    data["questionIndex"] = json!(question_index);
    data
}

/// 清理已处理的文件
fn cleanup_file(file_path: Option<&str>, paper_index: usize) -> Result<()> {
    info!("[试卷 {}] 🗑️ 清理已处理的文件...", paper_index);

    if let Some(file_path) = file_path {
        if Path::new(file_path).exists() {
            fs::remove_file(file_path).with_context(|| format!("无法删除文件: {}", file_path))?;
            info!(
                "[试卷 {}] ✓ 文件已删除: {}",
                paper_index,
                Path::new(file_path)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
            );
        } else {
            warn!("[试卷 {}] ⚠️ 文件不存在: {}", paper_index, file_path);
        }
    } else {
        warn!("[试卷 {}] ⚠️ 文件路径未设置", paper_index);
    }

    Ok(())
}

// ========== 日志辅助函数 ==========

fn log_paper_start(paper_index: usize, name: &str, page_id: &str, question_count: usize) {
    info!("[试卷 {}] 开始处理", paper_index);
    info!("[试卷 {}] 名称: {}", paper_index, name);
    info!("[试卷 {}] ID: {}", paper_index, page_id);
    info!("[试卷 {}] 题目总数: {}", paper_index, question_count);
}

fn log_question_start(paper_index: usize, question_index: usize, total: usize) {
    info!("\n[试卷 {}] {}", paper_index, "─".repeat(30));
    info!(
        "[试卷 {}] 处理第 {}/{} 道题目",
        paper_index, question_index, total
    );
}

fn log_stem(paper_index: usize, stem: &str) {
    let stem_preview = if stem.chars().count() > 80 {
        stem.chars().take(80).collect::<String>() + "..."
    } else {
        stem.to_string()
    };
    info!("[试卷 {}] 题干: {}", paper_index, stem_preview);
}

fn log_search_results(paper_index: usize, search_results: &[serde_json::Value]) {
    for (i, result) in search_results.iter().take(2).enumerate() {
        let similarity = result.get("xkwQuestionSimilarity").and_then(|v| v.as_f64());
        info!(
            "[试卷 {}]   {}. 相似度: {:?}",
            paper_index,
            i + 1,
            similarity
        );
    }
}

fn log_paper_complete(paper_index: usize, stats: &QuestionStats, total: usize) {
    info!(
        "[试卷 {}] 题目统计: 成功 {}, 跳过 {}, 总计 {}",
        paper_index, stats.processed, stats.skipped, total
    );
    info!("\n[试卷 {}] ✅ 试卷处理完成\n", paper_index);
}
