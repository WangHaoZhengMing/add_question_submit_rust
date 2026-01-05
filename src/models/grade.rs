/// 年级枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash,serde::Serialize, serde::Deserialize)]
pub enum Grade {
    /// 七年级/初一
    Grade7 = 161,
    /// 八年级/初二
    Grade8 = 162,
    /// 九年级/初三
    Grade9 = 163,
}

impl Grade {
    /// 获取年级代码
    pub fn code(self) -> i16 {
        self as i16
    }

    /// 获取标准名称
    pub fn name(self) -> &'static str {
        match self {
            Grade::Grade7 => "七年级",
            Grade::Grade8 => "八年级",
            Grade::Grade9 => "九年级",
        }
    }

    /// 从代码解析年级
    pub fn from_code(code: i16) -> Option<Self> {
        match code {
            161 => Some(Grade::Grade7),
            162 => Some(Grade::Grade8),
            163 => Some(Grade::Grade9),
            _ => None,
        }
    }

    /// 尝试从字符串解析年级（精确匹配）
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "七年级" | "初一" | "7年级" | "7" => Some(Grade::Grade7),
            "八年级" | "初二" | "8年级" | "8" => Some(Grade::Grade8),
            "九年级" | "初三" | "9年级" | "9" => Some(Grade::Grade9),
            _ => None,
        }
    }

    /// 智能查找年级（支持模糊匹配）
    pub fn find(s: &str) -> Option<Self> {
        // 先尝试精确匹配
        if let Some(grade) = Self::from_str(s) {
            return Some(grade);
        }

        // 模糊匹配
        let s_lower = s.to_lowercase();
        if s_lower.contains("七") || s_lower.contains("初一") || s.contains("7") {
            return Some(Grade::Grade7);
        }
        if s_lower.contains("八") || s_lower.contains("初二") || s.contains("8") {
            return Some(Grade::Grade8);
        }
        if s_lower.contains("九") || s_lower.contains("初三") || s.contains("9") {
            return Some(Grade::Grade9);
        }

        None
    }
}

impl std::fmt::Display for Grade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

// 为了保持向后兼容，提供函数接口
/// 获取年级code（向后兼容）
pub fn get_grade_code(grade_name: &str) -> Option<i16> {
    Grade::from_str(grade_name).map(|g| g.code())
}

/// 智能查找年级code（向后兼容）
pub fn find_grade_code(name: &str) -> Option<i16> {
    Grade::find(name).map(|g| g.code())
}
