/// 科目枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Subject {
    /// 语文
    Chinese = 55,
    /// 数学
    Math = 54,
    /// 英语
    English = 53,
    /// 物理
    Physics = 56,
    /// 化学
    Chemistry = 57,
    /// 生物
    Biology = 58,
    /// 历史
    History = 61,
    /// 政治
    Politics = 60,
    /// 地理
    Geography = 59,
    /// 科学
    Science = 62,
}

impl Subject {
    /// 获取科目代码
    pub fn code(self) -> i16 {
        self as i16
    }

    /// 获取标准名称
    pub fn name(self) -> &'static str {
        match self {
            Subject::Chinese => "语文",
            Subject::Math => "数学",
            Subject::English => "英语",
            Subject::Physics => "物理",
            Subject::Chemistry => "化学",
            Subject::Biology => "生物",
            Subject::History => "历史",
            Subject::Politics => "政治",
            Subject::Geography => "地理",
            Subject::Science => "科学",
        }
    }

    /// 从代码解析科目
    pub fn from_code(code: i16) -> Option<Self> {
        match code {
            55 => Some(Subject::Chinese),
            54 => Some(Subject::Math),
            53 => Some(Subject::English),
            56 => Some(Subject::Physics),
            57 => Some(Subject::Chemistry),
            58 => Some(Subject::Biology),
            61 => Some(Subject::History),
            60 => Some(Subject::Politics),
            59 => Some(Subject::Geography),
            62 => Some(Subject::Science),
            _ => None,
        }
    }

    /// 尝试从字符串解析科目（精确匹配）
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "语文" | "语" => Some(Subject::Chinese),
            "数学" | "数" => Some(Subject::Math),
            "英语" | "英" => Some(Subject::English),
            "物理" | "物" => Some(Subject::Physics),
            "化学" | "化" => Some(Subject::Chemistry),
            "生物" | "生" => Some(Subject::Biology),
            "历史" | "历" => Some(Subject::History),
            "政治" | "政" => Some(Subject::Politics),
            "地理" | "地" => Some(Subject::Geography),
            "科学" | "科" => Some(Subject::Science),
            _ => None,
        }
    }

    /// 智能查找科目（支持模糊匹配）
    pub fn find(s: &str) -> Option<Self> {
        // 先尝试精确匹配
        if let Some(subject) = Self::from_str(s) {
            return Some(subject);
        }

        // 模糊匹配
        let s_lower = s.to_lowercase();
        if s_lower.contains("语文") || s_lower.contains("语") {
            return Some(Subject::Chinese);
        }
        if s_lower.contains("数学") || s_lower.contains("数") {
            return Some(Subject::Math);
        }
        if s_lower.contains("英语") || s_lower.contains("英") {
            return Some(Subject::English);
        }
        if s_lower.contains("物理") || s_lower.contains("物") {
            return Some(Subject::Physics);
        }
        if s_lower.contains("化学") || s_lower.contains("化") {
            return Some(Subject::Chemistry);
        }
        if s_lower.contains("生物") || s_lower.contains("生") {
            return Some(Subject::Biology);
        }
        if s_lower.contains("历史") || s_lower.contains("历") {
            return Some(Subject::History);
        }
        if s_lower.contains("政治") || s_lower.contains("政") {
            return Some(Subject::Politics);
        }
        if s_lower.contains("地理") || s_lower.contains("地") {
            return Some(Subject::Geography);
        }
        if s_lower.contains("科学") || s_lower.contains("科") {
            return Some(Subject::Science);
        }

        None
    }
}

impl std::fmt::Display for Subject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

