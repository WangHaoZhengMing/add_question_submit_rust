mod app;
mod ask_llm;
mod browser;
mod config;
mod grade;
mod logger;
mod paper_processor;
mod search_bank;
mod subject;
mod model;

use anyhow::Result;
use app::App;
use config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    logger::init();

    // 加载配置
    let config = Config::from_env();

    // 初始化并运行应用
    let app = App::initialize(config).await?;
    app.run().await?;

    Ok(())
}



#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::browser::connect_to_browser_and_page;
    use crate::config::Config;
    use crate::logger;
    use crate::model::toml_loader::load_toml_to_question_page;
    use crate::paper_processor::process_single_paper;

    #[tokio::test]
    #[ignore] // 默认忽略，需要手动运行：cargo test -- --ignored
    async fn test_add_single_paper() {
        // 初始化日志
        logger::init();

        // 加载配置
        let config = Config::from_env();

        // 连接浏览器
        let (_browser, page) = connect_to_browser_and_page(
            config.browser_debug_port,
            Some(&config.target_url),
            None,
        )
        .await
        .expect("连接浏览器失败");

        // 加载 toml 文件
        // 注意：请根据实际情况修改文件路径
        let toml_path = Path::new(r"C:\Users\HallM\iCloudDrive\Desktop\rust_code\add_question_submit\output_toml\2025年5月陕西省榆林市榆阳区九年级下学期历史中考模拟练习题（二）.toml");
        
        let question_page: crate::model::model::QuestionPage = load_toml_to_question_page(toml_path)
            .await
            .expect("加载 toml 文件失败");

        // 处理试卷
        let result = process_single_paper(&page, question_page, 1, &config)
            .await
            .expect("处理试卷失败");

        assert!(result, "试卷处理应该成功");
    }
}