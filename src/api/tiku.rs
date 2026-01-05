//! 题库 API 模块
//!
//! 负责所有与题库 API 的交互，包括搜索、保存、提交等操作

use anyhow::{Context, Result};
use chromiumoxide::Page;
use regex::Regex;
use serde_json::{json, Value};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info, warn};

/// 搜索题目（带重试逻辑）
///
/// # 参数
/// - `page`: 浏览器页面对象
/// - `stem`: 题干内容
/// - `subject_code`: 科目代码
/// - `max_retries`: 最大重试次数
///
/// # 返回
/// 返回搜索结果数组
pub async fn search_questions_xueku(
    page: &Page,
    stem: &str,
    subject_code: &str,
    max_retries: usize,
) -> Result<Vec<Value>> {
    debug!("搜索题目，题干长度: {} 字符", stem.len());

    let search_data = json!({
        "stage": "3",
        "subject": subject_code,
        "text": stem
    });

    // 重试逻辑
    for retry_count in 0..max_retries {
        let script = build_search_script(&search_data, "text-search")?;

        let result: Value = page
            .evaluate(script.as_str())
            .await?
            .into_value()
            .context("无法执行搜索脚本")?;

        // 检查是否需要重试（频率限制）
        if is_rate_limited(&result) {
            warn!(
                "API请求频繁限制 (尝试 {}/{}), 等待2秒后重试...",
                retry_count + 1,
                max_retries
            );
            sleep(Duration::from_secs(2)).await;
            continue;
        }

        // 提取搜索结果
        if let Some(data) = result.get("data") {
            if !data.is_null() {
                if let Some(array) = data.as_array() {
                    let results = parse_search_results(array)?;
                    return Ok(results);
                }
            }
        }

        // 如果不是频率限制，就不继续重试
        if !is_rate_limited(&result) {
            break;
        }
    }

    warn!("搜索失败，已重试 {} 次", max_retries);
    Ok(Vec::new())
}

/// 保存题目
///
/// # 参数
/// - `page`: 浏览器页面对象
/// - `question_data`: 题目数据
pub async fn save_question(page: &Page, question_data: &Value) -> Result<()> {
    let script = build_api_call("question/new/save", question_data)?;

    debug!("保存题目");

    let result: Value = page.evaluate(script.as_str()).await?.into_value()?;

    if !result.is_null() {
        info!("✓ 题目保存成功");
    } else {
        warn!("⚠️ 题目保存可能失败");
    }

    Ok(())
}

/// 保存标题
///
/// # 参数
/// - `page`: 浏览器页面对象
/// - `paper_id`: 试卷ID
/// - `question_index`: 题目索引
/// - `stem`: 标题内容
pub async fn save_title(
    page: &Page,
    paper_id: &str,
    question_index: usize,
    stem: &str,
) -> Result<()> {
    let title_data = json!({
        "paperId": paper_id,
        "inputType": 1,
        "questionIndex": question_index,
        "questionType": "2",
        "addFlag": 1,
        "sysCode": 1,
        "relationType": 0,
        "questionSource": 3,
        "structureType": "biaoti",
        "questionInfo": {
            "stem": format!("<span>{}</span>", stem)
        }
    });

    let script = build_api_call("question/new/save", &title_data)?;

    debug!("保存标题: {}", stem);

    let _: serde_json::Value = page.evaluate(script.as_str()).await?.into_value()?;

    info!("✓ 标题保存成功");

    Ok(())
}

/// 提交试卷
///
/// # 参数
/// - `page`: 浏览器页面对象
/// - `paper_id`: 试卷ID
pub async fn submit_paper(page: &Page, paper_id: &str) -> Result<()> {
    let submit_data = json!({
        "paperId": paper_id,
        "type": "NEW_INPUT"
    });

    let script = build_api_call("paper/process/submit", &submit_data)?;

    info!("📋 提交试卷...");

    let result: Value = page.evaluate(script.as_str()).await?.into_value()?;

    if !result.is_null() {
        info!("✓ 试卷提交成功");
    } else {
        warn!("⚠️ 试卷提交可能失败");
    }

    Ok(())
}

// ========== 辅助函数 ==========

/// 构建搜索脚本
fn build_search_script(search_data: &Value, search_destination: &str) -> Result<String> {
    let search_data_json = serde_json::to_string(search_data)?;

    Ok(format!(
        r#"
        (async () => {{
            try {{
                const res = await fetch("https://tps-tiku-api.staff.xdf.cn/api/third/xkw/question/v2/{}", {{
                    method: "POST",
                    headers: {{
                        "Content-Type": "application/json",
                        "Accept": "application/json, text/plain, */*"
                    }},
                    credentials: "include",
                    body: JSON.stringify({})
                }});
                const data = await res.json();
                return data;
            }} catch (err) {{
                console.error("搜索请求失败:", err);
                return null;
            }}
        }})()
        "#,
        search_destination, search_data_json
    ))
}

/// 构建通用的 API 调用脚本
fn build_api_call(endpoint: &str, data: &Value) -> Result<String> {
    let json_data = serde_json::to_string(data)?;

    Ok(format!(
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
                console.error("API请求失败:", err);
                return null;
            }}
        }})()
        "#,
        endpoint, json_data
    ))
}

/// 检查是否是频率限制错误
fn is_rate_limited(result: &Value) -> bool {
    if let Some(code) = result.get("code").and_then(|v| v.as_u64()) {
        if code == 600 {
            if let Some(msg) = result.get("message").and_then(|v| v.as_str()) {
                return msg.contains("请求过于频繁");
            }
        }
    }
    false
}

/// 解析搜索结果，提取题目内容和图片URL
fn parse_search_results(data_array: &[Value]) -> Result<Vec<Value>> {
    let mut results = Vec::new();

    for item in data_array {
        let mut item_clone = item.clone();

        // 提取图片URL
        if let Some(html) = item.get("questionContent").and_then(|v| v.as_str()) {
            if let Ok(re) = Regex::new(r#"<img\s+[^>]*src="([^"]+)""#) {
                let urls: Vec<String> = re
                    .captures_iter(html)
                    .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
                    .collect();

                if !urls.is_empty() {
                    item_clone["imgUrls"] = json!(urls);
                }
            }
        }

        results.push(item_clone);
    }

    Ok(results)
}
