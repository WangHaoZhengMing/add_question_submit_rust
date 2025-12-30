use anyhow::Result;
use chromiumoxide::{Browser, Page};
use futures::StreamExt;
use tokio::time::sleep;
use tracing::{debug, error, info};

/// 连接到浏览器并获取页面
pub async fn connect_to_browser_and_page(
    port: u16,
    target_url: Option<&str>,
    target_title: Option<&str>,
) -> Result<(Browser, Page)> {
    let browser_url = format!("http://localhost:{}", port);
    info!("正在连接到浏览器: {}", browser_url);
    debug!("目标 URL: {:?}, 目标标题: {:?}", target_url, target_title);

    let (browser, mut handler) = Browser::connect(&browser_url).await.map_err(|e| {
        error!("连接浏览器失败: {}", e);
        e
    })?;
    debug!("浏览器连接成功");

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

    let pages = browser.pages().await?;
    debug!("获取到 {} 个页面", pages.len());

    // 如果指定了目标标题，尝试查找匹配的页面
    if let Some(title) = target_title {
        debug!("正在查找标题包含 '{}' 的页面", title);
        for p in pages.iter() {
            if let Ok(Some(page_title)) = p.get_title().await {
                debug!("检查页面标题: {}", page_title);
                if page_title.contains(title) {
                    info!("✓ 找到目标页面: {}", page_title);
                    return Ok((browser, p.clone()));
                }
            }
        }
        debug!("未找到匹配的页面，将创建新页面");
    }

    // 如果没有找到匹配的页面，创建新页面
    let new_page = if let Some(url) = target_url {
        debug!("创建新页面并导航到: {}", url);
        let page = browser.new_page("about:blank").await.map_err(|e| {
            error!("创建新页面失败: {}", e);
            e
        })?;
        page.goto(url).await.map_err(|e| {
            error!("导航到 {} 失败: {}", url, e);
            e
        })?;
        info!("已导航到: {}", url);
        debug!("页面导航成功");
        page
    } else {
        debug!("创建空白页面");
        browser.new_page("about:blank").await.map_err(|e| {
            error!("创建空白页面失败: {}", e);
            e
        })?
    };

    Ok((browser, new_page))
}

