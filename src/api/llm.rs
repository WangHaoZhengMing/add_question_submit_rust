//! LLM API 模块
//!
//! 负责所有与 LLM API 的交互，包括聊天和题目匹配

use anyhow::{Context, Result};
use openai::chat::{ChatCompletion, ChatCompletionMessage, ChatCompletionMessageRole};
use openai::Credentials;
use serde_json::Value;
use tracing::{debug, info};

/// 发送聊天请求
///
/// # 参数
/// - `prompt`: 用户提示词
/// - `system_message`: 系统消息（可选）
/// - `api_key`: API密钥
/// - `api_base`: API基础URL
/// - `model`: 模型名称
///
/// # 返回
/// 返回 LLM 的响应内容
pub async fn chat(
    prompt: &str,
    system_message: Option<&str>,
    api_key: &str,
    api_base: &str,
    model: &str,
) -> Result<String> {
    debug!("调用 LLM API，模型: {}", model);

    let credentials = Credentials::new(api_key, api_base);

    let mut messages = vec![];

    // 添加系统消息
    if let Some(sys_msg) = system_message {
        messages.push(ChatCompletionMessage {
            role: ChatCompletionMessageRole::System,
            content: Some(sys_msg.to_string()),
            name: None,
            function_call: None,
            tool_call_id: None,
            tool_calls: None,
        });
    }

    // 添加用户消息
    messages.push(ChatCompletionMessage {
        role: ChatCompletionMessageRole::User,
        content: Some(prompt.to_string()),
        name: None,
        function_call: None,
        tool_call_id: None,
        tool_calls: None,
    });

    let response = ChatCompletion::builder(model, messages)
        .credentials(credentials)
        .create()
        .await
        .context("LLM API 调用失败")?;

    let content = response
        .choices
        .first()
        .and_then(|c| c.message.content.as_ref())
        .context("LLM 返回内容为空")?;

    debug!("LLM API 调用成功");

    Ok(content.trim().to_string())
}

/// 找最佳匹配（包含快速匹配逻辑）
///
/// # 参数
/// - `search_results`: 搜索结果数组
/// - `stem`: 题干内容
/// - `imgs`: 题目图片URL列表（可选）
/// - `api_key`: API密钥
/// - `api_base`: API基础URL
///
/// # 返回
/// 返回最佳匹配的索引
pub async fn find_best_match(
    search_results: &[Value],
    stem: &str,
    imgs: Option<&[String]>,
    api_key: &str,
    api_base: &str,
) -> Result<usize> {
    if search_results.is_empty() {
        anyhow::bail!("搜索结果为空");
    }

    // 尝试快速匹配
    if let Some(index) = try_quick_match(search_results) {
        info!("⚡ 快速匹配成功，选择第 {} 个结果", index + 1);
        return Ok(index);
    }

    // 使用 LLM 判断
    info!("🤖 使用 LLM 判断最佳匹配...");

    let prompt = build_matching_prompt(search_results, stem, imgs);
    let system_message = "你是一个专业的题目匹配助手，擅长通过文字内容和图片内容判断两个题目是否是同一个题目。你需要综合考虑题目的文字和图片来判断匹配度。当题目包含图片时，图片URL已包含在提示词中，你需要根据图片URL来判断图片内容是否相同或相似。";

    let response = chat(
        &prompt,
        Some(system_message),
        api_key,
        api_base,
        "doubao-seed-1.6",
    )
    .await?;

    // 解析索引
    let index = response
        .trim()
        .parse::<usize>()
        .context("无法解析 LLM 返回的索引")?;

    // 验证索引范围
    if index >= search_results.len() {
        anyhow::bail!(
            "LLM 返回的索引 {} 超出范围 [0, {}]",
            index,
            search_results.len() - 1
        );
    }

    info!("✓ LLM 选择了第 {} 个结果", index + 1);

    Ok(index)
}

// ========== 辅助函数 ==========

/// 尝试快速匹配（基于相似度阈值）
///
/// 如果第一个结果的相似度明显高于第二个，则直接返回第一个
fn try_quick_match(search_results: &[Value]) -> Option<usize> {
    if search_results.len() < 2 {
        return None;
    }

    let s1 = search_results[0]
        .get("xkwQuestionSimilarity")
        .and_then(|v| v.as_f64());
    let s2 = search_results[1]
        .get("xkwQuestionSimilarity")
        .and_then(|v| v.as_f64());

    if let (Some(s1), Some(s2)) = (s1, s2) {
        // 自动判断是 0-1 还是 0-100 的尺度
        let is_scale_100 = s1 > 1.0 || s2 > 1.0;
        let threshold = if is_scale_100 { 90.0 } else { 0.85 };
        let diff_threshold = if is_scale_100 { 5.0 } else { 0.05 };

        // 如果第一个相似度大于阈值，且与第二个相差够大
        if s1 > threshold && (s1 - s2) > diff_threshold {
            return Some(0);
        }
    }

    None
}

/// 构建匹配提示词
fn build_matching_prompt(results: &[Value], stem: &str, imgs: Option<&[String]>) -> String {
    // 构建搜索结果JSON
    let results_json = serde_json::to_string_pretty(results).unwrap_or_default();

    // 构建图片信息
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

    // 构建候选题目图片信息
    let mut candidate_img_info = String::new();
    for (idx, result) in results.iter().enumerate() {
        if let Some(img_urls) = result.get("imgUrls").and_then(|v| v.as_array()) {
            if !img_urls.is_empty() {
                candidate_img_info.push_str(&format!(
                    "  候选题目 {}: 包含 {} 张图片\n",
                    idx,
                    img_urls.len()
                ));
                for (i, url) in img_urls.iter().enumerate() {
                    if let Some(url_str) = url.as_str() {
                        candidate_img_info.push_str(&format!("    图片 {}: {}\n", i + 1, url_str));
                    }
                }
            }
        }
    }
    if candidate_img_info.is_empty() {
        candidate_img_info = "  所有候选题目均无图片\n".to_string();
    }

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
