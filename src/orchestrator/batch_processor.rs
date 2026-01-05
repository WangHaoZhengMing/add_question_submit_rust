//! 批量试卷处理器 - 编排层
//!
//! ## 职责
//!
//! 本模块是整个应用的入口，负责批量试卷的处理和资源管理。
//!
//! ## 核心功能
//!
//! 1. **应用初始化**：启动日志、连接浏览器、创建 JsExecutor
//! 2. **批量加载**：扫描并加载所有待处理的试卷（`Vec<QuestionPage>`）
//! 3. **并发控制**：使用 Semaphore 限制并发数量
//! 4. **分批处理**：将试卷分批次处理，每批完成后再开始下一批
//! 5. **资源管理**：持有 Browser 和 JsExecutor，确保生命周期正确
//! 6. **全局统计**：汇总所有试卷的处理结果
//!
//! ## 设计特点
//!
//! - **顶层编排**：不处理单个试卷的细节
//! - **资源所有者**：唯一持有 Browser 的模块
//! - **并发安全**：通过 Semaphore 和 tokio::spawn 实现并发
//! - **向下委托**：委托 paper_processor 处理单个试卷

use crate::browser;
use crate::config::Config;
use crate::infrastructure::JsExecutor;
use crate::models::QuestionPage;
use crate::orchestrator::paper_processor;
use anyhow::Result;
use chromiumoxide::Browser;
use std::fs;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{error, info, warn};

/// 应用主结构
pub struct App {
    config: Config,
    _browser: Browser,
    executor: JsExecutor,
}

impl App {
    /// 初始化应用
    pub async fn initialize(config: Config) -> Result<Self> {
        // 初始化日志文件
        init_log_file(&config.output_log_file)?;

        log_startup(&config);

        // 连接浏览器
        let (browser, page) = browser::connect_to_browser_and_page(
            config.browser_debug_port,
            Some(&config.target_url),
            None,
        )
        .await?;

        // 创建 JsExecutor（持有 page）
        let executor = JsExecutor::new(page);

        Ok(Self {
            config,
            _browser: browser,
            executor,
        })
    }

    /// 运行应用主逻辑
    pub async fn run(&self) -> Result<()> {
        // 加载所有待处理的试卷
        let all_papers = self.load_papers().await?;

        if all_papers.is_empty() {
            warn!("⚠️ 没有找到待处理的TOML文件，程序结束");
            return Ok(());
        }

        let total_papers = all_papers.len();
        log_papers_loaded(total_papers, self.config.max_concurrent_papers);

        // 处理所有试卷
        let stats = self.process_all_papers(all_papers).await?;

        // 输出最终统计
        print_final_stats(&stats, &self.config);

        Ok(())
    }

    /// 加载试卷
    async fn load_papers(&self) -> Result<Vec<QuestionPage>> {
        info!("\n📁 正在扫描待处理的试卷...");
        crate::models::load_all_toml_files(&self.config.toml_folder).await
    }

    /// 处理所有试卷
    async fn process_all_papers(&self, all_papers: Vec<QuestionPage>) -> Result<ProcessingStats> {
        let semaphore = Arc::new(Semaphore::new(self.config.max_concurrent_papers));
        let total_papers = all_papers.len();
        let mut stats = ProcessingStats {
            total: total_papers,
            ..Default::default()
        };

        // 分批处理
        for batch_start in (0..total_papers).step_by(self.config.max_concurrent_papers) {
            let batch_end = (batch_start + self.config.max_concurrent_papers).min(total_papers);
            let batch_papers = &all_papers[batch_start..batch_end];
            let batch_num = (batch_start / self.config.max_concurrent_papers) + 1;
            let total_batches = (total_papers + self.config.max_concurrent_papers - 1)
                / self.config.max_concurrent_papers;

            log_batch_start(
                batch_num,
                total_batches,
                batch_start + 1,
                batch_end,
                total_papers,
            );

            // 处理本批
            let batch_result = self
                .process_batch(batch_papers, batch_start, semaphore.clone())
                .await?;

            stats.success += batch_result.success;
            stats.failed += batch_result.failed;

            log_batch_complete(batch_num, &batch_result);
        }

        Ok(stats)
    }

