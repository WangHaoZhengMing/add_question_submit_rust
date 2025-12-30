use openai::chat::{ChatCompletion, ChatCompletionMessage, ChatCompletionMessageRole};
use openai::Credentials;
use tracing::{debug, info, warn};

use crate::model::model::{SearchResult, SearchResultForLlm};

// LLM 配置
const API_KEY: &str = "26e96c4d312e48feacbd78b7c42bd71e";
const API_BASE_URL: &str = "http://menshen.xdf.cn/v1";
const MODEL_NAME: &str = "gemini-3.0-pro-preview"; // 可以根据需要修改模型名称

/// LLM 请求配置
pub struct LlmConfig {
    /// API 密钥
    pub api_key: Option<String>,
    /// API 基础 URL
    pub api_base_url: Option<String>,
    /// 模型名称
    pub model_name: Option<String>,
    /// 系统消息
    pub system_message: Option<String>,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            api_base_url: None,
            model_name: None,
            system_message: None,
        }
    }
}

/// 通用的 LLM 调用函数（使用默认配置）
/// 
/// # 参数
/// - `user_message`: 用户消息内容
/// 
/// # 返回
/// 返回 LLM 的响应内容字符串
/// 
/// # 示例
/// ```no_run
/// use crate::ask_llm::ask_llm;
/// 
/// # async fn example() -> anyhow::Result<()> {
/// let response = ask_llm("你好，请介绍一下你自己").await?;
/// println!("{}", response);
/// # Ok(())
/// # }
/// ```
pub async fn ask_llm(user_message: &str) -> anyhow::Result<String> {
    ask_llm_with_config(user_message, None).await
}

/// 带自定义配置的 LLM 调用函数
/// 
/// # 参数
/// - `user_message`: 用户消息内容
/// - `config`: LLM 配置（可选，可以直接传 `LlmConfig` 或不传使用默认配置）
/// 
/// # 返回
/// 返回 LLM 的响应内容字符串
/// 
/// # 示例
/// ```no_run
/// use crate::ask_llm::{ask_llm_with_config, LlmConfig};
/// 
/// # async fn example() -> anyhow::Result<()> {
/// // 使用自定义配置
/// let config = LlmConfig {
///     system_message: Some("你是一个专业的助手".to_string()),
///     ..Default::default()
/// };
/// let response = ask_llm_with_config("你的问题", config).await?;
/// # Ok(())
/// # }
/// ```
pub async fn ask_llm_with_config(
    user_message: &str,
    config: impl Into<Option<LlmConfig>>,
) -> anyhow::Result<String> {
    let config = config.into().unwrap_or_default();
    
    let api_key = config.api_key.as_deref().unwrap_or(API_KEY);
    let api_base_url = config.api_base_url.as_deref().unwrap_or(API_BASE_URL);
    let model_name = config.model_name.as_deref().unwrap_or(MODEL_NAME);
    
    debug!("正在调用 LLM API，模型: {}", model_name);
    debug!("用户消息: {}", user_message);
    
    let credentials = Credentials::new(api_key, api_base_url);
    
    let mut messages = Vec::new();
    
    // 添加系统消息（如果提供）
    if let Some(system_msg) = config.system_message {
        messages.push(ChatCompletionMessage {
            role: ChatCompletionMessageRole::System,
            content: Some(system_msg),
            name: None,
            function_call: None,
            tool_call_id: None,
            tool_calls: None,
        });
    }
    
    // 添加用户消息
    messages.push(ChatCompletionMessage {
        role: ChatCompletionMessageRole::User,
        content: Some(user_message.to_string()),
        name: None,
        function_call: None,
        tool_call_id: None,
        tool_calls: None,
    });
    
    let chat_completion = ChatCompletion::builder(model_name, messages)
        .credentials(credentials)
        .create()
        .await
        .map_err(|e| {
            warn!("LLM API 调用失败: {}", e);
            anyhow::anyhow!("LLM API 调用失败: {}", e)
        })?;
    
    debug!("LLM API 调用成功");
    
    let returned_message = chat_completion
        .choices
        .first()
        .ok_or_else(|| anyhow::anyhow!("LLM 返回结果为空"))?
        .message
        .clone();
    
    let content = returned_message
        .content
        .ok_or_else(|| anyhow::anyhow!("LLM 返回内容为空"))?;
    
    Ok(content.trim().to_string())
}


