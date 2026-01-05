//! LLM 服务 - 业务能力层
//!
//! 只负责"LLM 判断"能力，不关心流程
//!
//! ## 技术栈
//! - 使用 `async-openai` crate 进行 API 调用
//! - 支持自定义 API 端点和模型
//! - 兼容 OpenAI API 的服务（如 Azure, Gemini, Doubao 等）

use anyhow::Result;
use async_openai::{
    config::OpenAIConfig,
    types::chat::{
        ChatCompletionRequestMessage, ChatCompletionRequestMessageContentPartImage,
        ChatCompletionRequestMessageContentPartText, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs, ChatCompletionRequestUserMessageContent,
        ChatCompletionRequestUserMessageContentPart, CreateChatCompletionRequestArgs, ImageDetail,
        ImageUrl,
    },
    Client,
};
use tracing::{debug, warn};

use crate::config::Config;
use crate::models::question::SearchResult;

/// LLM 服务
///
/// 职责：
/// - 调用 LLM API 进行题目匹配判断
/// - 提供通用的 LLM 调用接口
/// - 只处理单个题目的匹配
/// - 不出现 Vec<Question>
/// - 不出现 paper_id / question_index
/// - 不关心流程顺序
pub struct LlmService {
    client: Client<OpenAIConfig>,
    model_name: String,
}

impl LlmService {
    /// 创建新的 LLM 服务
    pub fn new(config: &Config) -> Self {
        // 配置 OpenAI 客户端（兼容 OpenAI API 的服务）
        let openai_config = OpenAIConfig::new()
            .with_api_key(&config.llm_api_key)
            .with_api_base(&config.llm_api_base_url);

        let client = Client::with_config(openai_config);

        Self {
            client,
            model_name: config.llm_model_name.clone(),
        }
    }

