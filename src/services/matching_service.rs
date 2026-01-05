/// 题目匹配服务
///
/// 负责使用 LLM 判断哪个搜索结果与给定题干最相似
use crate::clients::LlmClient;
use crate::config::Config;
use crate::models::question::SearchResult;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// 用于发送给 LLM 的搜索结果格式
#[derive(Debug, Serialize, Deserialize)]
struct SearchResultForLlm {
    index: usize,
    question_content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    xkw_question_similarity: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    img_count: Option<usize>,
}

impl From<(usize, &SearchResult)> for SearchResultForLlm {
    fn from((idx, sr): (usize, &SearchResult)) -> Self {
        Self {
            index: idx,
            question_content: sr.question_content.clone(),
            xkw_question_similarity: sr.xkw_question_similarity,
            img_count: sr.img_urls.as_ref().map(|urls| urls.len()),
        }
    }
}

/// 题目匹配服务
pub struct MatchingService {
    llm_client: LlmClient,
}

impl MatchingService {
    /// 创建新的匹配服务
    pub fn new(config: &Config) -> Self {
        // 使用特定模型进行题目匹配
        let llm_client = LlmClient::with_model(config, "doubao-seed-1.6");
        Self { llm_client }
    }

    /// 使用 LLM 判断哪个搜索结果与给定题干最相似
    ///
    /// # 参数
    /// - `search_results`: 搜索结果列表
    /// - `stem`: 待比较的题干
    /// - `imgs`: 题目的图片URL列表（可选）
    ///
    /// # 返回
    /// 返回最相似题目的索引（0-based）
    pub async fn find_best_match(
        &self,
        search_results: &[SearchResult],
        stem: &str,
        imgs: Option<&[String]>,
    ) -> Result<usize> {
        if search_results.is_empty() {
            anyhow::bail!("搜索结果列表不能为空");
        }

        // 尝试快速匹配
        if let Some(index) = self.try_quick_match(search_results) {
            return Ok(index);
        }

        // 使用 LLM 判断
        info!("🤖 正在使用LLM判断最佳匹配...");
        let index = self.ask_llm_for_index(search_results, stem, imgs).await?;
        info!("✓ LLM选择了第 {} 个结果", index + 1);

        Ok(index)
    }

    /// 尝试快速匹配（基于相似度阈值）
    ///
    /// 如果第一个结果的相似度明显高于第二个，则直接返回第一个
    fn try_quick_match(&self, search_results: &[SearchResult]) -> Option<usize> {
        if search_results.len() < 2 {
            return None;
        }

        if let (Some(s1), Some(s2)) = (
            search_results[0].xkw_question_similarity,
            search_results[1].xkw_question_similarity,
        ) {
            // 自动判断是0-1还是0-100
            let is_scale_100 = s1 > 1.0 || s2 > 1.0;
            let threshold = if is_scale_100 { 90.0 } else { 0.85 };
            let diff_threshold = if is_scale_100 { 5.0 } else { 0.05 };

            // 如果前两个相似度都大于阈值，并且相差大于阈值
            if s1 > threshold && (s1 - s2) > diff_threshold {
                info!(
                    "⚡ 满足快速匹配条件 (第一个相似度 > {} 且 差值 > {})，跳过LLM，直接选择第 1 个结果",
                    threshold, diff_threshold
                );
                return Some(0);
            }
        }

        None
    }

