//! 题目搜索服务 - 业务能力层
//!
//! 只负责"搜索"能力，不关心流程

use crate::infrastructure::JsExecutor;
use crate::models::question::SearchResult;
use anyhow::Result;
use regex::Regex;
use serde_json::Value as JsonValue;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn};

/// 题目搜索服务
///
/// 职责：
/// - 提供 k14 和 xueke 两种搜索能力
/// - 只处理单个题目的搜索
/// - 不出现 Vec<Question>
/// - 不出现 paper_id / question_index
/// - 不关心流程顺序
pub struct QuestionSearch {
    max_retries: usize,
}

impl QuestionSearch {
    /// 创建新的搜索服务
    pub fn new() -> Self {
        Self { max_retries: 50 }
    }

    /// # 返回
    /// 返回 (搜索结果列表, 完整 JSON 数据)
    pub async fn search_k14(
        &self,
        subject_code: &str,
        executor: &JsExecutor,
        stem: &str,
    ) -> Result<(Vec<SearchResult>, Vec<JsonValue>)> {
        debug!("k14题库搜索 - 题干长度: {} 字符", stem.len());

        // 重试逻辑
        for retry_count in 0..self.max_retries {
            sleep(Duration::from_millis(300)).await; // 避免请求过快
            let result = self.call_k14_api(executor, stem, subject_code).await?;

            debug!("K14 搜索结果: {:?}", result);

            // 检查是否被限流
            if self.is_rate_limited(&result) {
                warn!(
                    "API 请求频繁限制 (尝试 {}/{}), 等待 2 秒后重试...",
                    retry_count + 1,
                    self.max_retries
                );
                sleep(Duration::from_secs(2)).await;
                continue;
            }

            // 检查结果
            if result.is_null() {
                warn!("API 返回为空");
                break;
            }

            // 提取并解析搜索结果
            if let Some(data_array) = self.extract_search_data(&result) {
                let (search_results, full_data) = self.parse_search_results(data_array)?;
                return Ok((search_results, full_data));
            } else {
                // 检查其他错误
                if let Some(code) = result.get("code").and_then(|v| v.as_u64()) {
                    if code != 200 {
                        warn!("API 返回 data 为 None: {:?}", result);
                    }
                }
                break;
            }
        }

        // 超过最大重试次数或其他错误
        warn!("搜索失败，已重试 {} 次", self.max_retries);
        Ok((Vec::new(), Vec::new()))
    }

    /// 学科题库搜索
    ///
    /// # 参数
    /// - `executor`: JS 执行器
    /// - `stem`: 题干内容
    /// - `subject_code`: 科目代码（如 "1" 表示数学）
    ///
    /// # 返回
    /// 返回 (搜索结果列表, 完整 JSON 数据)
    pub async fn search_xueke(
        &self,
        executor: &JsExecutor,
        stem: &str,
        subject_code: &str,
    ) -> Result<(Vec<SearchResult>, Vec<JsonValue>)> {
        debug!("学科题库搜索 - 题干长度: {} 字符", stem.len());

        // 重试逻辑
        for retry_count in 0..self.max_retries {
            let result = self.call_xueke_api(executor, stem, subject_code).await?;

            // 检查是否被限流
            if self.is_rate_limited(&result) {
                warn!(
                    "API 请求频繁限制 (尝试 {}/{}), 等待 2 秒后重试...",
                    retry_count + 1,
                    self.max_retries
                );
                sleep(Duration::from_secs(2)).await;
                continue;
            }

            // 检查结果
            if result.is_null() {
                warn!("API 返回为空");
                break;
            }

            // 提取并解析搜索结果
            if let Some(data_array) = self.extract_search_data(&result) {
                let (search_results, full_data) = self.parse_search_results(data_array)?;
                return Ok((search_results, full_data));
            } else {
                // 检查其他错误
                if let Some(code) = result.get("code").and_then(|v| v.as_u64()) {
                    if code != 200 {
                        warn!("API 返回 data 为 None: {:?}", result);
                    }
                }
                break;
            }
        }

        // 超过最大重试次数或其他错误
        warn!("搜索失败，已重试 {} 次", self.max_retries);
        Ok((Vec::new(), Vec::new()))
    }

