/// 程序配置文件
#[derive(Clone)]
pub struct Config {
    /// 同时处理的试卷数量（建议根据机器性能和网络情况调整）
    /// 建议值：1-5
    /// - 1: 顺序处理，最稳定
    /// - 3: 适中的并发，推荐值
    /// - 5: 较高并发，需要较好的机器性能
    pub max_concurrent_papers: usize,
    
    /// 浏览器调试端口
    pub browser_debug_port: u16,
    
    /// 目标URL
    pub target_url: String,
    
    /// TOML文件存放目录
    pub toml_folder: String,
    
    /// 是否显示详细日志
    pub verbose_logging: bool,
    
    /// 输出日志文件
    pub output_log_file: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_concurrent_papers: 100,
            browser_debug_port: 2001,
            target_url: "https://tk-lpzx.xdf.cn/#/paperEnterList".to_string(),
            toml_folder: "output_toml".to_string(),
            verbose_logging: false,
            output_log_file: "output.txt".to_string(),
        }
    }
}

impl Config {
    /// 从环境变量或使用默认值创建配置
    pub fn from_env() -> Self {
        Self {
            max_concurrent_papers: std::env::var("MAX_CONCURRENT_PAPERS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(100),
            browser_debug_port: std::env::var("BROWSER_DEBUG_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(2001),
            target_url: std::env::var("TARGET_URL")
                .unwrap_or_else(|_| "https://tk-lpzx.xdf.cn/#/paperEnterList".to_string()),
            toml_folder: std::env::var("TOML_FOLDER")
                .unwrap_or_else(|_| "output_toml".to_string()),
            verbose_logging: std::env::var("VERBOSE_LOGGING")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(false),
            output_log_file: std::env::var("OUTPUT_LOG_FILE")
                .unwrap_or_else(|_| "output.txt".to_string()),
        }
    }
}

