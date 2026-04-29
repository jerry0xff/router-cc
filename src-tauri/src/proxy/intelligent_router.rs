//! 智能路由器 — 基于查询画像选择最优 Provider
//!
//! 两种策略：
//! - rule_match：匹配 provider 的能力标签，按质量评分排序
//! - avengers：α·quality + (1-α)·cost_efficiency 综合评分
//!
//! Key 全程保留在本地，此模块不涉及任何网络请求。

use super::query_classifier::{Complexity, Domain, QueryProfile};
use crate::provider::{Provider, ProviderRoutingConfig};
use serde::{Deserialize, Serialize};

// ── 路由设置（全局，按 app_type 存储）─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntelligentRoutingSettings {
    /// 是否启用智能路由
    #[serde(default)]
    pub enabled: bool,
    /// 路由策略
    #[serde(default)]
    pub strategy: RoutingStrategy,
    /// Avengers 策略的 α 参数（0=纯成本，1=纯性能），默认 0.7
    #[serde(default = "default_alpha")]
    pub avengers_alpha: f64,
    /// 无匹配时是否回落到当前 provider（默认 true）
    #[serde(default = "default_true")]
    pub fallback_to_current: bool,
    /// 是否在代理面板显示路由原因
    #[serde(default = "default_true")]
    pub show_routing_reason: bool,
}

fn default_alpha() -> f64 {
    0.7
}
fn default_true() -> bool {
    true
}

impl Default for IntelligentRoutingSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            strategy: RoutingStrategy::ArchRouter,
            avengers_alpha: 0.7,
            fallback_to_current: true,
            show_routing_reason: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RoutingStrategy {
    /// Arch-Router：按查询类型自动识别最优 provider，无需手动打标签
    #[default]
    ArchRouter,
    /// 标签规则匹配 + 质量分排序
    RuleMatch,
    /// Avengers 综合评分（α·质量 + (1-α)·成本效率）
    Avengers,
}

// ── 路由决策结果 ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouterDecision {
    pub provider_id: String,
    pub provider_name: String,
    /// 人类可读的路由原因，例如 "coding · complex → 质量最高(5)"
    pub reason: String,
    pub domain: String,
    pub complexity: String,
    pub strategy: String,
}

// ── 主入口 ───────────────────────────────────────────────────────────────────

/// 从可用 provider 列表中选择最优 provider。
///
/// 返回 `None` 表示无合适 provider，调用方应回落到原有故障转移队列。
pub fn select(
    profile: &QueryProfile,
    providers: &[Provider],
    settings: &IntelligentRoutingSettings,
) -> Option<RouterDecision> {
    if !settings.enabled {
        return None;
    }

    // ArchRouter 不依赖用户配置的 routing_config 标签，直接匹配
    if settings.strategy == RoutingStrategy::ArchRouter {
        return select_arch_router(profile, providers);
    }

    // rule_match / avengers：筛选出显式配置了路由标签的 providers
    let candidates: Vec<(&Provider, &ProviderRoutingConfig)> = providers
        .iter()
        .filter_map(|p| {
            let rc = p.meta.as_ref()?.routing_config.as_ref()?;
            if !rc.enabled {
                return None;
            }
            if !tag_matches(rc, &profile.domain) {
                return None;
            }
            if !complexity_matches(rc, &profile.complexity) {
                return None;
            }
            Some((p, rc))
        })
        .collect();

    if candidates.is_empty() {
        return None;
    }

    match settings.strategy {
        RoutingStrategy::ArchRouter => unreachable!(),
        RoutingStrategy::RuleMatch => select_rule_match(profile, &candidates),
        RoutingStrategy::Avengers => {
            select_avengers(profile, &candidates, settings.avengers_alpha)
        }
    }
}

// ── Arch-Router 策略 ─────────────────────────────────────────────────────────
//
// 按 (domain, complexity) 查路由表，得到有序的 "provider 类型" 列表，
// 逐一检查可用 providers 是否匹配（按名称 / baseURL / icon 关键词），
// 返回第一个命中的 provider。无需用户手动打标签。

