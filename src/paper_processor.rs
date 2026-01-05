use crate::ask_llm;
use crate::config::Config;
use crate::model::model::{Question, QuestionPage, SearchResult};
use crate::search_bank::search_from_bank;
use anyhow::{Context, Result};
use chromiumoxide::Page;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use tracing::{debug, error, info, warn};

/// 题目处理结果统计
#[derive(Debug, Default)]
pub struct QuestionStats {
    pub processed: usize,
    pub skipped: usize,
}

/// 处理单个试卷的所有题目
pub async fn process_single_paper(
    page: &Page,
    page_data: QuestionPage,
    paper_index: usize,
    config: &Config,
) -> Result<bool> {
    let page_id = page_data
        .page_id
        .as_ref()
        .context("试卷ID不能为空")?;

    log_paper_start(paper_index, &page_data.name, page_id, page_data.stemlist.len());

    let mut stats = QuestionStats::default();

    let mut question_index = 0;

    for (_idx, question) in page_data.stemlist.iter().enumerate() {
        question_index += 1; // 先递增索引（从1开始）
        log_question_start(paper_index, question_index, page_data.stemlist.len());

        if question.is_title {
            submit_title(page, page_id, question, question_index, paper_index).await?;
            continue;
        }

        match process_single_question(
            page,
            question,
            page_id,
            paper_index,
            config,
            &page_data.subject,
            question_index
        )
        .await
        {
            Ok(ProcessResult::Success) => {
                stats.processed += 1;
            }
            Ok(ProcessResult::Skipped) => {
                stats.skipped += 1;
            }
            Err(e) => {
                error!("[试卷 {}] 处理题目失败: {}", paper_index, e);
                stats.skipped += 1;
            }
        }
        // 注意：question_index 已经在循环开始时递增，这里不需要再次递增
    }

    // 提交整个试卷
    submit_paper(page, page_id, paper_index).await?;

    // 清理文件
    cleanup_file(page_data.file_path.as_deref(), paper_index)?;

    log_paper_complete(paper_index, &stats, page_data.stemlist.len());

    Ok(true)
}

/// 处理单个题目
async fn process_single_question(
    page: &Page,
    question: &Question,
    page_id: &str,
    paper_index: usize,
    config: &Config,
    subject: &str,
    question_index: usize
) -> Result<ProcessResult> {
    let stem = &question.stem;
    log_stem(paper_index, stem);

    // 搜索题库
    info!("[试卷 {}] 🔍 正在题库中搜索...", paper_index);
    let (search_results, full_search_result) = search_from_bank(page, stem, 50,subject).await?;
    info!(
        "[试卷 {}] ✓ 搜索完成，找到 {} 个相似题目",
        paper_index,
        search_results.len()
    );

    if search_results.is_empty() {
        warn!("[试卷 {}] ⚠️ 未找到相似题目，跳过此题", paper_index);
        return Ok(ProcessResult::Skipped);
    }

    if config.verbose_logging {
        log_search_results(paper_index, &search_results);
    }

    // 选择最佳匹配
    let selected_index = select_best_match(
        &search_results,
        stem,
        question.imgs.as_deref(),
        paper_index,
    )
    .await?;

    // 构建并提交题目
    let question_data = build_question_data(
        &full_search_result[selected_index],
        page_id,
        question_index
    );

    let success = submit_question(page, &question_data, paper_index).await?;
    if success {
        Ok(ProcessResult::Success)
    } else {
        Ok(ProcessResult::Skipped)
    }
}

/// 处理结果枚举
#[derive(Debug)]
enum ProcessResult {
    Success,
    Skipped,
}

/// 选择最佳匹配（快速匹配或LLM）
async fn select_best_match(
    search_results: &[SearchResult],
    stem: &str,
    imgs: Option<&[String]>,
    paper_index: usize,
) -> Result<usize> {
    // 尝试快速匹配
    if let Some(index) = try_quick_match(search_results, paper_index) {
        return Ok(index);
    }

    // 使用LLM判断
    info!("[试卷 {}] 🤖 正在使用LLM判断最佳匹配...", paper_index);
    let index = ask_llm::ask_llm_for_which_index(search_results, stem, imgs)
        .await
        .map_err(|e| {
            error!("[试卷 {}] ❌ LLM判断失败: {}", paper_index, e);
            e
        })?;

    info!(
        "[试卷 {}] ✓ LLM选择了第 {} 个结果 (相似度: {:?})",
        paper_index,
        index + 1,
        search_results[index].xkw_question_similarity
    );

    Ok(index)
}

