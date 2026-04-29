import { invoke } from "@tauri-apps/api/core";

export interface IntelligentRoutingSettings {
  enabled: boolean;
  strategy: "arch_router" | "rule_match" | "avengers";
  avengersAlpha: number;
  fallbackToCurrent: boolean;
  showRoutingReason: boolean;
  /** 外部 Arch-Router 服务端点，如 "http://localhost:8000"。留空使用内置关键词分类器 */
  archRouterEndpoint?: string;
}

export const routingApi = {
  async getSettings(appType: string): Promise<IntelligentRoutingSettings> {
    return invoke("get_intelligent_routing_settings", { appType });
  },

  async updateSettings(
    appType: string,
    settings: IntelligentRoutingSettings,
  ): Promise<void> {
    return invoke("update_intelligent_routing_settings", { appType, settings });
  },
};
