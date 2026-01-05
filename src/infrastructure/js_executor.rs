//! JS 执行器 - 基础设施层
//!
//! 持有唯一的 page 资源，只暴露"执行 JS"的能力

use anyhow::Result;
use chromiumoxide::Page;
use serde::de::DeserializeOwned;
use serde_json::Value as JsonValue;

/// JS 执行器
///
/// 职责：
/// - 持有唯一的 Page 资源
/// - 暴露 eval() 能力
/// - 不认识 Question / Paper
/// - 不处理业务流程
pub struct JsExecutor {
    page: Page,
}

impl JsExecutor {
    /// 创建新的 JS 执行器
    pub fn new(page: Page) -> Self {
        Self { page }
    }

    /// 获取 page 的引用（用于其他操作）
    pub fn page(&self) -> &Page {
        &self.page
    }

    /// 执行 JS 代码并返回 JSON 结果
    ///
    /// # 参数
    /// - `js_code`: 要执行的 JavaScript 代码
    ///
    /// # 返回
    /// 返回 JSON 值
    pub async fn eval(&self, js_code: impl Into<String>) -> Result<JsonValue> {
        let result = self.page.evaluate(js_code.into()).await?;
        let json_value = result.into_value()?;
        Ok(json_value)
    }

    /// 执行 JS 代码并反序列化为指定类型
    ///
    /// # 参数
    /// - `js_code`: 要执行的 JavaScript 代码
    ///
    /// # 返回
    /// 返回反序列化后的类型
    pub async fn eval_as<T: DeserializeOwned>(&self, js_code: impl Into<String>) -> Result<T> {
        let json_value = self.eval(js_code).await?;
        let typed_value = serde_json::from_value(json_value)?;
        Ok(typed_value)
    }
}