    /// 通用的 LLM 调用函数
    ///
    /// 这是最基础的 LLM 调用接口，其他所有 LLM 相关功能都应该基于此函数。
    ///
    /// # 参数
    /// - `user_message`: 用户消息内容
    /// - `system_message`: 系统消息（可选）
    /// - `imgs`: 图片 URL 列表（可选），会追加到用户消息中
    ///
    /// # 返回
    /// 返回 LLM 的响应内容（字符串）
    ///
    /// # 示例
    /// ```no_run
    /// # use add_question_submit::services::LlmService;
    /// # async fn example(service: &LlmService) -> anyhow::Result<()> {
    /// let response = service.send_to_llm(
    ///     "你好，请介绍一下你自己",
    ///     Some("你是一个友好的助手"),
    ///     None
    /// ).await?;
    /// println!("LLM 响应: {}", response);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn send_to_llm(
        &self,
        user_message: &str,
        system_message: Option<&str>,
        imgs: Option<&[String]>,
    ) -> Result<String> {
        debug!("调用 LLM API，模型: {}", self.model_name);
        debug!("用户消息长度: {} 字符", user_message.len());
        if let Some(img_urls) = imgs {
            debug!("包含 {} 张图片", img_urls.len());
        }

        // 构建消息列表
        let mut messages = Vec::new();

        // 添加系统消息（如果提供）
        if let Some(sys_msg) = system_message {
            let system_msg = ChatCompletionRequestSystemMessageArgs::default()
                .content(sys_msg)
                .build()?;
            messages.push(ChatCompletionRequestMessage::System(system_msg));
        }

        // 构建用户消息内容（支持图片）
        let user_msg = if let Some(img_urls) = imgs {
            if !img_urls.is_empty() {
                // 使用 Vision API：构建包含文本和图片的内容
                let mut content_parts: Vec<ChatCompletionRequestUserMessageContentPart> =
                    Vec::new();

                // 添加文本部分
                content_parts.push(ChatCompletionRequestUserMessageContentPart::Text(
                    ChatCompletionRequestMessageContentPartText {
                        text: user_message.to_string(),
                    },
                ));

                // 添加图片部分
                for url in img_urls.iter() {
                    content_parts.push(ChatCompletionRequestUserMessageContentPart::ImageUrl(
                        ChatCompletionRequestMessageContentPartImage {
                            image_url: ImageUrl {
                                url: url.clone(),
                                detail: Some(ImageDetail::Auto), // Auto, High, Low
                            },
                        },
                    ));
                }

                debug!("使用 Vision API，包含 {} 张图片", img_urls.len());

                // 构建包含多部分内容的用户消息
                ChatCompletionRequestUserMessageArgs::default()
                    .content(ChatCompletionRequestUserMessageContent::Array(
                        content_parts,
                    ))
                    .build()?
            } else {
                // 没有图片，只有文本
                ChatCompletionRequestUserMessageArgs::default()
                    .content(user_message)
                    .build()?
            }
        } else {
            // 没有图片参数，只有文本
            ChatCompletionRequestUserMessageArgs::default()
                .content(user_message)
                .build()?
        };

        messages.push(ChatCompletionRequestMessage::User(user_msg));

        // 构建请求
        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.model_name)
            .messages(messages)
            .temperature(0.3)
            .max_tokens(1024u32)
            .build()?;

        // 调用 API
        let response = self.client.chat().create(request).await.map_err(|e| {
            warn!("LLM API 调用失败: {}", e);
            anyhow::anyhow!("LLM API 调用失败: {}", e)
        })?;

        debug!("LLM API 调用成功");

        // 提取响应内容
        let content = response
            .choices
            .first()
            .and_then(|choice| choice.message.content.clone())
            .ok_or_else(|| anyhow::anyhow!("LLM 返回内容为空"))?;

        Ok(content.trim().to_string())
    }

    /// 从搜索结果中找到最佳匹配
    ///
    /// 这个函数基于 `send_to_llm` 实现，专门用于题目匹配场景。
    ///
    /// # 参数
    /// - `search_results`: 搜索结果列表
    /// - `stem`: 题干内容
    /// - `imgs`: 可选的图片 URL 列表
    ///
    /// # 返回
    /// 返回最佳匹配的索引（0-based）
    ///
    /// # 示例
    /// ```no_run
    /// # use add_question_submit::services::LlmService;
    /// # use add_question_submit::models::question::SearchResult;
    /// # async fn example(service: &LlmService) -> anyhow::Result<()> {
    /// let search_results = vec![/* ... */];
    /// let stem = "1+1等于几？";
    /// let index = service.find_best_match(&search_results, stem, None).await?;
    /// println!("最佳匹配索引: {}", index);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn find_best_match(
        &self,
        search_results: &[SearchResult],
        stem: &str,
        imgs: Option<&[String]>,
    ) -> Result<usize> {
        if search_results.is_empty() {
            anyhow::bail!("搜索结果为空，无法进行匹配");
        }

        debug!(
            "开始 LLM 匹配，候选数量: {}, 模型: {}",
            search_results.len(),
            self.model_name
        );

        // 构建专门用于题目匹配的 prompt
        let (user_message, system_message) = self.build_match_messages(search_results, stem, imgs);

        // 调用通用的 LLM 接口（传入图片参数）
        let response = self
            .send_to_llm(&user_message, Some(&system_message), imgs)
            .await?;

        // 解析响应，提取索引
        let selected_index = self.parse_match_response(&response, search_results.len())?;

        debug!("LLM 选择索引: {} (0-based)", selected_index);

        Ok(selected_index)
    }

    /// 构建用于题目匹配的消息
    ///
    /// 返回 (user_message, system_message)
    fn build_match_messages(
        &self,
        search_results: &[SearchResult],
        stem: &str,
        imgs: Option<&[String]>,
    ) -> (String, String) {
        // 构建系统消息
        let system_message = "你是一个专业的题目匹配助手，擅长通过文字内容和图片信息判断两个题目是否是同一个题目。\
                             你需要综合考虑题目的文字和图片来判断匹配度。\
                             当题目包含图片时，图片URL已包含在提示词中，你需要根据图片信息来判断是否匹配。".to_string();

        // 构建目标题目的图片信息
        let toml_img_info = if let Some(imgs) = imgs {
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
        };

        // 构建候选题目列表（JSON 格式）
        let mut candidates = Vec::new();
        for (idx, result) in search_results.iter().enumerate() {
            let mut candidate = serde_json::json!({
                "index": idx,
                "content": &result.question_content,
                "similarity": result.xkw_question_similarity,
            });

            // 添加图片信息
            if let Some(img_urls) = &result.img_urls {
                candidate["image_count"] = serde_json::json!(img_urls.len());
                candidate["image_urls"] = serde_json::json!(img_urls);
            }

            candidates.push(candidate);
        }

        let candidates_json = serde_json::to_string_pretty(&candidates).unwrap_or_default();

        // 构建候选题目图片信息摘要
        let mut candidate_img_info = String::new();
        for (idx, result) in search_results.iter().enumerate() {
            if let Some(img_urls) = &result.img_urls {
                if !img_urls.is_empty() {
                    candidate_img_info.push_str(&format!(
                        "  候选题目 {}: 包含 {} 张图片\n",
                        idx,
                        img_urls.len()
                    ));
                }
            }
        }
        if candidate_img_info.is_empty() {
            candidate_img_info = "  所有候选题目均无图片\n".to_string();
        }

        // 构建用户消息
        let user_message = format!(
            r#"你需要判断目标题目和候选题目列表中哪个是同一个题目。

【重要说明】
- 目标题目（来自TOML文件）和候选题目（来自题库搜索结果）都可能有图片
- 图片以URL形式提供，你需要根据URL的文件名和路径判断图片是否相同
- 如果两个题目的图片URL相同或非常相似（例如只有尺寸参数不同），说明是同一张图片
- 判断标准：是否是同一个题目，而不仅仅是相似

目标题目（来自TOML文件）：
  题干内容：{}
  图片信息：{}

候选题目列表（来自题库搜索结果）：
{}

候选题目图片信息：
{}

【判断步骤】
1. 首先比较题目的文字内容是否相同或高度一致
2. 如果目标题目有图片，检查候选题目是否也有相同的图片URL（或URL只有参数差异）
3. 如果候选题目有图片，检查目标题目是否也有相同的图片URL
4. 综合文字内容和图片URL，判断哪个候选题目与目标题目是同一个题目
5. 图片URL相同的权重很高，优先选择文字和图片URL都匹配的题目

只返回该题目的index数字（0、1、2等），不要返回任何其他内容。"#,
            stem, toml_img_info, candidates_json, candidate_img_info
        );

        (user_message, system_message)
    }

    /// 解析题目匹配的 LLM 响应
    ///
    /// 从 LLM 的响应中提取题目索引
    fn parse_match_response(&self, response: &str, max_index: usize) -> Result<usize> {
        let response = response.trim();

        // 尝试直接解析数字
        if let Ok(index) = response.parse::<usize>() {
            if index < max_index {
                return Ok(index);
            } else {
                warn!(
                    "LLM 返回的索引 {} 超出范围 [0, {}]，使用默认值 0",
                    index,
                    max_index - 1
                );
                return Ok(0);
            }
        }

        // 尝试从文本中提取数字
        for word in response.split_whitespace() {
            let cleaned = word.trim_matches(|c: char| !c.is_numeric());
            if let Ok(index) = cleaned.parse::<usize>() {
                if index < max_index {
                    debug!("从响应 '{}' 中提取到索引: {}", response, index);
                    return Ok(index);
                }
            }
        }

        // 如果无法解析，返回默认值 0
        warn!("无法解析 LLM 响应: '{}', 默认选择第一个候选", response);
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 创建测试用的 LlmService
    fn create_test_service() -> LlmService {
        let config = OpenAIConfig::new()
            .with_api_key("26e96c4d312e48feacbd78b7c42bd71e")
            .with_api_base("http://menshen.xdf.cn/v1");

        let client = Client::with_config(config);

        LlmService {
            client,
            model_name: "doubao-seed-1.6".to_string(),
        }
    }

    /// 创建测试用的搜索结果

    #[test]
    fn test_parse_match_response_direct_number() {
        let service = create_test_service();

        // 测试直接返回数字
        assert_eq!(service.parse_match_response("0", 3).unwrap(), 0);
        assert_eq!(service.parse_match_response("1", 3).unwrap(), 1);
        assert_eq!(service.parse_match_response("2", 3).unwrap(), 2);
    }

    #[test]
    fn test_parse_match_response_with_text() {
        let service = create_test_service();

        // 测试包含文字的响应
        assert_eq!(service.parse_match_response("我选择 0", 3).unwrap(), 0);
        assert_eq!(service.parse_match_response("答案是 1", 3).unwrap(), 1);
        assert_eq!(service.parse_match_response("2 是最匹配的", 3).unwrap(), 2);
    }

    /// 测试通用 LLM 调用
    #[tokio::test]
    #[ignore]
    async fn test_send_to_llm_simple() {
        let _ = tracing_subscriber::fmt::try_init();

        let service = create_test_service();

        println!("\n========== 测试通用 LLM 调用 ==========");
        let user_message = "请描写一下这个图片中的内容";
        let system_message = Some("你是一个简洁的助手，回答要简短。");

        let result = service
            .send_to_llm(user_message, system_message, None)
            .await;

        match result {
            Ok(response) => {
                println!("\n========== LLM 响应 ==========");
                println!("{}", response);
                println!("==============================\n");
                println!("✅ 通用 LLM 调用成功！");
                assert!(!response.is_empty());
            }
            Err(e) => {
                println!("❌ LLM 调用失败: {}", e);
                panic!("测试失败: {}", e);
            }
        }
    }

    /// 测试 LLM API 连接性（带图片）
    #[tokio::test]
    #[ignore]
    async fn test_llm_api_connectivity() {
        let _ = tracing_subscriber::fmt::try_init();

        let service = create_test_service();

        let search_results = vec![
            SearchResult {
                question_content: "北京是中国的首都，位于华北平原。".to_string(),
                xkw_question_similarity: Some(0.95),
                img_urls: Some(vec![
                    "https://img.xkw.com/dksih/QBM/editorImg/2025/12/26/beijing.png".to_string(),
                ]),
            },
            SearchResult {
                question_content: "上海是中国的经济中心。".to_string(),
                xkw_question_similarity: Some(0.85),
                img_urls: None,
            },
        ];

        let stem = "中国的首都是哪里？";
        let target_imgs =
            vec!["https://img.xkw.com/dksih/QBM/editorImg/2025/12/26/china-map.png".to_string()];

        println!("\n========== 测试数据 ==========");
        println!("目标题目: {}", stem);
        println!("目标题目图片: {} 张", target_imgs.len());
        println!("==============================\n");

        let result = service
            .find_best_match(&search_results, stem, Some(&target_imgs))
            .await;

        match result {
            Ok(index) => {
                println!("\n========== 测试结果 ==========");
                println!("✅ LLM API 调用成功！");
                println!("LLM 选择的索引: {}", index);
                println!("选择的题目: {}", search_results[index].question_content);
                println!("==============================\n");
                assert!(index < search_results.len());
            }
            Err(e) => {
                println!("\n❌ LLM API 调用失败: {}", e);
                panic!("LLM API 测试失败: {}", e);
            }
        }
    }

    /// 测试 Vision API 图片理解能力
    ///
    /// 运行方式：
    /// ```bash
    /// cargo test test_vision_api -- --ignored --nocapture
    /// ```
    #[tokio::test]
    #[ignore]
    async fn test_vision_api() {
        let _ = tracing_subscriber::fmt::try_init();

        let service = create_test_service();

        println!("\n========== 测试 Vision API 图片理解 ==========");

        // 使用真实的公开图片 URL
        let image_urls = vec![
            "https://img.xkw.com/dksih/QBM/editorImg/2025/12/26/7b53a389-80ac-48a3-9833-98112c3bb0a7.png?resizew=151".to_string(),
        ];

        let user_message = "请详细描述这张图片中的内容，包括场景、颜色、物体等。";

        println!("用户消息: {}", user_message);
        println!("图片数量: {}", image_urls.len());
        for (i, url) in image_urls.iter().enumerate() {
            println!("  图片 {}: {}", i + 1, url);
        }
        println!("==========================================\n");

        let result = service
            .send_to_llm(
                user_message,
                Some("你是一个专业的图片分析助手。请仔细观察图片并给出详细描述。"),
                Some(&image_urls),
            )
            .await;

        match result {
            Ok(response) => {
                println!("\n========== LLM 响应 ==========");
                println!("{}", response);
                println!("==============================\n");
                println!("✅ Vision API 调用成功！");

                // 验证响应不为空且包含描述性内容
                assert!(!response.is_empty());
                assert!(response.len() > 50); // 应该有详细的描述
            }
            Err(e) => {
                println!("\n❌ Vision API 调用失败: {}", e);
                println!("注意：如果模型不支持 Vision API，此测试会失败");
                println!("请确保使用支持视觉功能的模型，如 gpt-4-vision-preview");
                panic!("Vision API 测试失败: {}", e);
            }
        }
    }

    /// 测试 Vision API 多图片处理
    #[tokio::test]
    #[ignore]
    async fn test_vision_api_multiple_images() {
        let _ = tracing_subscriber::fmt::try_init();

        let service = create_test_service();

        println!("\n========== 测试 Vision API 多图片处理 ==========");

        let image_urls = vec![
            "https://upload.wikimedia.org/wikipedia/commons/thumb/3/3a/Cat03.jpg/1200px-Cat03.jpg"
                .to_string(),
            "https://upload.wikimedia.org/wikipedia/commons/thumb/4/4d/Cat_November_2010-1a.jpg/1200px-Cat_November_2010-1a.jpg"
                .to_string(),
        ];

        let user_message = "请比较这两张图片的异同点。";

        println!("用户消息: {}", user_message);
        println!("图片数量: {}", image_urls.len());
        println!("==========================================\n");

        let result = service
            .send_to_llm(user_message, None, Some(&image_urls))
            .await;

        match result {
            Ok(response) => {
                println!("\n========== LLM 响应 ==========");
                println!("{}", response);
                println!("==============================\n");
                println!("✅ 多图片 Vision API 调用成功！");
                assert!(!response.is_empty());
            }
            Err(e) => {
                println!("\n❌ Vision API 调用失败: {}", e);
                panic!("多图片 Vision API 测试失败: {}", e);
            }
        }
    }
}
