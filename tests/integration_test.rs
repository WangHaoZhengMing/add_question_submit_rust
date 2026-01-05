use add_question_submit::browser::connect_to_browser_and_page;
use add_question_submit::config::Config;
use add_question_submit::logger;
use add_question_submit::models::load_toml_to_question_page;
use add_question_submit::services::PaperService;
use std::path::Path;

#[tokio::test]
#[ignore] // 默认忽略，需要手动运行：cargo test -- --ignored
async fn test_add_single_paper() {
    // 初始化日志
    logger::init();

    // 加载配置
    let config = Config::from_env();

    // 连接浏览器
    let (_browser, page) =
        connect_to_browser_and_page(config.browser_debug_port, Some(&config.target_url), None)
            .await
            .expect("连接浏览器失败");

    // 加载 toml 文件
    // 注意：请根据实际情况修改文件路径
    let toml_path = Path::new(
        r"C:\Users\HallM\iCloudDrive\Desktop\rust_code\add_question_submit\output_toml\2025年云南省初中学业水平考试历史模拟试卷（3）.toml",
    );

    let question_page = load_toml_to_question_page(toml_path)
        .await
        .expect("加载 toml 文件失败");

    // 创建试卷处理服务
    let paper_service = PaperService::new(&config);

    // 处理试卷
    let result = paper_service
        .process_paper(&page, question_page, 1)
        .await
        .expect("处理试卷失败");

    assert!(result, "试卷处理应该成功");
}

#[tokio::test]
#[ignore]
async fn test_browser_connection() {
    // 初始化日志
    logger::init();

    // 加载配置
    let config = Config::from_env();

    // 测试浏览器连接
    let result =
        connect_to_browser_and_page(config.browser_debug_port, Some(&config.target_url), None)
            .await;

    assert!(result.is_ok(), "应该能够成功连接浏览器");
}

#[tokio::test]
#[ignore]
async fn test_load_toml_files() {
    // 初始化日志
    logger::init();

    // 加载配置
    let config = Config::from_env();

    // 测试加载所有 TOML 文件
    let result = add_question_submit::models::load_all_toml_files(&config.toml_folder).await;

    assert!(result.is_ok(), "应该能够加载 TOML 文件");

    let papers = result.unwrap();
    println!("找到 {} 个试卷", papers.len());
}