    /// 使用 LLM 判断最佳匹配索引
    async fn ask_llm_for_index(
        &self,
        search_results: &[SearchResult],
        stem: &str,
        imgs: Option<&[String]>,
    ) -> Result<usize> {
        // 构建搜索结果JSON，包含图片信息
        let results_for_llm: Vec<SearchResultForLlm> = search_results
            .iter()
            .enumerate()
            .map(|(idx, sr)| SearchResultForLlm::from((idx, sr)))
            .collect();

        let results_json = serde_json::to_string_pretty(&results_for_llm)?;

        // 构建图片信息说明
        let toml_img_info = self.build_image_info(imgs);
        let candidate_img_info = self.build_candidate_image_info(search_results);

        // 构建提示词
        let prompt = self.build_prompt(stem, &toml_img_info, &results_json, &candidate_img_info);

        let system_message = "你是一个专业的题目匹配助手，擅长通过文字内容和图片内容判断两个题目是否是同一个题目。你需要综合考虑题目的文字和图片来判断匹配度。当题目包含图片时，图片URL已包含在提示词中，你需要根据图片URL来判断图片内容是否相同或相似。";

        // 调用 LLM
        let response = self.llm_client.chat(&prompt, Some(system_message)).await?;

        // 解析索引
        let index = self.parse_index(&response, search_results.len())?;

        debug!("LLM选择了索引: {}", index);
        Ok(index)
    }

    /// 构建图片信息说明
    fn build_image_info(&self, imgs: Option<&[String]>) -> String {
        if let Some(imgs) = imgs {
            if imgs.is_empty() {
                "无图片".to_string()
            } else {
                let img_list: Vec<String> = imgs
                    .iter()
                    .enumerate()
                    .map(|(i, url)| format!("    图片 {}: {}", i + 1, url))
                    .collect();
                format!("包含 {} 张图片：\n{}", imgs.len(), img_list.join("\n"))
            }
        } else {
            "无图片".to_string()
        }
    }

    /// 构建候选题目图片信息
    fn build_candidate_image_info(&self, search_results: &[SearchResult]) -> String {
        let mut candidate_img_info = String::new();
        for (idx, sr) in search_results.iter().enumerate() {
            if let Some(img_urls) = &sr.img_urls {
                if !img_urls.is_empty() {
                    candidate_img_info.push_str(&format!(
                        "  候选题目 {}: 包含 {} 张图片\n",
                        idx,
                        img_urls.len()
                    ));
                    for (i, url) in img_urls.iter().enumerate() {
                        candidate_img_info.push_str(&format!("    图片 {}: {}\n", i + 1, url));
                    }
                }
            }
        }
        if candidate_img_info.is_empty() {
            candidate_img_info = "  所有候选题目均无图片\n".to_string();
        }
        candidate_img_info
    }

    /// 构建 LLM 提示词
    fn build_prompt(
        &self,
        stem: &str,
        toml_img_info: &str,
        results_json: &str,
        candidate_img_info: &str,
    ) -> String {
        format!(
            r#"你需要判断目标题目和候选题目列表中哪个是同一个题目。

【重要说明】
- 目标题目（来自TOML文件）和候选题目（来自题库搜索结果）都可能有图片
- 你需要同时比较题目的文字内容和图片内容
- 判断标准：是否是同一个题目，而不仅仅是相似
- 如果题目包含图片，图片内容也是判断的重要依据
- 两个题目都可能有图片，需要对比图片内容是否相同或相似

目标题目（来自TOML文件）：
  题干内容：{}
  图片信息：{}

候选题目列表（来自题库搜索结果）：
{}

候选题目图片信息：
{}

【判断步骤】
1. 首先比较题目的文字内容是否相同或高度一致
2. 如果目标题目有图片，检查候选题目是否也有相同或相似的图片
3. 如果候选题目有图片，检查目标题目是否也有相同或相似的图片
4. 综合文字内容和图片内容，判断哪个候选题目与目标题目是同一个题目
5. 优先选择文字和图片都匹配的题目

只返回该题目的index数字（0、1、2...），不要返回任何其他内容。"#,
            stem, toml_img_info, results_json, candidate_img_info
        )
    }

    /// 解析 LLM 返回的索引
    fn parse_index(&self, response: &str, max_len: usize) -> Result<usize> {
        let index_str = response.trim();
        let index = index_str
            .parse::<usize>()
            .map_err(|e| anyhow::anyhow!("无法解析索引: {}, 错误: {}", index_str, e))?;

        // 验证索引范围
        if index >= max_len {
            anyhow::bail!("返回的索引 {} 超出范围 [0, {}]", index, max_len - 1);
        }

        Ok(index)
    }
}
