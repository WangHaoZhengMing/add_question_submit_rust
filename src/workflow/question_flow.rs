//! 题目处理流程 - 流程层
//!
//! 核心职责：定义"一道题"的完整处理流程
//!
//! 流程顺序：
//! 1. search_k14 → LLM 判断 → 提交
//! 2. search_xueke → LLM 判断 → 提交
//! 3. warn.txt（兜底）

use anyhow::Result;
use serde_json::{json, Value as JsonValue};
use tracing::{error, info, warn};

use crate::config::Config;
use crate::infrastructure::JsExecutor;
use crate::models::question::Question;
use crate::services::{LlmService, QuestionSearch, WarnWriter};
use crate::workflow::question_ctx::QuestionCtx;

/// 题目处理结果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessResult {
    /// 处理成功
    Success,
    /// 跳过（未找到匹配）
    Skipped,
}

/// 题目处理流程

/// - 编排完整的题目处理流程
/// - 决定何时搜索、何时判断、何时兜底
/// - 不持有任何资源（page）
/// - 只依赖业务能力（services）
pub struct QuestionFlow {
    question_search: QuestionSearch,
    llm_service: LlmService,
    warn_writer: WarnWriter,
    verbose_logging: bool,
}

impl QuestionFlow {
    /// 创建新的题目处理流程
    pub fn new(config: &Config) -> Self {
        Self {
            question_search: QuestionSearch::new(),
            llm_service: LlmService::new(config),
            warn_writer: WarnWriter::new(),
            verbose_logging: config.verbose_logging,
        }
    }

    pub async fn run(
        &self,
        executor: &JsExecutor,
        question: &Question,
        ctx: &QuestionCtx,
    ) -> Result<ProcessResult> {
        let stem = &question.stem;

        // 显示题干预览
        self.log_stem(ctx.paper_index, stem);

        // ========== 流程 1: 尝试 k14 搜索 ==========
        info!("[试卷 {}] 🔍 尝试 K14 题库搜索...", ctx.paper_index);

        let (k14_results, k14_full_data) = self
            .question_search
            .search_k14(stem, executor, &ctx.subject_code)
            .await?;

        if !k14_results.is_empty() {
            info!(
                "[试卷 {}] ✓ K14 搜索完成，找到 {} 个相似题目",
                ctx.paper_index,
                k14_results.len()
            );

            // LLM 判断
            match self
                .llm_service
                .find_best_match(&k14_results, stem, question.imgs.as_deref())
                .await
            {
                // 情况 1: 成功找到匹配 (Some)
                Ok(Some(selected_index)) => {
                    info!(
                        "[试卷 {}] ✓ LLM 选择了第 {} 个结果 (相似度: {:?})",
                        ctx.paper_index,
                        selected_index + 1,
                        k14_results[selected_index].xkw_question_similarity
                    );

                    // 提交逻辑
                    return self
                        .submit_question(executor, &k14_full_data[selected_index], ctx)
                        .await;
                }

                // 情况 2: LLM 明确表示没找到，或者重试 3 次后仍无法解析 (None)
                Ok(None) => {
                    info!(
                        "[试卷 {}] K14 LLM 未找到匹配结果 (or 已尝试 3 次)，尝试学科网题库...",
                        ctx.paper_index
                    );
                    // 这里不需要写代码，自然会跳出 match，执行下面的 "else" 或者后续逻辑
                }

                // 情况 3: 严重的 API 错误 (3次全挂)
                Err(e) => {
                    error!(
                        "[试卷 {}] ⚠️ K14 LLM 调用彻底失败: {} (已重试 3 次)",
                        ctx.paper_index, e
                    );
                }
            }
        } else {
            info!("[试卷 {}] K14 未找到结果，尝试学科题库", ctx.paper_index);
        }

        // ========== 流程 2: 尝试 xueke 搜索 ==========
        info!("[试卷 {}] 🔍 正在学科题库中搜索...", ctx.paper_index);

        let (xueke_results, xueke_full_data) = self
            .question_search
            .search_xueke(executor, stem, &ctx.subject_code)
            .await?;

        info!(
            "[试卷 {}] ✓ 学科题库搜索完成，找到 {} 个相似题目",
            ctx.paper_index,
            xueke_results.len()
        );

        // 分支：未找到结果
        if xueke_results.is_empty() {
            warn!(
                "[试卷 {}] ⚠️ 未找到相似题目，写入 warn.txt",
                ctx.paper_index
            );
            self.write_warn(ctx, question).await?;
            return Ok(ProcessResult::Skipped);
        }

        // 详细日志（如果启用）
        if self.verbose_logging {
            self.log_search_results(ctx.paper_index, &xueke_results);
        }

        // LLM 判断
        let match_result = self
            .llm_service
            .find_best_match(&xueke_results, stem, question.imgs.as_deref())
            .await?;

        match match_result {
            Some(index) => {
                // LLM 找到了匹配项
                info!(
                    "[试卷 {}] ✓ LLM 选择了第 {} 个结果 (相似度: {:?})",
                    ctx.paper_index,
                    index + 1,
                    xueke_results[index].xkw_question_similarity
                );

                // 提交
                return self
                    .submit_question(executor, &xueke_full_data[index], ctx)
                    .await;
            }
            None => {
                warn!(
                    "[试卷 {}] ⚠️ 学科题库有结果但 LLM 认为都不匹配，写入 warn.txt",
                    ctx.paper_index
                );
                self.write_warn(ctx, question).await?;
                return Ok(ProcessResult::Skipped);
            }
        }
    }

