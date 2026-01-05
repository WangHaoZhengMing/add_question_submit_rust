/// 题库搜索服务
///
/// 负责从题库中搜索题目，包括重试逻辑和结果解析
use crate::clients::TikuClient;
use crate::config::Config;
use crate::models::question::SearchResult;
use anyhow::{Context, Result};
use chromiumoxide::Page;
use regex::Regex;
use serde_json::Value;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn};

/// 搜索服务
pub struct SearchService {
    tiku_client: TikuClient,
    max_retries: usize,
}

impl SearchService {
    /// 创建新的搜索服务
    pub fn new(config: &Config) -> Self {
        Self {
            tiku_client: TikuClient::new(config),
            max_retries: 50,
        }
    }

    /// 从题库搜索题目
    ///
    /// # 参数
    /// - `page`: 浏览器页面对象
    /// - `stem`: 题干内容
    /// - `subject`: 科目名称
    ///
    /// # 返回
    /// 返回 (搜索结果列表, 完整搜索结果JSON)
    pub async fn search(
        &self,
        page: &Page,
        stem: &str,
        subject: &str,
    ) -> Result<(Vec<SearchResult>, Vec<Value>)> {
        debug!("题干长度: {} 字符", stem.len());

        // 获取科目代码
        let subject_code = crate::models::subject::Subject::from_str(subject)
            .with_context(|| format!("无法解析科目: {}", subject))?
            .code()
            .to_string();

        // 重试逻辑
        for retry_count in 0..self.max_retries {
            let result = self
                .tiku_client
                .search_question(page, stem, &subject_code)
                .await?;

            // 检查是否需要重试
            if TikuClient::is_rate_limited(&result) {
                warn!(
                    "API请求频繁限制 (尝试 {}/{}), 等待2秒后重试...",
                    retry_count + 1,
                    self.max_retries
                );
                sleep(Duration::from_secs(2)).await;
                continue; // 重试
            }

            // 检查结果
            if result.is_null() {
                warn!("API返回为空");
                break;
            }

            // 提取并处理搜索结果
            if let Some(data_array) = TikuClient::extract_search_data(&result) {
                let (search_results, full_search_result) = self.parse_search_results(data_array)?;
                return Ok((search_results, full_search_result));
            } else {
                // 检查是否有其他错误
                if let Some(code) = result.get("code").and_then(|v| v.as_u64()) {
                    if code != 600 {
                        warn!("API返回data为None: {:?}", result);
                    }
                }
                break;
            }
        }

        // 超过最大重试次数或其他错误
        warn!("搜索失败，已重试 {} 次", self.max_retries);
        Ok((Vec::new(), Vec::new()))
    }

    /// 解析搜索结果
    fn parse_search_results(
        &self,
        data_array: &[Value],
    ) -> Result<(Vec<SearchResult>, Vec<Value>)> {
        let mut search_results = Vec::new();

        for item in data_array {
            // 提取题目内容
            let question_content = item
                .get("questionContent")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            // 提取图片URL
            let img_urls = self.extract_image_urls(item)?;

            let search_result = SearchResult {
                question_content,
                xkw_question_similarity: item.get("xkwQuestionSimilarity").and_then(|v| v.as_f64()),
                img_urls,
            };
            search_results.push(search_result);
        }

        let full_search_result = data_array.to_vec();
        Ok((search_results, full_search_result))
    }

    /// 从 HTML 中提取图片 URL
    fn extract_image_urls(&self, item: &Value) -> Result<Option<Vec<String>>> {
        if let Some(html) = item.get("questionContent").and_then(|v| v.as_str()) {
            // 使用正则表达式提取所有 img 标签的 src 属性
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
