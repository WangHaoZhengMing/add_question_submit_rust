/// 程序配置文件
#[derive(Clone, Debug)]
pub struct Config {
    /// 同时处理的试卷数量
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
    // --- LLM 配置 ---
    pub llm_api_key: String,
    pub llm_api_base_url: String,
    pub llm_model_name: String,
    // --- 题库 API 配置 ---
    pub tiku_api_base_url: String,
    pub tiku_token: String,
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
            llm_api_key: "26e96c4d312e48feacbd78b7c42bd71e".to_string(),
            llm_api_base_url: "http://menshen.xdf.cn/v1".to_string(),
            llm_model_name: "gemini-3.0-pro-preview".to_string(),
            tiku_api_base_url: "https://tps-tiku-api.staff.xdf.cn".to_string(),
            tiku_token: "732FD8402F95087CD934374135C46EE5".to_string(),
        }
    }
}

impl Config {
    pub fn from_env() -> Self {
        let default = Self::default();
        Self {
            max_concurrent_papers: std::env::var("MAX_CONCURRENT_PAPERS").ok().and_then(|v| v.parse().ok()).unwrap_or(default.max_concurrent_papers),
            browser_debug_port: std::env::var("BROWSER_DEBUG_PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(default.browser_debug_port),
            target_url: std::env::var("TARGET_URL").unwrap_or(default.target_url),
            toml_folder: std::env::var("TOML_FOLDER").unwrap_or(default.toml_folder),
            verbose_logging: std::env::var("VERBOSE_LOGGING").ok().and_then(|v| v.parse().ok()).unwrap_or(default.verbose_logging),
            output_log_file: std::env::var("OUTPUT_LOG_FILE").unwrap_or(default.output_log_file),
            llm_api_key: std::env::var("LLM_API_KEY").unwrap_or(default.llm_api_key),
            llm_api_base_url: std::env::var("LLM_API_BASE_URL").unwrap_or(default.llm_api_base_url),
            llm_model_name: std::env::var("LLM_MODEL_NAME").unwrap_or(default.llm_model_name),
            tiku_api_base_url: std::env::var("TIKU_API_BASE_URL").unwrap_or(default.tiku_api_base_url),
            tiku_token: std::env::var("TIKU_TOKEN").unwrap_or(default.tiku_token),
        }
    }
}
