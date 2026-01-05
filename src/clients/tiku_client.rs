/// 题库 API 客户端
///
/// 封装所有与题库 API 相关的调用逻辑
use crate::config::Config;
use anyhow::{Context, Result};
use chromiumoxide::Page;
use serde_json::{json, Value};
use tracing::debug;

/// 题库 API 客户端
pub struct TikuClient {
    base_url: String,
    token: String,
}

impl TikuClient {
    /// 创建新的题库客户端
    pub fn new(config: &Config) -> Self {
        Self {
            base_url: config.tiku_api_base_url.clone(),
            token: config.tiku_token.clone(),
        }
    }

    /// 搜索题目
    ///
    /// # 参数
    /// - `page`: 浏览器页面对象
    /// - `stem`: 题干内容
    /// - `subject_code`: 科目代码
    ///
    /// # 返回
    /// 返回完整的搜索结果 JSON
    pub async fn search_question(
        &self,
        page: &Page,
        stem: &str,
        subject_code: &str,
    ) -> Result<Value> {
        let search_data = json!({
            "stage": "3",
            "subject": subject_code,
            "text": stem
        });

        let search_data_json = serde_json::to_string(&search_data)?;

        let script = format!(
            r#"
            (async () => {{
                try {{
                    const res = await fetch("{}/api/third/xkw/question/v2/text-search", {{
                        method: "POST",
                        headers: {{
                            "Content-Type": "application/json",
                            "Accept": "application/json, text/plain, */*"
                        }},
                        credentials: "include",
                        body: JSON.stringify({})
                    }});
                    const data = await res.json();
                    return data;
                }} catch (err) {{
                    console.error("搜索请求失败:", err);
                    return null;
                }}
            }})()
            "#,
            self.base_url, search_data_json
        );

        let result: Value = page
            .evaluate(script.as_str())
            .await?
            .into_value()
            .context("无法执行搜索脚本")?;

        Ok(result)
    }

    pub async fn search_question_k12(
        &self,
        page: &Page,
        stem: &str,
        subject_code: &str,
    ) -> Result<Value> {
        let search_data = json!({
            "stage": "3",
            "subject": subject_code,
            "text": stem
        });

        let search_data_json = serde_json::to_string(&search_data)?;

        let script = format!(
            r#"
        (async () => {{
            try {{
                const res = await fetch("{}/api/third/xkw/question/v2/text-search", {{
                    method: "POST",
                    headers: {{
                        "Content-Type": "application/json",
                        "Accept": "application/json, text/plain, */*"
                    }},
                    credentials: "include",
                    body: JSON.stringify({})
                }});
                const data = await res.json();
                return data;
            }} catch (err) {{
                console.error("搜索请求失败:", err);
                return null;
            }}
        }})()
        "#,
            self.base_url, search_data_json
        );

        let result: Value = page
            .evaluate(script.as_str())
            .await?
            .into_value()
            .context("无法执行搜索脚本")?;

        Ok(result)
    }
    /// 保存题目
    ///
    /// # 参数
    /// - `page`: 浏览器页面对象
    /// - `question_data`: 题目数据
    ///
    /// # 返回
    /// 返回保存结果
    pub async fn save_question(&self, page: &Page, question_data: &Value) -> Result<Value> {
        let question_json = serde_json::to_string(question_data)?;
        let script = self.build_api_script("question/new/save", &question_json);

        debug!("保存题目 Payload: {}", question_json);

        let result: Value = page.evaluate(script.as_str()).await?.into_value()?;

        debug!("保存题目结果: {}", result);

        Ok(result)
    }

    /// 保存标题
    ///
    /// # 参数
    /// - `page`: 浏览器页面对象
    /// - `page_id`: 试卷ID
    /// - `question_index`: 题目索引
    /// - `stem`: 标题内容
    ///
    /// # 返回
    /// 返回保存结果
    pub async fn save_title(
        &self,
        page: &Page,
        page_id: &str,
        question_index: usize,
        stem: &str,
    ) -> Result<Value> {
        let title_data = json!({
            "paperId": page_id,
            "inputType": 1,
            "questionIndex": question_index,
            "questionType": "2",
            "addFlag": 1,
            "sysCode": 1,
            "relationType": 0,
            "questionSource": 3,
            "structureType": "biaoti",
            "questionInfo": {
                "stem": format!("<span>{}</span>", stem)
            }
        });

        let title_json = serde_json::to_string(&title_data)?;
        let script = self.build_api_script("question/new/save", &title_json);

        debug!("保存标题 Payload: {}", title_json);

        let result: Value = page.evaluate(script.as_str()).await?.into_value()?;

        debug!("保存标题结果: {}", result);

        Ok(result)
    }

    /// 提交试卷
    ///
    /// # 参数
    /// - `page`: 浏览器页面对象
    /// - `page_id`: 试卷ID
    ///
    /// # 返回
    /// 返回提交结果
    pub async fn submit_paper(&self, page: &Page, page_id: &str) -> Result<Value> {
        let submit_data = json!({
            "paperId": page_id,
            "type": "NEW_INPUT"
        });

        let submit_json = serde_json::to_string(&submit_data)?;
        let script = self.build_api_script("paper/process/submit", &submit_json);

        let result: Value = page.evaluate(script.as_str()).await?.into_value()?;

        Ok(result)
    }

    /// 构建 API 调用脚本
    fn build_api_script(&self, endpoint: &str, json_data: &str) -> String {
        format!(
            r#"
            (async () => {{
                try {{
                    const res = await fetch("{}/{}", {{
                        method: "POST",
                        headers: {{
                            "Content-Type": "application/json",
                            "Accept": "application/json, text/plain, */*",
                            "tikutoken": "{}"
                        }},
                        credentials: "include",
                        body: JSON.stringify({})
                    }});
                    const data = await res.json();
                    return data;
                }} catch (err) {{
                    console.error("API请求失败:", err);
                    return null;
                }}
            }})()
            "#,
            self.base_url, endpoint, self.token, json_data
        )
    }

    /// 检查 API 响应是否成功
    pub fn is_success_response(result: &Value) -> bool {
        !result.is_null()
    }

    /// 检查是否是频率限制错误
    pub fn is_rate_limited(result: &Value) -> bool {
        if let Some(code) = result.get("code").and_then(|v| v.as_u64()) {
            if code == 600 {
                if let Some(msg) = result.get("message").and_then(|v| v.as_str()) {
                    return msg.contains("请求过于频繁");
                }
            }
        }
        false
    }

    /// 提取搜索结果数据
    pub fn extract_search_data(result: &Value) -> Option<&Vec<Value>> {
        result.get("data").and_then(|v| v.as_array())
    }
}
