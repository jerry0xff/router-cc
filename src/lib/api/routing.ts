import { invoke } from "@tauri-apps/api/core";

export interface IntelligentRoutingSettings {
  enabled: boolean;
  strategy: "rule_match" | "avengers";
  avengersAlpha: number;
  fallbackToCurrent: boolean;
  showRoutingReason: boolean;
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