    /// 调用学科题库 API
    async fn call_xueke_api(
        &self,
        executor: &JsExecutor,
        stem: &str,
        subject_code: &str,
    ) -> Result<JsonValue> {
        let js_code = format!(
            r#"
            (async () => {{
                try {{
                    const response = await fetch("https://tps-tiku-api.staff.xdf.cn/api/third/xkw/question/v2/text-search", {{
                        method: 'POST',
                        headers: {{
                            'Content-Type': 'application/json',
                            'Accept': 'application/json, text/plain, */*',
                            'tikutoken': '732FD8402F95087CD934374135C46EE5'
                        }},
                        credentials: "include", // 这一行非常重要，用于发送 Cookie (XDFUUID, token 等)
                        body: JSON.stringify({{
                                "stage": "3",
                                "subject": {},
                                "text": {},
                        }})
                    }});
                    const result = await response.json();
                    return result;
                }} catch (error) {{
                    return {{ error: error.message }};
                }}
            }})()
            "#,
            serde_json::to_string(subject_code)?,
            serde_json::to_string(stem)?,
        );

        debug!("xueke 搜索 JS Playload: {}", &js_code);
        executor.eval(js_code).await
    }

    async fn call_k14_api(
        &self,
        executor: &JsExecutor,
        stem: &str,
        subject_code: &str,
    ) -> Result<JsonValue> {
        let js_code = format!(
            r#"
            (async () => {{
                try {{
                    const response = await fetch("https://tps-tiku-api.staff.xdf.cn/api/questionsimilar/queryByText", {{
                        method: 'POST',
                        headers: {{
                            'Content-Type': 'application/json',
                            'Accept': 'application/json, text/plain, */*',
                            'tikutoken': '732FD8402F95087CD934374135C46EE5'
                        }},
                        credentials: "include", // 这一行非常重要，用于发送 Cookie (XDFUUID, token 等)
                        body: JSON.stringify({{
                                "stage": "3",
                                "subject": {},
                                "text": {}
                                }})
                    }});
                    const result = await response.json();
                    return result;
                }} catch (error) {{
                    return {{ error: error.message }};
                }}
            }})()
            "#,
            serde_json::to_string(stem)?,
            serde_json::to_string(subject_code)?
        );

        debug!("K14 搜索 JS Playload: {}", &js_code);

        executor.eval(js_code).await
    }

    /// 检查是否被限流
    fn is_rate_limited(&self, result: &JsonValue) -> bool {
        if let Some(code) = result.get("code").and_then(|v| v.as_u64()) {
            code == 600
        } else {
            false
        }
    }

    /// 提取搜索数据
    fn extract_search_data<'a>(&self, result: &'a JsonValue) -> Option<&'a [JsonValue]> {
        result
            .get("data")?
            .as_array()
            .map(|v| v.as_slice())
    }

    /// 解析搜索结果
    fn parse_search_results(
        &self,
        data_array: &[JsonValue],
    ) -> Result<(Vec<SearchResult>, Vec<JsonValue>)> {
        let mut search_results = Vec::new();

        for item in data_array {
            // 提取题目内容
            let question_content = item
                .get("questionContent")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            // 提取图片 URL
            let img_urls = self.extract_image_urls(item)?;

            let search_result = SearchResult {
                question_content,
                xkw_question_similarity: item.get("xkwQuestionSimilarity").and_then(|v| v.as_f64()),
                img_urls,
            };
            search_results.push(search_result);
        }

        let full_data = data_array.to_vec();
        Ok((search_results, full_data))
    }

    /// 从 HTML 中提取图片 URL
    fn extract_image_urls(&self, item: &JsonValue) -> Result<Option<Vec<String>>> {
        if let Some(html) = item.get("questionContent").and_then(|v| v.as_str()) {
            let re = Regex::new(r#"<img\s+[^>]*src="([^"]+)""#)?;
            let urls: Vec<String> = re
                .captures_iter(html)
                .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
                .collect();

            if urls.is_empty() {
                Ok(None)
            } else {
                Ok(Some(urls))
            }
        } else {
            Ok(None)
        }
    }
}

impl Default for QuestionSearch {
    fn default() -> Self {
        Self::new()
    }
}
