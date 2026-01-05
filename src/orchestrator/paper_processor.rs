//! 单个试卷处理器 - 编排层
//!
//! ## 职责
//!
//! 本模块负责处理单个试卷的所有题目，是试卷级别的编排器。
//!
//! ## 核心功能
//!
//! 1. **遍历题目**：循环处理 `Vec<Question>`
//! 2. **流程调度**：创建并复用 `QuestionFlow`
//! 3. **特殊处理**：区分标题和普通题目
//! 4. **试卷提交**：完成后提交整个试卷
//! 5. **文件清理**：删除已处理的 TOML 文件
//! 6. **统计输出**：记录成功/跳过/失败数量

use crate::config::Config;
use crate::infrastructure::JsExecutor;
use crate::models::question::{Question, QuestionPage};
use crate::workflow::{ProcessResult, QuestionCtx, QuestionFlow};
use anyhow::{Context, Result};
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
/// - `executor`: JS 执行器（持有 page）
/// - `paper`: 试卷数据
/// - `paper_index`: 试卷索引（用于日志）
/// - `config`: 配置
///
/// # 返回
/// 返回是否成功处理
pub async fn process_paper(
    executor: &JsExecutor,
    paper: QuestionPage,
    paper_index: usize,
    config: &Config,
) -> Result<bool> {
    let paper_id = paper.page_id.as_ref().context("试卷ID不能为空")?;

    log_paper_start(paper_index, &paper.name, paper_id, paper.stemlist.len());

    // 创建流程对象（只创建一次，复用）
    let question_flow = QuestionFlow::new(config);

    // 获取科目代码（提前计算，避免重复）
    let subject_code = crate::models::subject::Subject::from_str(&paper.subject)
        .with_context(|| format!("无法解析科目: {}", paper.subject))?
        .code()
        .to_string();

    let mut stats = QuestionStats::default();

    // ========== 遍历所有题目（Vec<Question>） ==========
    // 使用 enumerate() 自动获取索引（从 0 开始，所以需要 +1）
    for (index, question) in paper.stemlist.iter().enumerate() {
        let question_index = index + 1; // 题目索引从 1 开始
        log_question_start(paper_index, question_index, paper.stemlist.len());

        // 特殊处理：标题
        if question.is_title {
            match process_title(executor, paper_id, question_index, question, paper_index).await {
                Ok(_) => info!("[试卷 {}] ✓ 标题保存成功", paper_index),
                Err(e) => {
                    error!("[试卷 {}] 标题保存失败: {}", paper_index, e);
                    stats.skipped += 1;
                }
            }
            continue;
        }

        // 普通题目：构建上下文
        let ctx = QuestionCtx::new(
            paper_id.to_string(),
            paper_index,
            question_index,
            subject_code.clone(),
        );

        // 执行流程（委托给 QuestionFlow）
        match question_flow.run(executor, question, &ctx).await {
            Ok(ProcessResult::Success) => {
                stats.processed += 1;
            }
            Ok(ProcessResult::Skipped) => {
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
    match submit_paper(executor, paper_id, paper_index).await {
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

/// 处理标题
async fn process_title(
    executor: &JsExecutor,
    paper_id: &str,
    question_index: usize,
    question: &Question,
    paper_index: usize,
) -> Result<()> {
    info!("[试卷 {}] 检测到标题，开始传入标题", paper_index);

    let js_code = format!(
        r#"
        (async () => {{
            try {{
                const response = await fetch('/tiku/api/paper/saveTitle', {{
                    method: 'POST',
                    headers: {{
                        'Content-Type': 'application/json',
                    }},
                    body: JSON.stringify({{
                        paperId: {},
                        questionIndex: {},
                        titleContent: {}
                    }})
                }});
                const result = await response.json();
                return result;
            }} catch (error) {{
                return {{ error: error.message }};
            }}
        }})()
        "#,
        serde_json::to_string(paper_id)?,
        question_index,
        serde_json::to_string(&question.stem)?
    );

    executor.eval(js_code).await?;
    Ok(())
}

/// 提交试卷
async fn submit_paper(executor: &JsExecutor, paper_id: &str, paper_index: usize) -> Result<()> {
    info!("[试卷 {}] 📤 正在提交试卷...", paper_index);

    let js_code = format!(
        r#"
        (async () => {{
            try {{
                const response = await fetch('/tiku/api/paper/submitPaper', {{
                    method: 'POST',
                    headers: {{
                        'Content-Type': 'application/json',
                    }},
                    body: JSON.stringify({{
                        paperId: {}
                    }})
                }});
                const result = await response.json();
                return result;
            }} catch (error) {{
                return {{ error: error.message }};
            }}
        }})()
        "#,
        serde_json::to_string(paper_id)?
    );

    executor.eval(js_code).await?;
    Ok(())
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

fn log_paper_complete(paper_index: usize, stats: &QuestionStats, total: usize) {
    info!(
        "[试卷 {}] 题目统计: 成功 {}, 跳过 {}, 总计 {}",
        paper_index, stats.processed, stats.skipped, total
    );
    info!("\n[试卷 {}] ✅ 试卷处理完成\n", paper_index);
}
