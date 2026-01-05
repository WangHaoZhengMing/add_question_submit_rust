//! 警告写入服务 - 业务能力层
//!
//! 只负责"写 warn.txt"能力，不关心流程

use anyhow::Result;
use std::fs::OpenOptions;
use std::io::Write;
use tracing::debug;

/// 警告写入服务
///
/// 职责：
/// - 将无法处理的题目写入 warn.txt
/// - 只处理单个题目的警告
/// - 不出现 Vec<Question>
/// - 不关心流程顺序
pub struct WarnWriter {
    warn_file_path: String,
}

impl WarnWriter {
    /// 创建新的警告写入服务
    pub fn new() -> Self {
        Self {
            warn_file_path: "warn.txt".to_string(),
        }
    }

    /// 使用自定义文件路径创建
    pub fn with_path(path: impl Into<String>) -> Self {
        Self {
            warn_file_path: path.into(),
        }
    }

    /// 写入警告信息
    ///
    /// # 参数
    /// - `paper_id`: 试卷ID
    /// - `question_index`: 题目索引
    /// - `stem`: 题干内容
    ///
    /// # 返回
    /// 返回是否成功写入
    pub async fn write(
        &self,
        paper_id: &str,
        question_index: usize,
        stem: &str,
    ) -> Result<()> {
        debug!(
            "写入警告: 试卷 {} | 题目 {} | 题干长度: {}",
            paper_id,
            question_index,
            stem.len()
        );

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.warn_file_path)?;

        let warn_msg = format!(
            "试卷 {} | 题目 {} | 题干: {}\n",
            paper_id, question_index, stem
        );

        file.write_all(warn_msg.as_bytes())?;

        Ok(())
    }
}

impl Default for WarnWriter {
    fn default() -> Self {
        Self::new()
    }
}
