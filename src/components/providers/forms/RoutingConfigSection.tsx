/**
 * Provider 表单中的"智能路由"配置分区
 *
 * 不写入 live 配置，仅保存在数据库 meta.routingConfig 字段中。
 */
import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";
import { useTranslation } from "react-i18next";
import type { ProviderMeta } from "@/types";

export interface RoutingConfig {
  enabled: boolean;
  tags: string[];
  complexity: "simple" | "medium" | "complex" | "all";
  qualityScore: number; // 1-5
  costPer1k?: number;
}

const DOMAIN_TAGS = [
  { value: "coding", labelKey: "routing.tag.coding", defaultLabel: "代码/编程" },
  { value: "math", labelKey: "routing.tag.math", defaultLabel: "数学/推理" },
  { value: "writing", labelKey: "routing.tag.writing", defaultLabel: "写作/创意" },
  { value: "translation", labelKey: "routing.tag.translation", defaultLabel: "翻译" },
  { value: "analysis", labelKey: "routing.tag.analysis", defaultLabel: "分析理解" },
  { value: "general", labelKey: "routing.tag.general", defaultLabel: "通用对话" },
] as const;

const COMPLEXITY_OPTIONS = [
  { value: "simple", labelKey: "routing.complexity.simple", defaultLabel: "简单" },
  { value: "medium", labelKey: "routing.complexity.medium", defaultLabel: "中等" },
  { value: "complex", labelKey: "routing.complexity.complex", defaultLabel: "复杂" },
  { value: "all", labelKey: "routing.complexity.all", defaultLabel: "全部" },
] as const;

interface Props {
  value: RoutingConfig;
  onChange: (v: RoutingConfig) => void;
}

export function RoutingConfigSection({ value, onChange }: Props) {
  const { t } = useTranslation();

  const set = <K extends keyof RoutingConfig>(k: K, v: RoutingConfig[K]) =>
    onChange({ ...value, [k]: v });

  const toggleTag = (tag: string) => {
    const tags = value.tags.includes(tag)
      ? value.tags.filter((t) => t !== tag)
      : [...value.tags, tag];
    set("tags", tags);
  };

  return (
    <div className="space-y-4 rounded-xl border border-border bg-card/30 p-4">
      {/* 标题行 + 主开关 */}
      <div className="flex items-center justify-between">
        <p className="text-sm font-semibold">
          {t("routing.sectionTitle", { defaultValue: "智能路由" })}
        </p>
        <Switch
          checked={value.enabled}
          onCheckedChange={(v) => set("enabled", v)}
        />
      </div>

      <p className="text-xs text-muted-foreground">
        {t("routing.sectionDesc", {
          defaultValue:
            "配置后，智能路由会根据查询内容优先选择此供应商",
        })}
      </p>

      <div
        className={`space-y-4 transition-opacity ${
          value.enabled ? "opacity-100" : "opacity-40 pointer-events-none"
        }`}
      >
        {/* 能力标签 */}
        <div className="space-y-2">
          <Label className="text-xs text-muted-foreground">
            {t("routing.tags", { defaultValue: "能力标签" })}
          </Label>
          <div className="flex flex-wrap gap-2">
            {DOMAIN_TAGS.map(({ value: tagValue, labelKey, defaultLabel }) => {
              const selected = value.tags.includes(tagValue);
              return (
                <button
                  key={tagValue}
                  type="button"
                  onClick={() => toggleTag(tagValue)}
                  className={`rounded-full px-3 py-1 text-xs font-medium transition-colors ${
                    selected
                      ? "bg-primary text-primary-foreground"
                      : "border border-border bg-background text-muted-foreground hover:border-primary/50"
                  }`}
                >
                  {t(labelKey, { defaultValue: defaultLabel })}
                </button>
              );
            })}
          </div>
        </div>

        {/* 复杂度覆盖 */}
        <div className="space-y-2">
          <Label className="text-xs text-muted-foreground">
            {t("routing.complexityCoverage", { defaultValue: "复杂度覆盖" })}
          </Label>
          <div className="flex gap-2">
            {COMPLEXITY_OPTIONS.map(({ value: cVal, labelKey, defaultLabel }) => (
              <button
                key={cVal}
                type="button"
                onClick={() => set("complexity", cVal)}
                className={`flex-1 rounded-md border py-1.5 text-xs font-medium transition-colors ${
                  value.complexity === cVal
                    ? "border-primary/60 bg-primary/10 text-primary"
                    : "border-border bg-background text-muted-foreground hover:border-primary/30"
                }`}
              >
                {t(labelKey, { defaultValue: defaultLabel })}
              </button>
            ))}
          </div>
        </div>

        {/* 质量评分 */}
        <div className="space-y-2">
          <Label className="text-xs text-muted-foreground">
            {t("routing.qualityScore", { defaultValue: "质量评分（影响规则匹配的优先级）" })}
          </Label>
          <div className="flex gap-1">
            {[1, 2, 3, 4, 5].map((score) => (
              <button
                key={score}
                type="button"
                onClick={() => set("qualityScore", score)}
                className={`text-lg transition-colors ${
                  score <= value.qualityScore
                    ? "text-yellow-400"
                    : "text-muted-foreground/30"
                }`}
              >
                ★
              </button>
            ))}
            <span className="ml-2 text-xs text-muted-foreground self-center">
              {value.qualityScore}/5
            </span>
          </div>
        </div>

        {/* 成本（可选，用于 Avengers 评分） */}
        <div className="space-y-2">
          <Label className="text-xs text-muted-foreground">
            {t("routing.costPer1k", {
              defaultValue: "每 1k token 成本（USD，用于 Avengers 评分，留空则不参与成本计算）",
            })}
          </Label>
          <input
            type="number"
            min={0}
            step={0.0001}
            placeholder="0.0030"
            value={value.costPer1k ?? ""}
            onChange={(e) => {
              const v = parseFloat(e.target.value);
              set("costPer1k", isNaN(v) ? undefined : v);
            }}
            className="w-full rounded-md border border-border bg-background px-3 py-1.5 text-sm placeholder:text-muted-foreground/50 focus:outline-none focus:ring-1 focus:ring-primary"
          />
        </div>
      </div>
    </div>
  );
}

/** 将 ProviderMeta.routingConfig 转换为组件使用的格式 */
export function fromMeta(meta: ProviderMeta | undefined): RoutingConfig {
  const rc = meta?.routingConfig;
  return {
    enabled: (rc?.enabled as boolean) ?? false,
    tags: (rc?.tags as string[]) ?? [],
    complexity: (rc?.complexity as RoutingConfig["complexity"]) ?? "all",
    qualityScore: (rc?.quality_score as number) ?? 3,
    costPer1k: rc?.cost_per_1k as number | undefined,
  };
}

/** 将组件格式写回 meta.routingConfig */
export function toMeta(config: RoutingConfig): Record<string, unknown> {
  return {
    enabled: config.enabled,
    tags: config.tags,
    complexity: config.complexity,
    quality_score: config.qualityScore,
    ...(config.costPer1k !== undefined ? { cost_per_1k: config.costPer1k } : {}),
  };
}