/// 尝试快速匹配
fn try_quick_match(search_results: &[SearchResult], paper_index: usize) -> Option<usize> {
    if search_results.len() < 2 {
        return None;
    }

    if let (Some(s1), Some(s2)) = (
        search_results[0].xkw_question_similarity,
        search_results[1].xkw_question_similarity,
    ) {
        // 自动判断是0-1还是0-100
        let is_scale_100 = s1 > 1.0 || s2 > 1.0;
        let threshold = if is_scale_100 { 90.0 } else { 0.85 };
        let diff_threshold = if is_scale_100 { 5.0 } else { 0.05 };

        // 如果前两个相似度都大于阈值，并且相差大于阈值
        if s1 > threshold && (s1 - s2) > diff_threshold {
            info!(
                "[试卷 {}] ⚡ 满足快速匹配条件 (第一个相似度 > {} 且 差值 > {})，跳过LLM，直接选择第 1 个结果",
                paper_index, threshold, diff_threshold
            );
            return Some(0);
        }
    }

    None
}

/// 构建题目数据
fn build_question_data(
    search_result: &Value,
    page_id: &str,
    question_index: usize
) -> Value {
    let mut data = search_result.clone();
    data["addFlag"] = json!(1);
    data["paperId"] = json!(page_id);
    data["sysCode"] = json!(1);
    data["questionType"] = json!("1");
    data["relationType"] = json!(1);
    data["inputType"] = json!(1);
    data["questionIndex"] = json!(question_index);
    data
}

/// 提交标题
async fn submit_title(
    page: &Page,
    page_id: &str,
    question: &Question,
    question_index: usize,
    paper_index: usize,
) -> Result<()> {
    info!("[试卷 {}] 检测到标题，开始传入标题", paper_index);

    let title_data = json!({
        "paperId": page_id,
        "inputType": 1,
        "questionIndex": question_index,
        "questionType": "2",
        "addFlag": 1,
        "sysCode": 1,
        "relationType": 0,
        "questionSource": 3,
        "structureType": "biaoti",
        "questionInfo": {
            "stem": format!("<span>{}</span>", question.stem)
        }
    });

    let title_json = serde_json::to_string(&title_data)?;
    debug!("Playload: {}",&title_json);

    let script = build_submit_script("question/new/save", &title_json);

    let result: Value = page
        .evaluate(script.as_str())
        .await?
        .into_value()?;

    debug!("result:{}",result);

    Ok(())
}

/// 提交题目
async fn submit_question(
    page: &Page,
    question_data: &Value,
    paper_index: usize,
) -> Result<bool> {
    info!("[试卷 {}] 📤 正在提交题目到题库...", paper_index);
    let question_json = serde_json::to_string(question_data)?;
    let script = build_submit_script("question/new/save", &question_json);

    let result: Value = page
        .evaluate(script.as_str())
        .await?
        .into_value()?;

    if !result.is_null() {
        info!("[试卷 {}] ✓ 题目提交成功", paper_index);
        Ok(true)
    } else {
        warn!("[试卷 {}] ⚠️ 题目提交可能失败", paper_index);
        Ok(false)
    }
}

/// 提交整个试卷
async fn submit_paper(page: &Page, page_id: &str, paper_index: usize) -> Result<bool> {
    info!("\n[试卷 {}] {}", paper_index, "=".repeat(30));
    info!("[试卷 {}] 📋 提交整个试卷...", paper_index);

    let submit_data = json!({
        "paperId": page_id,
        "type": "NEW_INPUT"
    });

    let submit_json = serde_json::to_string(&submit_data)?;
    let script = build_submit_script("paper/process/submit", &submit_json);

    let result: Value = page
        .evaluate(script.as_str())
        .await?
        .into_value()?;

    if !result.is_null() {
        info!("[试卷 {}] ✓ 试卷提交成功", paper_index);
        Ok(true)
    } else {
        warn!("[试卷 {}] ⚠️ 试卷提交可能失败", paper_index);
        Ok(false)
    }
}

/// 构建提交脚本
fn build_submit_script(endpoint: &str, json_data: &str) -> String {
    format!(
        r#"
        (async () => {{
            try {{
                const res = await fetch("https://tps-tiku-api.staff.xdf.cn/{}", {{
                    method: "POST",
                    headers: {{
                        "Content-Type": "application/json",
                        "Accept": "application/json, text/plain, */*",
                        "tikutoken": "732FD8402F95087CD934374135C46EE5"
                    }},
                    credentials: "include",
                    body: JSON.stringify({})
                }});
                const data = await res.json();
                return data;
            }} catch (err) {{
                console.error("提交失败:", err);
                return null;
            }}
        }})()
        "#,
        endpoint, json_data
    )
}

/// 清理已处理的文件
fn cleanup_file(file_path: Option<&str>, paper_index: usize) -> Result<()> {
    info!("[试卷 {}] 🗑️ 清理已处理的文件...", paper_index);

    if let Some(file_path) = file_path {
        if Path::new(file_path).exists() {
            fs::remove_file(file_path)
                .with_context(|| format!("无法删除文件: {}", file_path))?;
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

fn log_search_results(paper_index: usize, search_results: &[SearchResult]) {
    for (i, sr) in search_results.iter().take(2).enumerate() {
        info!(
            "[试卷 {}]   {}. 相似度: {:?}",
            paper_index,
            i + 1,
            sr.xkw_question_similarity
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
