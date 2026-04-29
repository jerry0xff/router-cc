//! 查询分类器 — 基于关键词的领域与复杂度检测
//!
//! 从用户最后一条消息分析：
//! - domain：任务类型 (coding | math | writing | translation | analysis | general)
//! - complexity：难度等级 (simple | medium | complex)
//!
//! 零外部依赖，亚毫秒级分类。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Domain {
    Coding,
    Math,
    Writing,
    Translation,
    Analysis,
    General,
}

impl Domain {
    pub fn as_str(&self) -> &'static str {
        match self {
            Domain::Coding => "coding",
            Domain::Math => "math",
            Domain::Writing => "writing",
            Domain::Translation => "translation",
            Domain::Analysis => "analysis",
            Domain::General => "general",
        }
    }
}

impl std::fmt::Display for Domain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Complexity {
    Simple,
    Medium,
    Complex,
}

impl Complexity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Complexity::Simple => "simple",
            Complexity::Medium => "medium",
            Complexity::Complex => "complex",
        }
    }

    /// 数值化复杂度，用于与 provider 配置的最低复杂度比较
    #[allow(dead_code)]
    pub fn level(&self) -> u8 {
        match self {
            Complexity::Simple => 0,
            Complexity::Medium => 1,
            Complexity::Complex => 2,
        }
    }
}

impl std::fmt::Display for Complexity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryProfile {
    pub domain: Domain,
    pub complexity: Complexity,
}

// ── 领域关键词 ──────────────────────────────────────────────────────────────

const CODE_KEYWORDS: &[&str] = &[
    "code", "function", "algorithm", "class", "def", "import",
    "python", "javascript", "typescript", "rust", "golang", "java",
    "react", "vue", "angular", "node", "api", "rest", "graphql",
    "sql", "database", "query", "select", "docker", "kubernetes",
    "git", "commit", "merge", "deploy", "ci/cd", "pipeline",
    "bug", "debug", "error", "exception", "stack trace",
    "unit test", "integration test", "refactor", "optimize",
    "regex", "html", "css", "component", "hook",
    "programming", "compile", "runtime", "async", "await",
    "write a script", "write code", "implement", "binary tree",
    "linked list", "sort", "search", "dynamic programming",
    "```",
];

const MATH_KEYWORDS: &[&str] = &[
    "solve", "equation", "integral", "derivative", "calculus",
    "algebra", "geometry", "trigonometry", "probability",
    "statistics", "theorem", "proof", "prove",
    "matrix", "vector", "eigenvalue", "linear",
    "∑", "∫", "√", "π", "∞", "≤", "≥", "≠",
    "quadratic", "polynomial", "logarithm",
    "combinatorics", "permutation", "binomial",
    "differential", "optimization problem",
    "what is the value of", "compute", "evaluate the expression",
];

const WRITING_KEYWORDS: &[&str] = &[
    "write a story", "write a poem", "write an essay",
    "creative writing", "story about", "narrative",
    "blog post", "article about", "newsletter",
    "rewrite", "rephrase", "paraphrase",
    "tone of voice", "make it sound", "more concise",
    "summarize", "summary", "tl;dr", "tldr",
    "email to", "draft an email", "write a letter",
    "copywriting", "marketing copy", "product description",
    "social media post", "tweet", "caption for",
];

const TRANSLATION_KEYWORDS: &[&str] = &[
    "translate", "translation",
    "in french", "in spanish", "in german", "in chinese",
    "in japanese", "in korean", "in italian", "in portuguese",
    "to english", "to french", "to spanish", "to german",
    "into english", "into french", "into spanish",
    "localize", "localization", "i18n",
];

const ANALYSIS_KEYWORDS: &[&str] = &[
    "analyze", "analysis", "explain why", "explain the",
    "compare", "contrast", "pros and cons",
    "what are the differences", "how does",
    "review this", "critique", "evaluate",
    "assessment", "break down", "breakdown",
    "in depth", "detailed explanation",
    "reasoning", "reason about", "think through",
    "what do you think about", "your opinion on",
    "pros and cons of", "advantages and disadvantages",
    "data analysis", "interpret", "findings",
    "research", "literature review",
];

// ── 复杂度信号词 ─────────────────────────────────────────────────────────────

