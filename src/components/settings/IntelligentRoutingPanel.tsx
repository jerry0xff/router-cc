import { useEffect, useState } from "react";
import { Save, Loader2, Zap } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { ToggleRow } from "@/components/ui/toggle-row";
import {
  useRoutingSettings,
  useUpdateRoutingSettings,
} from "@/lib/query/routing";
import type { IntelligentRoutingSettings } from "@/lib/api/routing";
import { useTranslation } from "react-i18next";

interface Props {
  /** "claude" | "codex" | "gemini" */
  appType: string;
}

const DEFAULT_SETTINGS: IntelligentRoutingSettings = {
  enabled: true,
  strategy: "arch_router",
  avengersAlpha: 0.7,
  fallbackToCurrent: true,
  showRoutingReason: true,
  archRouterEndpoint: "",
};

export function IntelligentRoutingPanel({ appType }: Props) {
  const { t } = useTranslation();
  const { data: saved, isLoading } = useRoutingSettings(appType);
  const updateSettings = useUpdateRoutingSettings(appType);

  const [form, setForm] =
    useState<IntelligentRoutingSettings>(DEFAULT_SETTINGS);

  useEffect(() => {
    if (saved) setForm(saved);
  }, [saved]);

  const set = <K extends keyof IntelligentRoutingSettings>(
    key: K,
    value: IntelligentRoutingSettings[K],
  ) => setForm((prev) => ({ ...prev, [key]: value }));

  const handleSave = () => updateSettings.mutate(form);

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-8 text-muted-foreground">
        <Loader2 className="h-4 w-4 animate-spin mr-2" />
        {t("common.loading", { defaultValue: "加载中..." })}
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* 主开关 */}
      <ToggleRow
        icon={<Zap className="h-4 w-4 text-yellow-500" />}
        title={t("routing.enable", { defaultValue: "启用智能路由" })}
        description={t("routing.enableDesc", {
          defaultValue:
            "根据查询内容自动选择最合适的供应商，替代固定的当前供应商",
        })}
        checked={form.enabled}
        onCheckedChange={(v) => set("enabled", v)}
      />

      {/* 以下配置仅在启用时生效 */}
      <div
        className={`space-y-4 transition-opacity ${form.enabled ? "opacity-100" : "opacity-40 pointer-events-none"}`}
      >
        {/* 路由策略 */}
        <div className="rounded-xl border border-border bg-card/50 p-4 space-y-3">
          <p className="text-sm font-medium">
            {t("routing.strategy", { defaultValue: "路由策略" })}
          </p>
          <div className="grid grid-cols-3 gap-3">
            <StrategyCard
              value="arch_router"
              current={form.strategy}
              title={t("routing.strategy.archRouter", {
                defaultValue: "Arch 智能",
              })}
              description={t("routing.strategy.archRouterDesc", {
                defaultValue: "按查询类型自动识别最优 provider，无需配置",
              })}
              onSelect={() => set("strategy", "arch_router")}
            />
            <StrategyCard
              value="rule_match"
              current={form.strategy}
              title={t("routing.strategy.ruleMatch", {
                defaultValue: "规则匹配",
              })}
              description={t("routing.strategy.ruleMatchDesc", {
                defaultValue: "匹配能力标签，按质量评分选最优",
              })}
              onSelect={() => set("strategy", "rule_match")}
            />
            <StrategyCard
              value="avengers"
              current={form.strategy}
              title={t("routing.strategy.avengers", {
                defaultValue: "Avengers",
              })}
              description={t("routing.strategy.avengersDesc", {
                defaultValue: "综合性能与成本，α 参数可调",
              })}
              onSelect={() => set("strategy", "avengers")}
            />
          </div>
        </div>

        {/* Arch-Router 外部端点 */}
        {form.strategy === "arch_router" && (
          <div className="rounded-xl border border-border bg-card/50 p-4 space-y-2">
            <p className="text-sm font-medium">
              {t("routing.archRouterEndpoint", {
                defaultValue: "Arch-Router 服务端点（可选）",
              })}
            </p>
            <p className="text-xs text-muted-foreground">
              {t("routing.archRouterEndpointDesc", {
                defaultValue:
                  "填入本地或远程 Router 服务地址（如 http://localhost:8000），留空则使用内置关键词分类器",
              })}
            </p>
            <input
              type="url"
              placeholder="http://localhost:8000"
              value={form.archRouterEndpoint ?? ""}
              onChange={(e) => set("archRouterEndpoint", e.target.value)}
              className="w-full rounded-md border border-border bg-background px-3 py-1.5 text-sm font-mono placeholder:text-muted-foreground/50 focus:outline-none focus:ring-1 focus:ring-primary"
            />
          </div>
        )}

        {/* Avengers α 参数 */}
        {form.strategy === "avengers" && (
          <div className="rounded-xl border border-border bg-card/50 p-4 space-y-3">
            <div className="flex items-center justify-between">
              <p className="text-sm font-medium">
                {t("routing.avengersAlpha", {
                  defaultValue: "性能 / 成本权衡 (α)",
                })}
              </p>
              <span className="text-sm font-mono text-primary">
                {form.avengersAlpha.toFixed(1)}
              </span>
            </div>
            <input
              type="range"
              min={0}
              max={1}
              step={0.1}
              value={form.avengersAlpha}
              onChange={(e) => set("avengersAlpha", parseFloat(e.target.value))}
              className="w-full accent-primary"
            />
            <div className="flex justify-between text-xs text-muted-foreground">
              <span>
                {t("routing.costFirst", { defaultValue: "成本优先" })}
              </span>
              <span>{t("routing.balanced", { defaultValue: "均衡" })}</span>
              <span>
                {t("routing.perfFirst", { defaultValue: "性能优先" })}
              </span>
            </div>
          </div>
        )}

        {/* 其他选项 */}
        <div className="space-y-2">
          <div className="flex items-center justify-between rounded-lg border border-border bg-card/50 px-4 py-3">
            <div>
              <Label className="text-sm font-medium">
                {t("routing.fallback", {
                  defaultValue: "无匹配时回落到当前供应商",
                })}
              </Label>
              <p className="text-xs text-muted-foreground">
                {t("routing.fallbackDesc", {
                  defaultValue: "若无供应商满足查询条件，使用默认供应商",
                })}
              </p>
            </div>
            <Switch
              checked={form.fallbackToCurrent}
              onCheckedChange={(v) => set("fallbackToCurrent", v)}
            />
          </div>

          <div className="flex items-center justify-between rounded-lg border border-border bg-card/50 px-4 py-3">
            <div>
              <Label className="text-sm font-medium">
                {t("routing.showReason", {
                  defaultValue: "代理面板显示路由原因",
                })}
              </Label>
              <p className="text-xs text-muted-foreground">
                {t("routing.showReasonDesc", {
                  defaultValue: "在代理状态中展示本次选择的供应商及原因",
                })}
              </p>
            </div>
            <Switch
              checked={form.showRoutingReason}
              onCheckedChange={(v) => set("showRoutingReason", v)}
            />
          </div>
        </div>
      </div>

      {/* 保存按钮 */}
      <div className="flex justify-end">
        <Button
          size="sm"
          onClick={handleSave}
          disabled={updateSettings.isPending}
        >
          {updateSettings.isPending ? (
            <>
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              {t("common.saving", { defaultValue: "保存中..." })}
            </>
          ) : (
            <>
              <Save className="mr-2 h-4 w-4" />
              {t("common.save", { defaultValue: "保存" })}
            </>
          )}
        </Button>
      </div>
    </div>
  );
}

// ── 策略卡片 ──────────────────────────────────────────────────────────────────

interface StrategyCardProps {
  value: string;
  current: string;
  title: string;
  description: string;
  onSelect: () => void;
}

function StrategyCard({
  value,
  current,
  title,
  description,
  onSelect,
}: StrategyCardProps) {
  const selected = value === current;
  return (
    <button
      type="button"
      onClick={onSelect}
      className={`rounded-lg border p-3 text-left transition-colors hover:bg-muted/50 ${
        selected ? "border-primary/60 bg-primary/5" : "border-border bg-card/50"
      }`}
    >
      <p className={`text-sm font-medium ${selected ? "text-primary" : ""}`}>
        {title}
      </p>
      <p className="mt-1 text-xs text-muted-foreground">{description}</p>
    </button>
  );
}
