mod api;
mod app;
mod browser;
mod config;
mod error;
mod logger;
mod models;
mod processing;

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
    let _app = App::initialize(config).await?.run().await?;

    Ok(())
}
