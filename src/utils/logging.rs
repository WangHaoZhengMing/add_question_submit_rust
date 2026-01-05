use anyhow::Result;
/// 日志工具模块
///
/// 提供日志格式化和输出的辅助函数
use std::fs;
use tracing::info;

/// 初始化日志文件
///
/// # 参数
/// - `log_file_path`: 日志文件路径
///
/// # 返回
/// 返回是否成功初始化
pub fn init_log_file(log_file_path: &str) -> Result<()> {
    let log_header = format!(
        "{}\n试卷处理日志 - {}\n{}\n\n",
        "=".repeat(60),
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        "=".repeat(60)
    );
    fs::write(log_file_path, log_header)?;
    Ok(())
}

/// 记录程序启动信息
///
/// # 参数
/// - `max_concurrent`: 最大并发数
pub fn log_startup(max_concurrent: usize) {
    info!("{}", "=".repeat(60));
    info!("🚀 程序启动 - 多线程试卷处理模式");
    info!("📊 最大并发数: {}", max_concurrent);
    info!("{}", "=".repeat(60));
}

/// 记录试卷加载信息
///
/// # 参数
/// - `total`: 试卷总数
/// - `max_concurrent`: 最大并发数
pub fn log_papers_loaded(total: usize, max_concurrent: usize) {
    info!("✓ 找到 {} 个待处理的试卷", total);
    info!("📋 将以每批 {} 个的方式处理", max_concurrent);
    info!("💡 每批完成后再开始下一批\n");
}

/// 记录批次开始信息
///
/// # 参数
/// - `batch_num`: 批次编号
/// - `total_batches`: 批次总数
/// - `start`: 起始试卷编号
/// - `end`: 结束试卷编号
/// - `total`: 试卷总数
pub fn log_batch_start(
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

/// 记录批次完成信息
///
/// # 参数
/// - `batch_num`: 批次编号
/// - `success`: 成功数量
/// - `total`: 批次总数
pub fn log_batch_complete(batch_num: usize, success: usize, total: usize) {
    info!("\n{}", "─".repeat(60));
    info!("✓ 第 {} 批完成: 成功 {}/{}", batch_num, success, total);
    info!("{}", "─".repeat(60));
}

/// 打印最终统计信息
///
/// # 参数
/// - `success`: 成功数量
/// - `failed`: 失败数量
/// - `total`: 总数
/// - `log_file_path`: 日志文件路径
pub fn print_final_stats(success: usize, failed: usize, total: usize, log_file_path: &str) {
    info!("\n{}", "=".repeat(60));
    info!("📊 全部处理完成统计");
    info!(
        "完成时间: {}",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    );
    info!("{}", "=".repeat(60));
    info!("✅ 成功: {}/{}", success, total);
    info!("❌ 失败: {}", failed);
    info!("{}", "=".repeat(60));
    info!("\n日志已保存至: {}", log_file_path);
}

/// 截断长文本用于日志显示
///
/// # 参数
/// - `text`: 原始文本
/// - `max_len`: 最大长度
///
/// # 返回
/// 返回截断后的文本
pub fn truncate_text(text: &str, max_len: usize) -> String {
    if text.chars().count() > max_len {
        text.chars().take(max_len).collect::<String>() + "..."
    } else {
        text.to_string()
    }
}
