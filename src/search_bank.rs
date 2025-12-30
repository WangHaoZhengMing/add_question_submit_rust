use crate::{model::model::SearchResult, subject::{Subject}};
use anyhow::{Context, Result};
use chromiumoxide::Page;
use regex::Regex;
use serde_json::Value;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn};

/// 从题库搜索题目
/// 
/// # 参数
/// - `page`: 浏览器页面对象
/// - `stem`: 题干内容
/// - `max_retries`: 最大重试次数
/// 
/// # 返回
/// 返回 (搜索结果列表, 完整搜索结果JSON)
pub async fn search_from_bank(
    page: &Page,
    stem: &str,
    max_retries: usize,
    subject: &str,
) -> Result<(Vec<SearchResult>, Vec<Value>)> {
    debug!("题干长度: {} 字符", stem.len());
    
    let search_data = serde_json::json!({
        "stage": "3",
        "subject": Subject::from_str(subject).unwrap().code().to_string(),
        "text": stem
    });
    
    let search_data_json = serde_json::to_string(&search_data)?;
    
    // 重试逻辑
    for retry_count in 0..max_retries {
        let script = format!(
            r#"
            (async () => {{
                try {{
                    const res = await fetch("https://tps-tiku-api.staff.xdf.cn/api/third/xkw/question/v2/text-search", {{
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
            search_data_json
        );
        
        let result: Value = page
            .evaluate(script.as_str())
            .await?
            .into_value()
            .context("无法执行搜索脚本")?;
        
        // 检查是否需要重试
        if let Some(code) = result.get("code").and_then(|v| v.as_u64()) {
            if code == 600 {
                if let Some(msg) = result.get("message").and_then(|v| v.as_str()) {
                    if msg.contains("请求过于频繁") {
                        warn!("API请求频繁限制 (尝试 {}/{})，等待2秒后重试...", retry_count + 1, max_retries);
                        sleep(Duration::from_secs(2)).await;
                        continue; // 重试
                    }
                }
            }
        }
        
        // 检查结果
        if result.is_null() {
            warn!("API返回为空");
            break;
        }
        
        if let Some(data) = result.get("data") {
            if data.is_null() {
                if let Some(code) = result.get("code").and_then(|v| v.as_u64()) {
                    if code != 600 {
                        warn!("API返回data为None: {:?}", result);
                    }
                }
                break;
            }
            
            // 提取data字段并转换为结构体列表
            if let Some(data_array) = data.as_array() {
                let mut search_results = Vec::new();
                
                for item in data_array {
                    // 提取题目内容
                    let question_content = item
                        .get("questionContent")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    
                    // 提取图片URL
                    let img_urls: Option<Vec<String>> = if let Some(html) = item.get("questionContent").and_then(|v| v.as_str()) {
                        // 使用正则表达式提取所有 img 标签的 src 属性
                        if let Ok(re) = Regex::new(r#"<img\s+[^>]*src="([^"]+)""#) {
                            let urls: Vec<String> = re
                                .captures_iter(html)
                                .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
                                .collect();
                            if urls.is_empty() {
                                None
                            } else {
                                Some(urls)
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    
                    let search_result = SearchResult {
                        question_content,
                        xkw_question_similarity: item
                            .get("xkwQuestionSimilarity")
                            .and_then(|v| v.as_f64()),
                        img_urls,
                    };
                    search_results.push(search_result);
                }
                
                let full_search_result = data_array.clone();
                return Ok((search_results, full_search_result));
            }
        } else {
            warn!("API返回缺少data字段: {:?}", result);
            break;
        }
    }
    
    // 超过最大重试次数或其他错误
    warn!("搜索失败，已重试 {} 次", max_retries);
    Ok((Vec::new(), Vec::new()))
}

