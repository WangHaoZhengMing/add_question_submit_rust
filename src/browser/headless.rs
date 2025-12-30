use std::path::Path;

use anyhow::Result;
use chromiumoxide::{Browser, BrowserConfig, Page};
use futures::StreamExt;
use tokio::time::sleep;
use tracing::{debug, error, info};

/// 启动无头浏览器并导航到指定 URL
pub async fn launch_headless_browser(url: &str) -> Result<(Browser, Page)> {
    info!("🚀 启动无头浏览器...");
    debug!("目标 URL: {}", url);

    // 配置无头浏览器
    let config = BrowserConfig::builder()
        .new_headless_mode()
        .chrome_executable(Path::new(
            r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        ))
        .args(vec![
            "--disable-gpu",             // Windows 无头模式必须禁用 GPU
            "--no-sandbox",              // 禁用沙盒，防止权限问题导致的崩溃
            "--disable-dev-shm-usage",   // 防止共享内存不足
            "--remote-debugging-port=0", // 这是一个好习惯，让浏览器自动选择端口
        ])
        .build()
        .map_err(|e| {
            error!("配置无头浏览器失败: {}", e);
            anyhow::anyhow!("配置无头浏览器失败: {}", e)
        })?;

    // 启动浏览器
    let (browser, mut handler) = Browser::launch(config).await.map_err(|e| {
        error!("启动无头浏览器失败: {}", e);
        anyhow::anyhow!("启动无头浏览器失败: {}", e)
    })?;
    debug!("无头浏览器启动成功");

    // 在后台处理浏览器事件
    tokio::spawn(async move {
        while let Some(h) = handler.next().await {
            if h.is_err() {
                break;
            }
        }
    });

    // 添加短暂延迟以等待浏览器状态同步
    sleep(tokio::time::Duration::from_millis(300)).await;

    // 创建新页面并导航
    let page = browser.new_page(url).await.map_err(|e| {
        error!("创建页面失败: {}", e);
        anyhow::anyhow!("创建页面失败: {}", e)
    })?;

    info!("✅ 无头浏览器已导航到: {}", url);
    debug!("页面导航成功");

    Ok((browser, page))
}
