/// 题目处理服务
///
/// 负责单个题目的处理逻辑，包括搜索、匹配、提交
use crate::clients::TikuClient;
use crate::config::Config;
use crate::models::question::Question;
use crate::services::matching_service::MatchingService;
use crate::services::search_service::SearchService;
use anyhow::{Context, Result};
use chromiumoxide::Page;
use serde_json::{json, Value};
use tracing::{info, warn};

/// 题目处理结果
#[derive(Debug)]
pub enum ProcessResult {
    /// 处理成功
    Success,
    /// 跳过（未找到匹配或其他原因）
    Skipped,
}

/// 题目处理服务
pub struct QuestionService {
    search_service: SearchService,
    matching_service: MatchingService,
    tiku_client: TikuClient,
    config: Config,
}

impl QuestionService {
    /// 创建新的题目处理服务
    pub fn new(config: &Config) -> Self {
        Self {
            search_service: SearchService::new(config),
            matching_service: MatchingService::new(config),
            tiku_client: TikuClient::new(config),
            config: config.clone(),
        }
    }

    /// 处理单个题目
    ///
    /// # 参数
    /// - `page`: 浏览器页面对象
    /// - `question`: 题目数据
    /// - `page_id`: 试卷ID
    /// - `subject`: 科目
    /// - `question_index`: 题目索引
    /// - `paper_index`: 试卷索引（用于日志）
    ///
    /// # 返回
    /// 返回处理结果
    pub async fn process_question(
        &self,
        page: &Page,
        question: &Question,
        page_id: &str,
        subject: &str,
        question_index: usize,
        paper_index: usize,
    ) -> Result<ProcessResult> {
        let stem = &question.stem;

        // 日志：显示题干预览
        self.log_stem(paper_index, stem);

        // 搜索题库
        info!("[试卷 {}] 🔍 正在题库中搜索...", paper_index);
        let (search_results, full_search_result) =
            self.search_service.search(page, stem, subject).await?;

        info!(
            "[试卷 {}] ✓ 搜索完成，找到 {} 个相似题目",
            paper_index,
            search_results.len()
        );

        if search_results.is_empty() {
            warn!("[试卷 {}] ⚠️ 未找到相似题目，跳过此题", paper_index);
            return Ok(ProcessResult::Skipped);
        }

        // 详细日志（如果启用）
        if self.config.verbose_logging {
            self.log_search_results(paper_index, &search_results);
        }

        // 选择最佳匹配
        let selected_index = self
            .matching_service
            .find_best_match(&search_results, stem, question.imgs.as_deref())
            .await?;

        info!(
            "[试卷 {}] ✓ 选择了第 {} 个结果 (相似度: {:?})",
            paper_index,
            selected_index + 1,
            search_results[selected_index].xkw_question_similarity
        );

        // 构建并提交题目
        let question_data =
            self.build_question_data(&full_search_result[selected_index], page_id, question_index);

        let success = self
            .submit_question(page, &question_data, paper_index)
            .await?;

        if success {
            Ok(ProcessResult::Success)
        } else {
            Ok(ProcessResult::Skipped)
        }
    }

    /// 处理标题
    ///
    /// # 参数
    /// - `page`: 浏览器页面对象
    /// - `page_id`: 试卷ID
    /// - `question`: 题目数据（标题）
    /// - `question_index`: 题目索引
    /// - `paper_index`: 试卷索引（用于日志）
    pub async fn process_title(
        &self,
        page: &Page,
        page_id: &str,
        question: &Question,
        question_index: usize,
        paper_index: usize,
    ) -> Result<()> {
        info!("[试卷 {}] 检测到标题，开始传入标题", paper_index);

        self.tiku_client
            .save_title(page, page_id, question_index, &question.stem)
            .await?;

        Ok(())
    }

    /// 构建题目数据
    fn build_question_data(
        &self,
        search_result: &Value,
        page_id: &str,
        question_index: usize,
    ) -> Value {
        let mut data = search_result.clone();
        data["addFlag"] = json!(1);
        data["paperId"] = json!(page_id);
        data["sysCode"] = json!(1);
        data["questionType"] = json!("1");
        data["relationType"] = json!(1);
        data["inputType"] = json!(1);
        data["questionIndex"] = json!(question_index);
        data
    }

    /// 提交题目
    async fn submit_question(
        &self,
        page: &Page,
        question_data: &Value,
        paper_index: usize,
    ) -> Result<bool> {
        info!("[试卷 {}] 📤 正在提交题目到题库...", paper_index);

        let result = self.tiku_client.save_question(page, question_data).await?;

        if TikuClient::is_success_response(&result) {
            info!("[试卷 {}] ✓ 题目提交成功", paper_index);
            Ok(true)
        } else {
            warn!("[试卷 {}] ⚠️ 题目提交可能失败", paper_index);
            Ok(false)
        }
    }

    /// 日志：显示题干
    fn log_stem(&self, paper_index: usize, stem: &str) {
        let stem_preview = if stem.chars().count() > 80 {
            stem.chars().take(80).collect::<String>() + "..."
        } else {
            stem.to_string()
        };
        info!("[试卷 {}] 题干: {}", paper_index, stem_preview);
    }

    /// 日志：显示搜索结果
    fn log_search_results(
        &self,
        paper_index: usize,
        search_results: &[crate::models::question::SearchResult],
    ) {
        for (i, sr) in search_results.iter().take(2).enumerate() {
            info!(
                "[试卷 {}]   {}. 相似度: {:?}",
                paper_index,
                i + 1,
                sr.xkw_question_similarity
            );
        }
    }
}