/// 从 provider 的 settingsConfig 中提取 baseURL（兼容大小写）
fn get_base_url(provider: &Provider) -> String {
    provider
        .settings_config
        .get("baseURL")
        .or_else(|| provider.settings_config.get("baseUrl"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase()
}

/// 判断 provider 是否属于某个 "provider 类型"
///
/// 匹配顺序：name → baseURL → icon，任一包含关键词即命中。
fn provider_matches_family(provider: &Provider, family: &str) -> bool {
    let name = provider.name.to_lowercase();
    let url = get_base_url(provider);
    let icon = provider.icon.as_deref().unwrap_or("").to_lowercase();
    let check = |kw: &str| name.contains(kw) || url.contains(kw) || icon.contains(kw);

    match family {
        // Anthropic — 按档位细分
        "claude-opus"   => check("opus"),
        "claude-sonnet" => check("sonnet"),
        "claude-haiku"  => check("haiku"),
        "claude"        => check("claude") || check("anthropic"),
        // DeepSeek
        "deepseek-r1"   => check("deepseek-r1") || check("deepseek-reasoner") || check("/r1"),
        "deepseek"      => check("deepseek"),
        // OpenAI
        "gpt-o"         => check("o1") || check("o3") || check("o4"),
        "openai"        => check("openai") || check("gpt"),
        // Google
        "gemini-pro"    => check("gemini") && (check("pro") || check("ultra") || check("2.5-pro") || check("3.1-pro")),
        "gemini"        => check("gemini"),
        // 其他
        "groq"          => check("groq"),
        "mistral"       => check("mistral"),
        "qwen"          => check("qwen") || check("tongyi") || check("alibaba"),
        _               => false,
    }
}

/// (domain, complexity) → 按优先级排列的 provider 类型列表
fn arch_route_families(domain: &Domain, complexity: &Complexity) -> &'static [&'static str] {
    match (domain, complexity) {
        // coding
        (Domain::Coding, Complexity::Simple)   => &["deepseek", "claude-haiku", "openai", "gemini", "groq"],
        (Domain::Coding, Complexity::Medium)   => &["claude-sonnet", "deepseek-r1", "openai", "deepseek", "gemini-pro", "claude"],
        (Domain::Coding, Complexity::Complex)  => &["claude-opus", "gpt-o", "deepseek-r1", "claude-sonnet", "openai", "deepseek", "claude"],
        // math
        (Domain::Math, Complexity::Simple)     => &["deepseek", "openai", "gemini", "groq"],
        (Domain::Math, Complexity::Medium)     => &["deepseek-r1", "gpt-o", "claude-sonnet", "openai", "deepseek"],
        (Domain::Math, Complexity::Complex)    => &["deepseek-r1", "gpt-o", "claude-opus", "claude-sonnet", "openai"],
        // writing
        (Domain::Writing, Complexity::Simple)  => &["deepseek", "claude-haiku", "openai", "gemini"],
        (Domain::Writing, Complexity::Medium)  => &["claude-sonnet", "deepseek", "openai", "gemini-pro"],
        (Domain::Writing, Complexity::Complex) => &["claude-opus", "claude-sonnet", "openai", "deepseek"],
        // translation
        (Domain::Translation, Complexity::Simple)  => &["deepseek", "openai", "gemini", "groq"],
        (Domain::Translation, Complexity::Medium)  => &["deepseek", "claude-haiku", "openai", "qwen"],
        (Domain::Translation, Complexity::Complex) => &["claude-sonnet", "openai", "deepseek", "qwen"],
        // analysis
        (Domain::Analysis, Complexity::Simple)  => &["deepseek", "openai", "gemini"],
        (Domain::Analysis, Complexity::Medium)  => &["deepseek-r1", "claude-sonnet", "openai", "deepseek"],
        (Domain::Analysis, Complexity::Complex) => &["deepseek-r1", "claude-opus", "gpt-o", "claude-sonnet", "openai"],
        // general
        (Domain::General, Complexity::Simple)   => &["deepseek", "groq", "openai", "gemini"],
        (Domain::General, Complexity::Medium)   => &["deepseek", "claude-haiku", "openai", "gemini"],
        (Domain::General, Complexity::Complex)  => &["deepseek", "claude-sonnet", "openai", "gemini-pro"],
    }
}

/// Arch-Router：直接在全部可用 provider 上做类型匹配，无需手动配置路由标签
fn select_arch_router(
    profile: &QueryProfile,
    providers: &[Provider],
) -> Option<RouterDecision> {
    let families = arch_route_families(&profile.domain, &profile.complexity);

    for family in families {
        if let Some(provider) = providers.iter().find(|p| provider_matches_family(p, family)) {
            return Some(RouterDecision {
                provider_id: provider.id.clone(),
                provider_name: provider.name.clone(),
                reason: format!(
                    "{} · {} → {}",
                    profile.domain, profile.complexity, provider.name
                ),
                domain: profile.domain.to_string(),
                complexity: profile.complexity.to_string(),
                strategy: "arch_router".to_string(),
            });
        }
    }
    None
}

// ── 规则匹配策略 ─────────────────────────────────────────────────────────────

fn select_rule_match(
    profile: &QueryProfile,
    candidates: &[(&Provider, &ProviderRoutingConfig)],
) -> Option<RouterDecision> {
    // 按质量评分降序，相同分数保持原有顺序（stable sort）
    let mut ranked: Vec<_> = candidates.iter().collect();
    ranked.sort_by(|(_, a), (_, b)| b.quality_score.cmp(&a.quality_score));

    let (provider, rc) = ranked.first()?;
    Some(RouterDecision {
        provider_id: provider.id.clone(),
        provider_name: provider.name.clone(),
        reason: format!(
            "{} · {} → 质量最高({})",
            profile.domain,
            profile.complexity,
            rc.quality_score
        ),
        domain: profile.domain.to_string(),
        complexity: profile.complexity.to_string(),
        strategy: "rule_match".to_string(),
    })
}

// ── Avengers 评分策略 ────────────────────────────────────────────────────────

fn select_avengers(
    profile: &QueryProfile,
    candidates: &[(&Provider, &ProviderRoutingConfig)],
    alpha: f64,
) -> Option<RouterDecision> {
    // 归一化成本（input + output per 1k tokens）
    let costs: Vec<f64> = candidates
        .iter()
        .map(|(_, rc)| rc.cost_per_1k.unwrap_or(0.0))
        .collect();

    let min_cost = costs.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_cost = costs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let cost_range = if (max_cost - min_cost).abs() < 1e-9 {
        1.0
    } else {
        max_cost - min_cost
    };

    let mut scored: Vec<(f64, &Provider, &ProviderRoutingConfig)> = candidates
        .iter()
        .zip(costs.iter())
        .map(|((p, rc), &cost)| {
            let perf = rc.quality_score as f64 / 5.0; // 归一化到 0-1
            let cost_norm = (cost - min_cost) / cost_range;
            let cost_efficiency = 1.0 - cost_norm;
            let score = alpha * perf + (1.0 - alpha) * cost_efficiency;
            (score, *p, *rc)
        })
        .collect();

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    let (score, provider, _rc) = scored.first()?;
    Some(RouterDecision {
        provider_id: provider.id.clone(),
        provider_name: provider.name.clone(),
        reason: format!(
            "{} · {} → Avengers(α={:.1}, 分={:.2})",
            profile.domain,
            profile.complexity,
            alpha,
            score
        ),
        domain: profile.domain.to_string(),
        complexity: profile.complexity.to_string(),
        strategy: format!("avengers(α={alpha:.1})"),
    })
}

// ── 辅助函数 ─────────────────────────────────────────────────────────────────

/// 检查 provider 的能力标签是否覆盖当前 domain
fn tag_matches(rc: &ProviderRoutingConfig, domain: &Domain) -> bool {
    if rc.tags.is_empty() {
        return false;
    }
    let domain_str = domain.as_str();
    rc.tags.iter().any(|t| t == domain_str || t == "general")
}

/// 检查 provider 的复杂度覆盖范围是否包含当前 complexity
fn complexity_matches(rc: &ProviderRoutingConfig, complexity: &Complexity) -> bool {
    match rc.complexity.as_str() {
        "all" => true,
        "simple" => complexity == &Complexity::Simple,
        "medium" => complexity != &Complexity::Complex,
        "complex" => complexity == &Complexity::Complex,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{Provider, ProviderMeta, ProviderRoutingConfig};
    use serde_json::json;

    fn make_provider(id: &str, tags: &[&str], complexity: &str, quality: u8, cost: f64) -> Provider {
        let mut meta = ProviderMeta::default();
        meta.routing_config = Some(ProviderRoutingConfig {
            enabled: true,
            tags: tags.iter().map(|s| s.to_string()).collect(),
            complexity: complexity.to_string(),
            quality_score: quality,
            cost_per_1k: Some(cost),
        });
        let mut p = Provider::with_id(
            id.to_string(),
            format!("Provider {id}"),
            json!({}),
            None,
        );
        p.meta = Some(meta);
        p
    }

    #[test]
    fn test_rule_match_picks_highest_quality() {
        let providers = vec![
            make_provider("a", &["coding"], "all", 3, 0.001),
            make_provider("b", &["coding"], "all", 5, 0.005),
            make_provider("c", &["writing"], "all", 5, 0.001),
        ];
        let profile = QueryProfile {
            domain: Domain::Coding,
            complexity: Complexity::Complex,
        };
        let settings = IntelligentRoutingSettings {
            enabled: true,
            strategy: RoutingStrategy::RuleMatch,
            ..Default::default()
        };
        let decision = select(&profile, &providers, &settings).unwrap();
        assert_eq!(decision.provider_id, "b");
    }

    #[test]
    fn test_no_match_returns_none() {
        let providers = vec![make_provider("a", &["writing"], "all", 5, 0.001)];
        let profile = QueryProfile {
            domain: Domain::Math,
            complexity: Complexity::Complex,
        };
        let settings = IntelligentRoutingSettings {
            enabled: true,
            strategy: RoutingStrategy::RuleMatch,
            ..Default::default()
        };
        assert!(select(&profile, &providers, &settings).is_none());
    }

    #[test]
    fn test_disabled_returns_none() {
        let providers = vec![make_provider("a", &["coding"], "all", 5, 0.001)];
        let profile = QueryProfile {
            domain: Domain::Coding,
            complexity: Complexity::Simple,
        };
        let settings = IntelligentRoutingSettings {
            enabled: false,
            ..Default::default()
        };
        assert!(select(&profile, &providers, &settings).is_none());
    }

    #[test]
    fn test_complexity_filter() {
        let providers = vec![
            make_provider("simple_only", &["coding"], "simple", 5, 0.0),
            make_provider("all", &["coding"], "all", 3, 0.0),
        ];
        let profile = QueryProfile {
            domain: Domain::Coding,
            complexity: Complexity::Complex,
        };
        let settings = IntelligentRoutingSettings {
            enabled: true,
            strategy: RoutingStrategy::RuleMatch,
            ..Default::default()
        };
        let decision = select(&profile, &providers, &settings).unwrap();
        // simple_only 被过滤，只剩 all
        assert_eq!(decision.provider_id, "all");
    }
}