/// 使用LLM判断哪个搜索结果与给定题干最相似
/// 
/// # 参数
/// - `search_results`: 搜索结果列表
/// - `stem`: 待比较的题干
/// - `imgs`: 题目的图片URL列表（可选），用于视觉理解题目内容
/// 
/// # 返回
/// 返回最相似题目的索引（0-based）
pub async fn ask_llm_for_which_index(
    search_results: &[SearchResult],
    stem: &str,
    imgs: Option<&[String]>,
) -> anyhow::Result<usize> {
    if search_results.is_empty() {
        anyhow::bail!("搜索结果列表不能为空");
    }
    
    // 构建搜索结果JSON，包含图片信息
    let results_for_llm: Vec<SearchResultForLlm> = search_results
        .iter()
        .enumerate()
        .map(|(idx, sr)| SearchResultForLlm::from((idx, sr)))
        .collect();
    
    let results_json = serde_json::to_string_pretty(&results_for_llm)?;
    
    // 构建图片信息说明（包含图片URL）
    let toml_img_info = if let Some(imgs) = imgs {
        if imgs.is_empty() {
            "无图片".to_string()
        } else {
            let img_list: Vec<String> = imgs.iter().enumerate().map(|(i, url)| {
                format!("    图片 {}: {}", i + 1, url)
            }).collect();
            format!("包含 {} 张图片：\n{}", imgs.len(), img_list.join("\n"))
        }
    } else {
        "无图片".to_string()
    };
    
    // 构建候选题目图片信息（包含图片URL）
    let mut candidate_img_info = String::new();
    for (idx, sr) in search_results.iter().enumerate() {
        if let Some(img_urls) = &sr.img_urls {
            if !img_urls.is_empty() {
                candidate_img_info.push_str(&format!(
                    "  候选题目 {}: 包含 {} 张图片\n",
                    idx, img_urls.len()
                ));
                for (i, url) in img_urls.iter().enumerate() {
                    candidate_img_info.push_str(&format!(
                        "    图片 {}: {}\n",
                        i + 1, url
                    ));
                }
            }
        }
    }
    if candidate_img_info.is_empty() {
        candidate_img_info = "  所有候选题目均无图片\n".to_string();
    }
    
    // 构建提示词，明确说明要对比两个题目（包括图片）
    let prompt = format!(
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
    );
    
    let system_message = "你是一个专业的题目匹配助手，擅长通过文字内容和图片内容判断两个题目是否是同一个题目。你需要综合考虑题目的文字和图片来判断匹配度。当题目包含图片时，图片URL已包含在提示词中，你需要根据图片URL来判断图片内容是否相同或相似。";
    
    // 调用LLM
    let api_key = API_KEY;
    let api_base_url = API_BASE_URL;
    let model_name = "doubao-seed-1.6";
    
    debug!("正在调用 LLM API，模型: {}", model_name);
    
    let credentials = Credentials::new(api_key, api_base_url);
    
    let mut messages = Vec::new();
    
    // 添加系统消息
    messages.push(ChatCompletionMessage {
        role: ChatCompletionMessageRole::System,
        content: Some(system_message.to_string()),
        name: None,
        function_call: None,
        tool_call_id: None,
        tool_calls: None,
    });
    
    // 添加用户消息
    // 注意：由于 openai crate 的 ChatCompletionMessage.content 是 Option<String>，
    // 我们暂时只发送文本 prompt（已包含图片URL信息）
    // 如果 API 支持，可以在 prompt 中说明图片已包含在消息中
    let user_message = ChatCompletionMessage {
        role: ChatCompletionMessageRole::User,
        content: Some(prompt.clone()),
        name: None,
        function_call: None,
        tool_call_id: None,
        tool_calls: None,
    };
    
    messages.push(user_message);
    
    let chat_completion = ChatCompletion::builder(model_name, messages)
        .credentials(credentials)
        .create()
        .await
        .map_err(|e| {
            warn!("LLM API 调用失败: {}", e);
            anyhow::anyhow!("LLM API 调用失败: {}", e)
        })?;
    
    debug!("LLM API 调用成功");
    
    let returned_message = chat_completion
        .choices
        .first()
        .ok_or_else(|| anyhow::anyhow!("LLM 返回结果为空"))?
        .message
        .clone();
    
    let response = returned_message
        .content
        .ok_or_else(|| anyhow::anyhow!("LLM 返回内容为空"))?;
    
    // 提取数字
    let index_str = response.trim();
    let index = index_str
        .parse::<usize>()
        .map_err(|e| anyhow::anyhow!("无法解析索引: {}, 错误: {}", index_str, e))?;
    
    // 验证索引范围
    if index >= search_results.len() {
        anyhow::bail!(
            "返回的索引 {} 超出范围 [0, {}]",
            index,
            search_results.len() - 1
        );
    }
    
    debug!("LLM选择了索引: {}", index);
    Ok(index)
}
