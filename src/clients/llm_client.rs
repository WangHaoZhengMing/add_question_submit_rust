/// LLM API 客户端
///
/// 封装所有与 LLM API 相关的调用逻辑
use crate::config::Config;
use anyhow::Result;
use openai::chat::{ChatCompletion, ChatCompletionMessage, ChatCompletionMessageRole};
use openai::Credentials;
use tracing::{debug, warn};

/// LLM 客户端
pub struct LlmClient {
    api_key: String,
    api_base_url: String,
    model_name: String,
}

impl LlmClient {
    /// 创建新的 LLM 客户端
    pub fn new(config: &Config) -> Self {
        Self {
            api_key: config.llm_api_key.clone(),
            api_base_url: config.llm_api_base_url.clone(),
            model_name: config.llm_model_name.clone(),
        }
    }

    /// 创建自定义配置的 LLM 客户端
    pub fn with_model(config: &Config, model_name: impl Into<String>) -> Self {
        Self {
            api_key: config.llm_api_key.clone(),
            api_base_url: config.llm_api_base_url.clone(),
            model_name: model_name.into(),
        }
    }

    /// 发送聊天请求
    ///
    /// # 参数
    /// - `user_message`: 用户消息内容
    /// - `system_message`: 系统消息（可选）
    ///
    /// # 返回
    /// 返回 LLM 的响应内容
    pub async fn chat(&self, user_message: &str, system_message: Option<&str>) -> Result<String> {
        debug!("正在调用 LLM API，模型: {}", self.model_name);
        debug!("用户消息: {}", user_message);

        let credentials = Credentials::new(&self.api_key, &self.api_base_url);

        let mut messages = Vec::new();

        // 添加系统消息（如果提供）
        if let Some(system_msg) = system_message {
            messages.push(ChatCompletionMessage {
                role: ChatCompletionMessageRole::System,
                content: Some(system_msg.to_string()),
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

        let chat_completion = ChatCompletion::builder(&self.model_name, messages)
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

    /// 简单的聊天请求（不带系统消息）
    pub async fn simple_chat(&self, user_message: &str) -> Result<String> {
        self.chat(user_message, None).await
    }
}