    /// 处理单个批次
    async fn process_batch(
        &self,
        batch_papers: &[QuestionPage],
        batch_start: usize,
        semaphore: Arc<Semaphore>,
    ) -> Result<BatchResult> {
        let mut batch_handles = Vec::new();

        // 为本批创建并发任务
        for (idx, paper_data) in batch_papers.iter().enumerate() {
            let paper_index = batch_start + idx + 1;
            let permit = semaphore.clone().acquire_owned().await?;

            // 注意：JsExecutor 持有 page，但 page 可以安全地 clone
            // 因为 chromiumoxide 的 Page 内部使用 Arc
            let executor_page = self.executor.page().clone();
            let executor = JsExecutor::new(executor_page);

            let paper_data_clone = paper_data.clone();
            let config_clone = self.config.clone();

            let handle = tokio::spawn(async move {
                let _permit = permit;
                // 使用 JsExecutor 而不是 Page
                match paper_processor::process_paper(
                    &executor,
                    paper_data_clone,
                    paper_index,
                    &config_clone,
                )
                .await
                {
                    Ok(true) => Ok(true),
                    Ok(false) => Ok(false),
                    Err(e) => {
                        error!("[试卷 {}] ❌ 处理过程中发生错误: {}", paper_index, e);
                        Err(e)
                    }
                }
            });
            batch_handles.push((paper_index, handle));
        }

        // 等待本批所有任务完成
        let mut result = BatchResult::default();

        for (paper_index, handle) in batch_handles {
            match handle.await {
                Ok(Ok(true)) => {
                    result.success += 1;
                }
                Ok(Ok(false)) | Ok(Err(_)) => {
                    result.failed += 1;
                }
                Err(e) => {
                    error!("[试卷 {}] 任务执行失败: {}", paper_index, e);
                    result.failed += 1;
                }
            }
        }

        Ok(result)
    }
}

/// 处理统计
#[derive(Debug, Default)]
struct ProcessingStats {
    success: usize,
    failed: usize,
    total: usize,
}

/// 批次处理结果
#[derive(Debug, Default)]
struct BatchResult {
    success: usize,
    failed: usize,
}

// ========== 日志辅助函数 ==========

fn init_log_file(log_file_path: &str) -> Result<()> {
    let log_header = format!(
        "{}\n试卷处理日志 - {}\n{}\n\n",
        "=".repeat(60),
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        "=".repeat(60)
    );
    fs::write(log_file_path, log_header)?;
    Ok(())
}

fn log_startup(config: &Config) {
    info!("{}", "=".repeat(60));
    info!("🚀 程序启动 - 多线程试卷处理模式");
    info!("📊 最大并发数: {}", config.max_concurrent_papers);
    info!("{}", "=".repeat(60));
}

fn log_papers_loaded(total: usize, max_concurrent: usize) {
    info!("✓ 找到 {} 个待处理的试卷", total);
    info!("📋 将以每批 {} 个的方式处理", max_concurrent);
    info!("💡 每批完成后再开始下一批\n");
}

fn log_batch_start(batch_num: usize, total_batches: usize, start: usize, end: usize, total: usize) {
    info!("\n{}", "=".repeat(60));
    info!("📦 开始处理第 {}/{} 批", batch_num, total_batches);
    info!("📄 本批试卷: {}-{} / 共 {} 个", start, end, total);
    info!("{}", "=".repeat(60));
}

fn log_batch_complete(batch_num: usize, result: &BatchResult) {
    info!("\n{}", "─".repeat(60));
    info!(
        "✓ 第 {} 批完成: 成功 {}/{}",
        batch_num,
        result.success,
        result.success + result.failed
    );
    info!("{}", "─".repeat(60));
}

fn print_final_stats(stats: &ProcessingStats, config: &Config) {
    info!("\n{}", "=".repeat(60));
    info!("📊 全部处理完成统计");
    info!(
        "完成时间: {}",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    );
    info!("{}", "=".repeat(60));
    info!("✅ 成功: {}/{}", stats.success, stats.total);
    info!("❌ 失败: {}", stats.failed);
    info!("{}", "=".repeat(60));
    info!("\n日志已保存至: {}", config.output_log_file);
}