    /// 提交题目到题库
    ///
    /// 使用 JsExecutor 执行提交操作
    async fn submit_question(
        &self,
        executor: &JsExecutor,
        search_result: &JsonValue,
        ctx: &QuestionCtx,
    ) -> Result<ProcessResult> {
        info!("[试卷 {}] 📤 正在提交题目到题库...", ctx.paper_index);

        // 构建题目数据
        let question_data = self.build_question_data(search_result, ctx);

        // 调用 JS 提交
        let js_code = format!(
            r#"
            (async () => {{
                try {{
                    const response = await fetch('https://tps-tiku-api.staff.xdf.cn/question/new/save', {{
                        method: 'POST',
                        headers: {{
                            'Content-Type': 'application/json',
                                "Accept": "application/json, text/plain, */*",
                                // 关键补充：根据之前的分析，这个头是必须的
                                "tikutoken": "732FD8402F95087CD934374135C46EE5",
                        }},
                        credentials: 'include',
                        body: JSON.stringify({})
                    }});
                    const result = await response.json();
                    return result;
                }} catch (error) {{
                    return {{ error: error.message }};
                }}
            }})()
            "#,
            question_data
        );

        let result = executor.eval(js_code).await?;

        // 检查提交结果
        if self.is_success_response(&result) {
            info!("[试卷 {}] ✓ 题目提交成功", ctx.paper_index);
            Ok(ProcessResult::Success)
        } else {
            warn!("[试卷 {}] ⚠️ 题目提交失败: {:?}", ctx.paper_index, result);
            // 提交失败也写入 warn.txt
            self.write_warn_by_ctx(ctx, "提交失败").await?;
            Ok(ProcessResult::Skipped)
        }
    }

    /// 构建题目数据
    fn build_question_data(&self, search_result: &JsonValue, ctx: &QuestionCtx) -> JsonValue {
        let mut data = search_result.clone();
        data["addFlag"] = json!(1);
        data["paperId"] = json!(&ctx.paper_id);
        data["sysCode"] = json!(1);
        data["questionType"] = json!("1");
        data["relationType"] = json!(1);
        data["inputType"] = json!(1);
        data["questionIndex"] = json!(ctx.question_index);
        data
    }

    /// 检查响应是否成功
    fn is_success_response(&self, result: &JsonValue) -> bool {
        if let Some(code) = result.get("code").and_then(|v| v.as_u64()) {
            code == 200
        } else {
            false
        }
    }

    /// 写入警告日志
    async fn write_warn(&self, ctx: &QuestionCtx, question: &Question) -> Result<()> {
        self.warn_writer
            .write(&ctx.paper_id, ctx.question_index, &question.stem)
            .await?;

        warn!(
            "[试卷 {}] ⚠️ 已写入 warn.txt: 题目 {}",
            ctx.paper_index, ctx.question_index
        );

        Ok(())
    }

    /// 写入警告日志（使用上下文信息）
    async fn write_warn_by_ctx(&self, ctx: &QuestionCtx, reason: &str) -> Result<()> {
        self.warn_writer
            .write(&ctx.paper_id, ctx.question_index, reason)
            .await?;

        warn!(
            "[试卷 {}] ⚠️ 已写入 warn.txt: 题目 {} (原因: {})",
            ctx.paper_index, ctx.question_index, reason
        );

        Ok(())
    }

    // ========== 日志辅助方法 ==========

    /// 显示题干预览
    fn log_stem(&self, paper_index: usize, stem: &str) {
        let stem_preview = if stem.chars().count() > 80 {
            stem.chars().take(80).collect::<String>() + "..."
        } else {
            stem.to_string()
        };
        info!("[试卷 {}] 题干: {}", paper_index, stem_preview);
    }

    /// 显示搜索结果
    fn log_search_results(
        &self,
        paper_index: usize,
        search_results: &[crate::models::question::SearchResult],
    ) {
        for (i, sr) in search_results.iter().take(2).enumerate() {
            info!(
                "[试卷 {}]   {}. 相似度: {:?}",
                paper_index,
                i + 1,
                sr.xkw_question_similarity
            );
        }
    }
}
