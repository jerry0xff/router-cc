/**
 * Provider 表单中的"智能路由"配置分区
 *
 * 不写入 live 配置，仅保存在数据库 meta.routingConfig 字段中。
 * 支持对每个模型独立打标签（domain × complexity 多选矩阵）。
 */
import { Plus, Trash2 } from "lucide-react";
import { Switch } from "@/components/ui/switch";
import { useTranslation } from "react-i18next";
import type { ProviderMeta } from "@/types";

// ── 类型定义 ─────────────────────────────────────────────────────────────────

export interface ModelLabel {
  domain: string;
  complexity: string;
}

export interface ModelRoutingConfig {
  modelId: string;
  displayName?: string;
  labels: ModelLabel[];
  qualityScore: number; // 1-5
  costPer1k?: number;
}

export interface RoutingConfig {
  enabled: boolean;
  models: ModelRoutingConfig[];
}

// ── 常量 ─────────────────────────────────────────────────────────────────────

const DOMAINS = [
  { value: "coding", labelKey: "routing.tag.coding", defaultLabel: "代码" },
  { value: "math", labelKey: "routing.tag.math", defaultLabel: "数学" },
  { value: "writing", labelKey: "routing.tag.writing", defaultLabel: "写作" },
  {
    value: "translation",
    labelKey: "routing.tag.translation",
    defaultLabel: "翻译",
  },
  { value: "analysis", labelKey: "routing.tag.analysis", defaultLabel: "分析" },
  { value: "general", labelKey: "routing.tag.general", defaultLabel: "通用" },
] as const;

const COMPLEXITIES = [
  {
    value: "simple",
    labelKey: "routing.complexity.simple",
    defaultLabel: "简单",
  },
  {
    value: "medium",
    labelKey: "routing.complexity.medium",
    defaultLabel: "中等",
  },
  {
    value: "complex",
    labelKey: "routing.complexity.complex",
    defaultLabel: "复杂",
  },
] as const;

function makeEmptyModel(): ModelRoutingConfig {
  return { modelId: "", labels: [], qualityScore: 3 };
}

// ── 主组件 ───────────────────────────────────────────────────────────────────

interface Props {
  value: RoutingConfig;
  onChange: (v: RoutingConfig) => void;
}

export function RoutingConfigSection({ value, onChange }: Props) {
  const { t } = useTranslation();

  const setEnabled = (v: boolean) => onChange({ ...value, enabled: v });

  const addModel = () =>
    onChange({ ...value, models: [...value.models, makeEmptyModel()] });

  const removeModel = (idx: number) =>
    onChange({ ...value, models: value.models.filter((_, i) => i !== idx) });

  const updateModel = (idx: number, patch: Partial<ModelRoutingConfig>) =>
    onChange({
      ...value,
      models: value.models.map((m, i) => (i === idx ? { ...m, ...patch } : m)),
    });

  return (
    <div className="space-y-4 rounded-xl border border-border bg-card/30 p-4">
      {/* 标题行 + 主开关 */}
      <div className="flex items-center justify-between">
        <p className="text-sm font-semibold">
          {t("routing.sectionTitle", { defaultValue: "智能路由" })}
        </p>
        <Switch checked={value.enabled} onCheckedChange={setEnabled} />
      </div>

      <p className="text-xs text-muted-foreground">
        {t("routing.sectionDesc", {
          defaultValue:
            "为每个模型配置标签，智能路由会根据查询内容自动选择最合适的模型",
        })}
      </p>

      <div
        className={`space-y-3 transition-opacity ${
          value.enabled ? "opacity-100" : "opacity-40 pointer-events-none"
        }`}
      >
        {value.models.map((model, idx) => (
          <ModelCard
            key={idx}
            model={model}
            onChange={(patch) => updateModel(idx, patch)}
            onRemove={() => removeModel(idx)}
            t={t}
          />
        ))}

        <button
          type="button"
          onClick={addModel}
          className="flex w-full items-center justify-center gap-1.5 rounded-lg border border-dashed border-border py-2 text-xs text-muted-foreground hover:border-primary/50 hover:text-primary transition-colors"
        >
          <Plus className="h-3.5 w-3.5" />
          {t("routing.addModel", { defaultValue: "添加模型" })}
        </button>
      </div>
    </div>
  );
}

// ── 单模型卡片 ───────────────────────────────────────────────────────────────

interface ModelCardProps {
  model: ModelRoutingConfig;
  onChange: (patch: Partial<ModelRoutingConfig>) => void;
  onRemove: () => void;
  t: ReturnType<typeof useTranslation>["t"];
}

