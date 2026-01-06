//! LLM 服务 - 业务能力层
//!
//! 只负责"LLM 判断"能力，不关心流程
//!
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

    pub async fn find_best_match(
        &self,
        search_results: &[SearchResult],
        stem: &str,
        imgs: Option<&[String]>,
    ) -> Result<Option<usize>> {
        if search_results.is_empty() {
            anyhow::bail!("搜索结果为空，无法进行匹配");
        }

        // --- 准备 Prompt (只需要做一次) ---
        let (user_message, system_message, all_images) = 
            self.build_match_messages(search_results, stem, imgs);

        let imgs_slice = if all_images.is_empty() {
            None
        } else {
            Some(all_images.as_slice())
        };

        // --- 开始重试循环 (最多 3 次) ---
        let max_retries = 3;
        
        for attempt in 1..=max_retries {
            debug!("LLM 匹配尝试第 {}/{} 次", attempt, max_retries);

            // 1. 发送请求
            let response_result = self
                .send_to_llm(&user_message, Some(&system_message), imgs_slice)
                .await;

            // 2. 处理网络/API 错误
            let response = match response_result {
                Ok(res) => res,
                Err(e) => {
                    warn!("第 {} 次 LLM API 调用失败: {}", attempt, e);
                    if attempt == max_retries {
                        return Err(e); // 最后一次也失败，抛出错误
                    }
                    continue; // 还有机会，重试
                }
            };

            // 3. 解析结果
            // 注意：parse_match_response 内部现在返回 Ok(None) 表示没找到
            // 如果解析出奇怪的东西，它也是返回 Ok(None)
            // 你需要决定：是“解析失败/没找到”就重试，还是只针对“API报错”重试？
            
            // 假设策略是：只要能正常解析出 Some(i) 或 None，就算成功，不重试。
            // 如果解析逻辑觉得 LLM 在胡言乱语（比如返回了空字符串），可以视为错误进行重试。
            
            match self.parse_match_response(&response, search_results.len()) {
                Ok(result) => {
                    // 成功拿到结果 (可能是 Some(index) 也可能是 None)
                    if let Some(idx) = result {
                        debug!("LLM 成功匹配到索引: {}", idx);
                    } else {
                        debug!("LLM 认为没有匹配项 (None)");
                    }
                    return Ok(result); 
                }
                Err(e) => {
                    // 解析过程报错了（比如格式严重错误）
                    warn!("第 {} 次解析 LLM 响应失败: {}", attempt, e);
                    if attempt == max_retries {
                        // 如果 3 次都解析不了，那也没办法了，返回 None 吧，别崩了
                        warn!("重试耗尽，无法解析响应，默认返回 None");
                        return Ok(None); 
                    }
                    // 继续重试
                }
            }
        }

        Ok(None) // 理论上走不到这里
    }


    /// 构建用于题目匹配的消息
    ///
    /// 返回 (user_message, system_message)
    /// 构建消息，同时整理所有需要发送的图片
    fn build_match_messages(
        &self,
        search_results: &[SearchResult],
        stem: &str,
        target_imgs: Option<&[String]>,
    ) -> (String, String, Vec<String>) {
        // 返回值增加了一个 Vec<String>

        let mut all_images_to_send = Vec::new();

        // --- 1. 处理目标题目图片 ---
        let target_img_desc = if let Some(imgs) = target_imgs {
            if !imgs.is_empty() {
                // 将目标图片加入总列表
                all_images_to_send.extend_from_slice(imgs);
                // 记录目标图片在总列表中的索引范围（从 1 开始数，方便 LLM 理解）
                format!(
                    "请参考前 {} 张图片 (Image 1 - Image {})",
                    imgs.len(),
                    imgs.len()
                )
            } else {
                "无图片".to_string()
            }
        } else {
            "无图片".to_string()
        };

        // 记录目标图片占用了多少个位置，后续候选图片的索引要在此基础上累加
        let mut current_img_index = all_images_to_send.len();

        // --- 2. 处理候选题目列表（构建 JSON） ---
        let candidates: Vec<serde_json::Value> = search_results
            .iter()
            .enumerate()
            .map(|(idx, result)| {
                let mut candidate = serde_json::json!({
                    "index": idx,
                    "content": &result.question_content,
                    "similarity": result.xkw_question_similarity,
                });

                // 处理该候选题目的图片
                if let Some(c_imgs) = &result.img_urls {
                    if !c_imgs.is_empty() {
                        // 将该候选题目的图片加入总列表
                        all_images_to_send.extend_from_slice(c_imgs);

                        // 计算该候选图片在总流中的范围
                        let start = current_img_index + 1;
                        let end = current_img_index + c_imgs.len();
                        current_img_index += c_imgs.len(); // 更新游标

                        // 在 JSON 里直接告诉 LLM 去看哪几张图
                        if start == end {
                            candidate["image_ref"] =
                                serde_json::json!(format!("对应 Image {}", start));
                        } else {
                            candidate["image_ref"] =
                                serde_json::json!(format!("对应 Image {} 到 Image {}", start, end));
                        }
                    } else {
                        candidate["image_ref"] = serde_json::json!("无图片");
                    }
                } else {
                    candidate["image_ref"] = serde_json::json!("无图片");
                }

                candidate
            })
            .collect();

        let candidates_json = serde_json::to_string_pretty(&candidates).unwrap_or_default();

        // --- 3. 构建 Prompt ---
        let system_message = "你是一个试题匹配专家。我将一次性发送多张图片给你。\
                             你需要根据 Prompt 中的指示，将不同的图片对应到【目标题目】或【候选题目】上。\
                             请综合【文字内容】和【图片视觉内容】判断是否是同一题。".to_string();

        let user_message = format!(
            r#"请判断【目标题目】与【候选列表】中的哪一项是同一个题目。
    
            【图片索引说明】
            我一共发送了 {} 张图片。请务必严格按照下方的指示查看对应的图片。
            
            【目标题目】
            文字：{}
            图片：{}
            
            【候选题目列表】
            {}
            
            【判断逻辑】
            1. **视觉比对**：请查看候选题目对应的 `image_ref` 图片，并与目标题目的图片进行视觉对比。
            2. **文字比对**：对比题干文字。
            3. **综合判断**：如果文字一致且图片视觉内容一致（允许清晰度、水印差异），则是同一题。
            
            【输出要求】
            - 找到匹配项：仅返回 index 数字（如 0）。
            - 未找到匹配项：返回 None。
            - 只输出数字。"#,
            all_images_to_send.len(),
            stem,
            target_img_desc,
            candidates_json
        );

        (user_message, system_message, all_images_to_send)
    }

    /// 解析题目匹配的 LLM 响应
    ///
    /// 从 LLM 的响应中提取题目索引
    fn parse_match_response(&self, response: &str, max_index: usize) -> Result<Option<usize>> {
        let response = response.trim();

        // 1. 优先检查 "None" (忽略大小写)
        // 即使 Prompt 只要 None，LLM 有时也会输出 "None." 或 "none"
        if response.eq_ignore_ascii_case("none") || response.contains("None") {
            debug!("LLM 返回 None，表示未找到匹配项");
            return Ok(None);
        }

        // 2. 尝试直接解析纯数字
        if let Ok(index) = response.parse::<usize>() {
            if index < max_index {
                return Ok(Some(index));
            } else {
                warn!(
                    "LLM 返回的索引 {} 超出范围 [0, {})，视为无匹配",
                    index, max_index
                );
                return Ok(None);
            }
        }

        // 3. 尝试从文本中提取数字 (容错处理，防止 LLM 输出 "是第 0 个" 这种话)
        // 只要找到第一个合法的、在范围内的数字，就认为是它
        for word in response.split(|c: char| !c.is_numeric()) {
            if word.is_empty() { continue; }
            
            if let Ok(index) = word.parse::<usize>() {
                if index < max_index {
                    debug!("从响应 '{}' 中提取到索引: {}", response, index);
                    return Ok(Some(index));
                }
            }
        }

        // 4. 既不是 None，也提取不到有效数字
        // 这种情况通常是 LLM 回复了 "没找到"、"不匹配" 等中文，或者胡言乱语
        warn!("无法解析 LLM 响应: '{}', 视为无匹配", response);
        Ok(None)
    }

}
#[cfg(test)]
mod tests {
    use crate::utils;

