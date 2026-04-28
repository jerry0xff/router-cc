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
            enabled: false,
            strategy: RoutingStrategy::default(),
            avengers_alpha: 0.7,
            fallback_to_current: true,
            show_routing_reason: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RoutingStrategy {
    /// 标签规则匹配 + 质量分排序
    #[default]
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

    // 筛选：参与智能路由 + 能力标签匹配 + 复杂度覆盖
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
        RoutingStrategy::RuleMatch => select_rule_match(profile, &candidates),
        RoutingStrategy::Avengers => {
            select_avengers(profile, &candidates, settings.avengers_alpha)
        }
    }
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