function ModelCard({ model, onChange, onRemove, t }: ModelCardProps) {
  const toggleLabel = (domain: string, complexity: string) => {
    const exists = model.labels.some(
      (l) => l.domain === domain && l.complexity === complexity,
    );
    const labels = exists
      ? model.labels.filter(
          (l) => !(l.domain === domain && l.complexity === complexity),
        )
      : [...model.labels, { domain, complexity }];
    onChange({ labels });
  };

  const isActive = (domain: string, complexity: string) =>
    model.labels.some(
      (l) => l.domain === domain && l.complexity === complexity,
    );

  return (
    <div className="rounded-lg border border-border bg-background/50 p-3 space-y-3">
      {/* 模型 ID + 删除按钮 */}
      <div className="flex items-center gap-2">
        <input
          type="text"
          placeholder={t("routing.modelIdPlaceholder", {
            defaultValue: "模型 ID，如 claude-opus-4-5",
          })}
          value={model.modelId}
          onChange={(e) => onChange({ modelId: e.target.value })}
          className="flex-1 rounded-md border border-border bg-background px-3 py-1.5 text-sm placeholder:text-muted-foreground/50 focus:outline-none focus:ring-1 focus:ring-primary"
        />
        <button
          type="button"
          onClick={onRemove}
          className="shrink-0 rounded-md p-1.5 text-muted-foreground hover:bg-destructive/10 hover:text-destructive transition-colors"
        >
          <Trash2 className="h-4 w-4" />
        </button>
      </div>

      {/* 标签矩阵 (domain × complexity) */}
      <div className="overflow-x-auto">
        <table className="w-full text-xs border-collapse">
          <thead>
            <tr>
              <th className="w-16 pb-1 text-left text-muted-foreground font-normal" />
              {COMPLEXITIES.map(({ value, labelKey, defaultLabel }) => (
                <th
                  key={value}
                  className="pb-1 text-center text-muted-foreground font-normal px-1"
                >
                  {t(labelKey, { defaultValue: defaultLabel })}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {DOMAINS.map(({ value: domain, labelKey, defaultLabel }) => (
              <tr key={domain}>
                <td className="py-0.5 text-muted-foreground">
                  {t(labelKey, { defaultValue: defaultLabel })}
                </td>
                {COMPLEXITIES.map(({ value: complexity }) => {
                  const active = isActive(domain, complexity);
                  return (
                    <td key={complexity} className="py-0.5 px-1 text-center">
                      <button
                        type="button"
                        onClick={() => toggleLabel(domain, complexity)}
                        className={`h-6 w-full rounded transition-colors ${
                          active
                            ? "bg-primary text-primary-foreground"
                            : "border border-border bg-background text-muted-foreground/30 hover:border-primary/50"
                        }`}
                        title={`${domain} · ${complexity}`}
                      >
                        {active ? "✓" : ""}
                      </button>
                    </td>
                  );
                })}
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* 质量评分 */}
      <div className="flex items-center gap-2">
        <span className="text-xs text-muted-foreground shrink-0">
          {t("routing.qualityScore", { defaultValue: "质量" })}
        </span>
        <div className="flex gap-0.5">
          {[1, 2, 3, 4, 5].map((score) => (
            <button
              key={score}
              type="button"
              onClick={() => onChange({ qualityScore: score })}
              className={`text-base leading-none transition-colors ${
                score <= model.qualityScore
                  ? "text-yellow-400"
                  : "text-muted-foreground/20"
              }`}
            >
              ★
            </button>
          ))}
          <span className="ml-1 text-xs text-muted-foreground self-center">
            {model.qualityScore}/5
          </span>
        </div>

        {/* 成本 */}
        <span className="text-xs text-muted-foreground ml-4 shrink-0">
          {t("routing.costPer1k", { defaultValue: "$/1k" })}
        </span>
        <input
          type="number"
          min={0}
          step={0.0001}
          placeholder="0.003"
          value={model.costPer1k ?? ""}
          onChange={(e) => {
            const v = parseFloat(e.target.value);
            onChange({ costPer1k: isNaN(v) ? undefined : v });
          }}
          className="w-24 rounded-md border border-border bg-background px-2 py-1 text-xs placeholder:text-muted-foreground/50 focus:outline-none focus:ring-1 focus:ring-primary"
        />
      </div>
    </div>
  );
}

// ── Meta 序列化 / 反序列化 ────────────────────────────────────────────────────

/** 将 ProviderMeta.routingConfig 转换为组件使用的格式 */
export function fromMeta(meta: ProviderMeta | undefined): RoutingConfig {
  const rc = meta?.routingConfig as Record<string, unknown> | undefined;
  if (!rc) return { enabled: false, models: [] };

  const rawModels = (rc.models as unknown[]) ?? [];
  const models: ModelRoutingConfig[] = rawModels.map((m) => {
    const model = m as Record<string, unknown>;
    const rawLabels = (model.labels as unknown[]) ?? [];
    return {
      modelId: (model.modelId as string) ?? "",
      displayName: model.displayName as string | undefined,
      labels: rawLabels.map((l) => {
        const label = l as Record<string, unknown>;
        return {
          domain: label.domain as string,
          complexity: label.complexity as string,
        };
      }),
      qualityScore: (model.qualityScore as number) ?? 3,
      costPer1k: model.costPer1k as number | undefined,
    };
  });

  return { enabled: (rc.enabled as boolean) ?? false, models };
}

/** 将组件格式写回 meta.routingConfig */
export function toMeta(config: RoutingConfig): Record<string, unknown> {
  return {
    enabled: config.enabled,
    models: config.models.map((m) => ({
      modelId: m.modelId,
      ...(m.displayName ? { displayName: m.displayName } : {}),
      labels: m.labels,
      qualityScore: m.qualityScore,
      ...(m.costPer1k !== undefined ? { costPer1k: m.costPer1k } : {}),
    })),
  };
}