    use super::*;

    /// 创建测试用的 LlmService
    fn create_test_service() -> LlmService {
        let config = OpenAIConfig::new()
            .with_api_key("26e96c4d312e48feacbd78b7c42bd71e")
            .with_api_base("http://menshen.xdf.cn/v1");

        let client = Client::with_config(config);

        LlmService {
            client,
            model_name: "gemini-3.0-pro-preview".to_string(),
        }
    }

    /// 测试通用 LLM 调用
    #[tokio::test]
    #[ignore]
    async fn test_send_to_llm_simple() {
        let _ = tracing_subscriber::fmt::try_init();

        let service = create_test_service();

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

    /// 测试 Vision API 图片理解能力
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
            "https://img.xkw.com/dksih/QBM/editorImg/2025/9/19/91e4fa99-85ae-4778-b769-0e1ed7d04ec6.png?resizew=72"
                .to_string(),
            "https://img.xkw.com/dksih/QBM/editorImg/2025/9/19/cd4ca712-4e58-4f15-91b7-e8f8691b6767.jpg?resizew=168"
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

    /// 测试 LLM API 连接性（带图片）
    #[tokio::test]
    async fn test_llm_api_match() {
        utils::logger::init_test_log();

        let service = create_test_service();

        let search_results = vec![
            SearchResult {
                question_content: "下列有关宋与辽、西夏、金政权并立的示意图，不正确的是（     ）".to_string(),
                xkw_question_similarity: Some(0.95),
                img_urls: Some(vec![
                    String::from("https://k12static.xdf.cn/k12/xkw/1765604022831/ea0764be559e4ceb92328b02d0b874a8.png"),
                    String::from("https://k12static.xdf.cn/k12/xkw/1765604022961/efaeadb21fb7491e8d79f1cd5b0cfd0b.png"),
                    String::from("https://k12static.xdf.cn/k12/xkw/1765604023110/038741fdd9004be7b6705902ef7720bf.png"),
                    String::from("https://k12static.xdf.cn/k12/xkw/1765604023265/5736c0da12284ebd87493436b2183e27.png")
                ]),
            },
            SearchResult {
                question_content: "下列有关宋与辽、西夏、金政权并立的示意图不正确的是（     ）

                北宋                 北宋                    南宋            南宋".to_string(),
                xkw_question_similarity: Some(0.85),
                img_urls: None,
            },
        ];

        let stem = "下列有关宋与辽、西夏、金政权并立的示意图，不正确的是（     ）";
        let target_imgs =
            vec!["https://k12static.xdf.cn/k12/xkw/1750294449260/01fdf398-c731-44b9-9ec7-183b13a892bb.png".to_string(),
                String::from("https://k12static.xdf.cn/k12/xkw/1750294449444/5f63c6c7-a6fd-47c5-b1c4-65e93055886b.png"),
                String::from("https://k12static.xdf.cn/k12/xkw/1750294449587/cbb524c1-c2be-4c05-a978-a1cc411d5191.png"),
                String::from("https://k12static.xdf.cn/k12/xkw/1750294449749/9c661e1b-2b5e-4949-912c-4b1d56a18624.png")

            ];

        let _result = service
            .find_best_match(&search_results, stem, Some(&target_imgs))
            .await;

        // match Some(index) {
        //     Ok(index) => {
        //         println!("LLM 选择的索引: {}", index);
        //         println!("选择的题目: {}", search_results[index].question_content);
        //         println!("==============================\n");
        //         assert!(index < search_results.len());
        //     }
        //     Err(e) => {
        //         println!("\n❌ LLM API 调用失败: {}", e);
        //         panic!("LLM API 测试失败: {}", e);
        //     }
        // }
    }
}
