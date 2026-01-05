/// 试卷处理服务
///
/// 负责整个试卷的处理流程，包括题目遍历、提交、文件清理
use crate::clients::TikuClient;
use crate::config::Config;
use crate::models::question::QuestionPage;
use crate::services::question_service::{ProcessResult, QuestionService};
use anyhow::{Context, Result};
use chromiumoxide::Page;
use std::fs;
use std::path::Path;
use tracing::{info, warn};

/// 题目处理统计
#[derive(Debug, Default)]
pub struct QuestionStats {
    pub processed: usize,
    pub skipped: usize,
}

/// 试卷处理服务
pub struct PaperService {
    question_service: QuestionService,
    tiku_client: TikuClient,
}

impl PaperService {
    /// 创建新的试卷处理服务
    pub fn new(config: &Config) -> Self {
        Self {
            question_service: QuestionService::new(config),
            tiku_client: TikuClient::new(config),
        }
    }

    /// 处理单个试卷
    ///
    /// # 参数
    /// - `page`: 浏览器页面对象
    /// - `page_data`: 试卷数据
    /// - `paper_index`: 试卷索引（用于日志）
    ///
    /// # 返回
    /// 返回是否成功处理
    pub async fn process_paper(
        &self,
        page: &Page,
        page_data: QuestionPage,
        paper_index: usize,
    ) -> Result<bool> {
        let page_id = page_data.page_id.as_ref().context("试卷ID不能为空")?;

        self.log_paper_start(paper_index, &page_data);

        let mut stats = QuestionStats::default();
        let mut question_index = 0;

        // 处理所有题目
        for question in page_data.stemlist.iter() {
            question_index += 1;
            self.log_question_start(paper_index, question_index, page_data.stemlist.len());

            // 如果是标题，单独处理
            if question.is_title {
                self.question_service
                    .process_title(page, page_id, question, question_index, paper_index)
                    .await?;
                continue;
            }

            // 处理普通题目
            match self
                .question_service
                .process_question(
                    page,
                    question,
                    page_id,
                    &page_data.subject,
                    question_index,
                    paper_index,
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
                    warn!("[试卷 {}] 处理题目失败: {}", paper_index, e);
                    stats.skipped += 1;
                }
            }
        }

        // 提交整个试卷
        self.submit_paper(page, page_id, paper_index).await?;

        // 清理文件
        self.cleanup_file(page_data.file_path.as_deref(), paper_index)?;

        // 输出统计信息
        self.log_paper_complete(paper_index, &stats, page_data.stemlist.len());

        Ok(true)
    }

    /// 提交整个试卷
    async fn submit_paper(&self, page: &Page, page_id: &str, paper_index: usize) -> Result<bool> {
        info!("\n[试卷 {}] {}", paper_index, "=".repeat(30));
        info!("[试卷 {}] 📋 提交整个试卷...", paper_index);

        let result = self.tiku_client.submit_paper(page, page_id).await?;

        if TikuClient::is_success_response(&result) {
            info!("[试卷 {}] ✓ 试卷提交成功", paper_index);
            Ok(true)
        } else {
            warn!("[试卷 {}] ⚠️ 试卷提交可能失败", paper_index);
            Ok(false)
        }
    }

    /// 清理已处理的文件
    fn cleanup_file(&self, file_path: Option<&str>, paper_index: usize) -> Result<()> {
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

    fn log_paper_start(&self, paper_index: usize, page_data: &QuestionPage) {
        info!("[试卷 {}] 开始处理", paper_index);
        info!("[试卷 {}] 名称: {}", paper_index, page_data.name);
        info!(
            "[试卷 {}] ID: {}",
            paper_index,
            page_data.page_id.as_ref().unwrap_or(&"未知".to_string())
        );
        info!(
            "[试卷 {}] 题目总数: {}",
            paper_index,
            page_data.stemlist.len()
        );
    }

    fn log_question_start(&self, paper_index: usize, question_index: usize, total: usize) {
        info!("\n[试卷 {}] {}", paper_index, "─".repeat(30));
        info!(
            "[试卷 {}] 处理第 {}/{} 道题目",
            paper_index, question_index, total
        );
    }

    fn log_paper_complete(&self, paper_index: usize, stats: &QuestionStats, total: usize) {
        info!(
            "[试卷 {}] 题目统计: 成功 {}, 跳过 {}, 总计 {}",
            paper_index, stats.processed, stats.skipped, total
        );
        info!("\n[试卷 {}] ✅ 试卷处理完成\n", paper_index);
    }
}
