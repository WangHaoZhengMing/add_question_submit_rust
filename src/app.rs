use crate::browser;
use crate::config::Config;
use crate::model::model::QuestionPage;
use crate::paper_processor;
use anyhow::Result;
use chromiumoxide::{Browser, Page};
use std::fs;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{error, info, warn};

/// 应用主结构
pub struct App {
    config: Config,
    browser: Browser,
    page: Page,
}

impl App {
    /// 初始化应用
    pub async fn initialize(config: Config) -> Result<Self> {
        // 初始化日志文件
        init_log_file(&config)?;

        log_startup(&config);

        // 连接浏览器
        let (browser, page) = browser::connect_to_browser_and_page(
            config.browser_debug_port,
            Some(&config.target_url),
            None,
        )
        .await?;

        Ok(Self {
            config,
            browser,
            page,
        })
    }

    /// 运行应用主逻辑
    pub async fn run(&self) -> Result<()> {
        // 加载所有待处理的试卷
        let all_papers = load_papers(&self.config).await?;

        if all_papers.is_empty() {
            warn!("⚠️ 没有找到待处理的TOML文件，程序结束");
            return Ok(());
        }

        let total_papers = all_papers.len();
        log_papers_loaded(total_papers, self.config.max_concurrent_papers);

        // 处理所有试卷
        let stats = process_all_papers(
            &self.browser,
            &self.page,
            all_papers,
            &self.config,
        )
        .await?;

        // 输出最终统计
        print_final_stats(&stats, &self.config);

        Ok(())
    }
}

/// 处理统计
#[derive(Debug, Default)]
struct ProcessingStats {
    success: usize,
    failed: usize,
    total: usize,
}

/// 加载试卷
async fn load_papers(config: &Config) -> Result<Vec<QuestionPage>> {
    info!("\n📁 正在扫描待处理的试卷...");
    crate::model::toml_loader::load_all_toml_files(&config.toml_folder).await
}

/// 处理所有试卷
async fn process_all_papers(
    browser: &Browser,
    page: &Page,
    all_papers: Vec<QuestionPage>,
    config: &Config,
) -> Result<ProcessingStats> {
    let semaphore = Arc::new(Semaphore::new(config.max_concurrent_papers));
    let total_papers = all_papers.len();
    let mut stats = ProcessingStats {
        total: total_papers,
        ..Default::default()
    };

    // 分批处理
    for batch_start in (0..total_papers).step_by(config.max_concurrent_papers) {
        let batch_end = (batch_start + config.max_concurrent_papers).min(total_papers);
        let batch_papers = &all_papers[batch_start..batch_end];
        let batch_num = (batch_start / config.max_concurrent_papers) + 1;
        let total_batches = (total_papers + config.max_concurrent_papers - 1) / config.max_concurrent_papers;

        log_batch_start(batch_num, total_batches, batch_start + 1, batch_end, total_papers);

        // 处理本批
        let batch_result = process_batch(
            browser,
            page,
            batch_papers,
            batch_start,
            semaphore.clone(),
            config,
        )
        .await?;

        stats.success += batch_result.success;
        stats.failed += batch_result.failed;

        log_batch_complete(batch_num, &batch_result);
    }

    Ok(stats)
}

/// 批次处理结果
#[derive(Debug, Default)]
struct BatchResult {
    success: usize,
    failed: usize,
}

/// 处理单个批次
async fn process_batch(
    _browser: &Browser,
    page: &Page,
    batch_papers: &[QuestionPage],
    batch_start: usize,
    semaphore: Arc<Semaphore>,
    config: &Config,
) -> Result<BatchResult> {
    let mut batch_handles = Vec::new();

    // 为本批创建并发任务
    for (idx, paper_data) in batch_papers.iter().enumerate() {
        let paper_index = batch_start + idx + 1;
        let permit = semaphore.clone().acquire_owned().await?;
        let page_clone = page.clone();
        let paper_data_clone: QuestionPage = paper_data.clone();
        let config_clone = config.clone();

        let handle = tokio::spawn(async move {
            let _permit = permit;
            match paper_processor::process_single_paper(
                &page_clone,
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

// ========== 日志辅助函数 ==========

fn init_log_file(config: &Config) -> Result<()> {
    let log_header = format!(
        "{}\n试卷处理日志 - {}\n{}\n\n",
        "=".repeat(60),
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        "=".repeat(60)
    );
    fs::write(&config.output_log_file, log_header)?;
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

fn log_batch_start(
    batch_num: usize,
    total_batches: usize,
    start: usize,
    end: usize,
    total: usize,
) {
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

