//! LLM 服务 - 业务能力层
//!
//! 只负责"LLM 判断"能力，不关心流程

use anyhow::{Context, Result};
use serde_json::json;
use tracing::debug;

use crate::config::Config;
use crate::models::question::SearchResult;

/// LLM 服务
///
/// 职责：
/// - 调用 LLM API 进行题目匹配判断
/// - 只处理单个题目的匹配
/// - 不出现 Vec<Question>
/// - 不出现 paper_id / question_index
/// - 不关心流程顺序
pub struct LlmService {
    api_key: String,
    api_base_url: String,
}

impl LlmService {
    /// 创建新的 LLM 服务
    pub fn new(config: &Config) -> Self {
        Self {
            api_key: config.llm_api_key.clone(),
            api_base_url: config.llm_api_base_url.clone(),
        }
    }

    /// 从搜索结果中找到最佳匹配
    ///
    /// # 参数
    /// - `search_results`: 搜索结果列表
    /// - `stem`: 题干内容
    /// - `imgs`: 可选的图片 URL 列表
    ///
    /// # 返回
    /// 返回最佳匹配的索引
    pub async fn find_best_match(
        &self,
        search_results: &[SearchResult],
        stem: &str,
        imgs: Option<&[String]>,
    ) -> Result<usize> {
        if search_results.is_empty() {
            anyhow::bail!("搜索结果为空，无法进行匹配");
        }

        // 如果只有一个结果，直接返回
        if search_results.len() == 1 {
            debug!("只有一个搜索结果，直接选择");
            return Ok(0);
        }

        // 构建 prompt
        let prompt = self.build_prompt(search_results, stem, imgs);

        // 调用 LLM API
        let response = self.call_llm_api(&prompt).await?;

        // 解析响应
        let selected_index = self.parse_llm_response(&response, search_results.len())?;

        Ok(selected_index)
    }

    /// 构建 LLM prompt
    fn build_prompt(
        &self,
        search_results: &[SearchResult],
        stem: &str,
        imgs: Option<&[String]>,
    ) -> String {
        let mut prompt = format!(
            "你是一个题目匹配专家。请从以下候选题目中选择与目标题目最相似的一个。\n\n目标题目：\n{}\n\n",
            stem
        );

        // 添加图片信息
        if let Some(img_urls) = imgs {
            if !img_urls.is_empty() {
                prompt.push_str(&format!("目标题目包含 {} 张图片\n\n", img_urls.len()));
            }
        }

        // 添加候选题目
        prompt.push_str("候选题目：\n");
        for (i, result) in search_results.iter().enumerate() {
            prompt.push_str(&format!("\n{}. ", i + 1));

            // 添加相似度（如果有）
            if let Some(similarity) = result.xkw_question_similarity {
                prompt.push_str(&format!("[相似度: {:.2}] ", similarity));
            }

            // 添加题目内容
            let content = &result.question_content;
            let content_preview = if content.len() > 200 {
                format!("{}...", &content[..200])
            } else {
                content.clone()
            };
            prompt.push_str(&content_preview);

            // 添加图片信息
            if let Some(img_urls) = &result.img_urls {
                if !img_urls.is_empty() {
                    prompt.push_str(&format!(" [包含 {} 张图片]", img_urls.len()));
                }
            }
        }

        prompt.push_str("\n\n请直接返回最匹配的题目编号（1 到 {}），只返回数字，不要其他内容。", search_results.len());

        prompt
    }

    /// 调用 LLM API
    async fn call_llm_api(&self, prompt: &str) -> Result<String> {
        let client = reqwest::Client::new();

        let request_body = json!({
            "model": "gpt-3.5-turbo",
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": 0.3,
            "max_tokens": 10
        });

        let response = client
            .post(&self.api_base_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("LLM API 请求失败")?;

        let status = response.status();
        let response_text = response.text().await?;

        if !status.is_success() {
            anyhow::bail!("LLM API 返回错误: {} - {}", status, response_text);
        }

        let response_json: serde_json::Value = serde_json::from_str(&response_text)
            .context("解析 LLM API 响应失败")?;

        let content = response_json
            .get("choices")
            .and_then(|v| v.get(0))
            .and_then(|v| v.get("message"))
            .and_then(|v| v.get("content"))
            .and_then(|v| v.as_str())
            .context("LLM API 响应格式错误")?;

        Ok(content.to_string())
    }

    /// 解析 LLM 响应
    fn parse_llm_response(&self, response: &str, max_index: usize) -> Result<usize> {
        // 提取数字
        let response = response.trim();

        // 尝试直接解析
        if let Ok(num) = response.parse::<usize>() {
            if num > 0 && num <= max_index {
                return Ok(num - 1); // 转换为 0-based index
            }
        }

        // 尝试从文本中提取第一个数字
        for word in response.split_whitespace() {
            if let Ok(num) = word.trim_matches(|c: char| !c.is_numeric()).parse::<usize>() {
                if num > 0 && num <= max_index {
                    return Ok(num - 1);
                }
            }
        }

        // 如果都失败，返回第一个（默认选择）
        debug!("无法解析 LLM 响应: {}, 默认选择第一个", response);
        Ok(0)
    }
}
