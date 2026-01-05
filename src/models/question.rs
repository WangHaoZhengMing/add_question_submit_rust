use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Question {
    pub origin: String,
    pub stem: String,

    #[serde(default)]
    pub origin_from_our_bank: Vec<String>,
    pub is_title: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imgs: Option<Vec<String>>,
}
impl Default for Question {
    fn default() -> Self {
        Self {
            origin: String::new(),
            stem: String::new(),
            origin_from_our_bank: Vec::new(),
            is_title: false,
            imgs: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionPage {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_for_cos: Option<String>,
    pub province: String,
    pub grade: String,
    #[serde(deserialize_with = "deserialize_year")]
    pub year: String,
    pub subject: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_id: Option<String>,
    pub stemlist: Vec<Question>,
    #[serde(skip_serializing, skip_deserializing)]
    pub file_path: Option<String>,
}

impl QuestionPage {
    /// 获取用于 COS 上传的文件名，如果不存在则使用原名称
    pub fn get_name_for_cos(&self) -> String {
        self.name_for_cos.clone().unwrap_or_else(|| self.name.clone())
    }
}

// Helper function to deserialize year as either string or integer
fn deserialize_year<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Visitor;
    use std::fmt;

    struct YearVisitor;

    impl<'de> Visitor<'de> for YearVisitor {
        type Value = String;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string or integer representing a year")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(value.to_string())
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(value.to_string())
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(value.to_string())
        }
    }

    deserializer.deserialize_any(YearVisitor)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperInfo {
    pub url: String,
    pub title: String,
}

/// 题库搜索结果数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    #[serde(rename = "questionContent")]
    pub question_content: String,
    
    #[serde(rename = "xkwQuestionSimilarity")]
    pub xkw_question_similarity: Option<f64>,

    pub img_urls: Option<Vec<String>>,
}

impl std::fmt::Display for SearchResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 截断题目内容以便显示（最多80个字符）
        let content_preview = if self.question_content.chars().count() > 80 {
            self.question_content
                .chars()
                .take(80)
                .collect::<String>()
                + "..."
        } else {
            self.question_content.clone()
        };

        // 如果有相似度，显示相似度信息
        if let Some(similarity) = self.xkw_question_similarity {
            write!(f, "{} [相似度: {:.2}]", content_preview, similarity)
        } else {
            write!(f, "{} [相似度: 未知]", content_preview)
        }
    }
}
/// 用于LLM判断的搜索结果项
#[derive(Debug, Clone, Serialize)]
pub struct SearchResultForLlm {
    pub index: usize,
    pub question_content: String,
    pub xkw_question_similarity: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub img_urls: Option<Vec<String>>,
}

impl From<(usize, &SearchResult)> for SearchResultForLlm {
    fn from((idx, sr): (usize, &SearchResult)) -> Self {
        Self {
            index: idx,
            question_content: sr.question_content.clone(),
            xkw_question_similarity: sr.xkw_question_similarity,
            img_urls: sr.img_urls.clone(),
        }
    }
}

/// 扩展QuestionPage以支持文件路径
impl QuestionPage {
    pub fn with_file_path(mut self, file_path: String) -> Self {
        self.file_path = Some(file_path);
        self
    }
}