const SIMPLE_INDICATORS: &[&str] = &[
    "hello", "hi", "hey", "good morning", "good afternoon",
    "thanks", "thank you", "bye", "ok", "okay",
    "what is", "who is", "when did", "where is",
    "define", "definition of",
    "how are you", "what's up",
    "in one sentence", "briefly", "short answer",
    "yes or no",
];

const COMPLEX_INDICATORS: &[&str] = &[
    "in detail", "detailed", "comprehensive",
    "step by step", "step-by-step", "walkthrough",
    "explain thoroughly", "deep dive",
    "write a comprehensive", "write a detailed",
    "architect", "architecture", "system design",
    "design pattern", "best practices",
    "trade-offs", "tradeoffs", "trade offs",
    "multiple", "several", "various",
    "compare and contrast", "pros and cons",
    "production-ready", "production ready",
    "enterprise", "scalable", "high performance",
    "security", "vulnerability",
    "multi-step", "multi step",
    "review and suggest improvements",
];

// ── 分类器 ───────────────────────────────────────────────────────────────────

/// 从请求体的 messages 数组中提取最后一条 user 消息文本
pub fn extract_last_user_message(body: &serde_json::Value) -> Option<String> {
    let messages = body.get("messages")?.as_array()?;
    for msg in messages.iter().rev() {
        if msg.get("role")?.as_str()? == "user" {
            // content 可能是字符串或数组（tool_use 场景）
            let content = msg.get("content")?;
            if let Some(s) = content.as_str() {
                return Some(s.to_lowercase());
            }
            // content 是数组时，拼接所有 text 类型块
            if let Some(arr) = content.as_array() {
                let text = arr
                    .iter()
                    .filter_map(|block| {
                        if block.get("type")?.as_str()? == "text" {
                            block.get("text")?.as_str().map(|s| s.to_string())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                if !text.is_empty() {
                    return Some(text.to_lowercase());
                }
            }
        }
    }
    None
}

/// 对查询文本进行分类，返回 domain + complexity
pub fn classify(text: &str) -> QueryProfile {
    let domain = detect_domain(text);
    let complexity = detect_complexity(text, &domain);
    QueryProfile { domain, complexity }
}

/// 调用外部 Arch-Router 服务对请求体进行分类
///
/// `endpoint` 格式：`http://localhost:8000`（不含路径），函数会补充 `/v1/classify`。
/// 失败时返回 `None`，调用方应回落到内置关键词分类器。
pub async fn classify_remote(
    body: &serde_json::Value,
    endpoint: &str,
) -> Option<QueryProfile> {
    // 从请求体提取 messages 数组
    let messages = body.get("messages")?.as_array()?;
    if messages.is_empty() {
        return None;
    }

    let url = format!("{}/v1/classify", endpoint.trim_end_matches('/'));

    let payload = serde_json::json!({ "messages": messages });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .ok()?;

    let resp = client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        log::debug!("[Classifier] remote endpoint returned {}", resp.status());
        return None;
    }

    #[derive(serde::Deserialize)]
    struct RemoteResult {
        domain: String,
        complexity: String,
    }

    let result: RemoteResult = resp.json().await.ok()?;

    let domain = match result.domain.as_str() {
        "coding"      => Domain::Coding,
        "math"        => Domain::Math,
        "writing"     => Domain::Writing,
        "translation" => Domain::Translation,
        "analysis"    => Domain::Analysis,
        _             => Domain::General,
    };
    let complexity = match result.complexity.as_str() {
        "simple"  => Complexity::Simple,
        "complex" => Complexity::Complex,
        _         => Complexity::Medium,
    };

    Some(QueryProfile { domain, complexity })
}

fn count_matches(text: &str, keywords: &[&str]) -> usize {
    keywords
        .iter()
        .filter(|&&kw| {
            if kw.contains(' ') {
                text.contains(kw)
            } else {
                // 单词边界匹配，避免 "hi" 匹配 "this"
                let mut found = false;
                let mut start = 0;
                while let Some(pos) = text[start..].find(kw) {
                    let abs = start + pos;
                    let before_ok = abs == 0
                        || !text
                            .as_bytes()
                            .get(abs - 1)
                            .map(|b| b.is_ascii_alphanumeric())
                            .unwrap_or(false);
                    let after_ok = abs + kw.len() >= text.len()
                        || !text
                            .as_bytes()
                            .get(abs + kw.len())
                            .map(|b| b.is_ascii_alphanumeric())
                            .unwrap_or(false);
                    if before_ok && after_ok {
                        found = true;
                        break;
                    }
                    start = abs + 1;
                }
                found
            }
        })
        .count()
}

fn detect_domain(text: &str) -> Domain {
    let mut scores = [
        (Domain::Coding, count_matches(text, CODE_KEYWORDS)),
        (Domain::Math, count_matches(text, MATH_KEYWORDS)),
        (Domain::Writing, count_matches(text, WRITING_KEYWORDS)),
        (Domain::Translation, count_matches(text, TRANSLATION_KEYWORDS)),
        (Domain::Analysis, count_matches(text, ANALYSIS_KEYWORDS)),
    ];

    // 代码块标记加权
    if text.contains("```") {
        scores[0].1 += 3;
    }
    // 数学符号加权
    let math_symbols = text
        .chars()
        .filter(|c| "∑∫√π∞≤≥≠∂∏".contains(*c))
        .count();
    scores[1].1 += math_symbols * 2;

    let best = scores.iter().max_by_key(|(_, s)| *s).unwrap();
    if best.1 == 0 {
        Domain::General
    } else {
        best.0.clone()
    }
}

fn detect_complexity(text: &str, domain: &Domain) -> Complexity {
    let word_count = text.split_whitespace().count();
    let mut simple_score: i32 = 0;
    let mut complex_score: i32 = 0;

    // 消息长度信号
    if word_count < 10 {
        simple_score += 3;
    } else if word_count > 80 {
        complex_score += 3;
    } else if word_count > 40 {
        complex_score += 1;
    }

    simple_score += count_matches(text, SIMPLE_INDICATORS) as i32;
    complex_score += count_matches(text, COMPLEX_INDICATORS) as i32;

    // 领域特定信号
    match domain {
        Domain::Math => {
            if text.chars().any(|c| "∑∫√∞≤≥≠∂".contains(c)) {
                complex_score += 2;
            }
            if text.chars().any(|c| c.is_ascii_digit()) {
                complex_score += 1;
            }
        }
        Domain::Coding => {
            if ["architecture", "system design", "multiple files", "refactor"]
                .iter()
                .any(|kw| text.contains(kw))
            {
                complex_score += 3;
            }
            if ["syntax", "error message", "how do i"]
                .iter()
                .any(|kw| text.contains(kw))
            {
                simple_score += 1;
            }
            let paragraphs = text.split("\n\n").filter(|p| !p.trim().is_empty()).count();
            if paragraphs > 3 {
                complex_score += 2;
            }
        }
        Domain::Analysis => {
            complex_score += 1;
        }
        _ => {}
    }

    let net = complex_score - simple_score;
    if net >= 3 {
        Complexity::Complex
    } else if net >= 1 {
        Complexity::Medium
    } else {
        Complexity::Simple
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coding_complex() {
        let profile = classify("please help me design a system architecture for a scalable microservices platform with kubernetes");
        assert_eq!(profile.domain, Domain::Coding);
        assert_eq!(profile.complexity, Complexity::Complex);
    }

    #[test]
    fn test_math_complex() {
        let profile = classify("prove that ∑(1/n²) = π²/6 using the Basel problem approach");
        assert_eq!(profile.domain, Domain::Math);
        assert_eq!(profile.complexity, Complexity::Complex);
    }

    #[test]
    fn test_general_simple() {
        let profile = classify("hello how are you");
        assert_eq!(profile.domain, Domain::General);
        assert_eq!(profile.complexity, Complexity::Simple);
    }

    #[test]
    fn test_translation() {
        let profile = classify("translate this paragraph into french");
        assert_eq!(profile.domain, Domain::Translation);
    }

    #[test]
    fn test_extract_user_message() {
        let body = serde_json::json!({
            "messages": [
                {"role": "user", "content": "hello"},
                {"role": "assistant", "content": "hi"},
                {"role": "user", "content": "Write a sorting algorithm"}
            ]
        });
        let text = extract_last_user_message(&body);
        assert_eq!(text.as_deref(), Some("write a sorting algorithm"));
    }
}
