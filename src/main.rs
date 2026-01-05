use add_question_submit::{App, Config};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    add_question_submit::logger::init();

    // 加载配置
    let config = Config::from_env();

    // 初始化并运行应用
    let _app = App::initialize(config).await?.run().await?;

    Ok(())
}
