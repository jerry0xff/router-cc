import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { routingApi, type IntelligentRoutingSettings } from "@/lib/api/routing";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";

export function useRoutingSettings(appType: string) {
  return useQuery({
    queryKey: ["routing_settings", appType],
    queryFn: () => routingApi.getSettings(appType),
  });
}

export function useUpdateRoutingSettings(appType: string) {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: (settings: IntelligentRoutingSettings) =>
      routingApi.updateSettings(appType, settings),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: ["routing_settings", appType],
      });
      toast.success(
        t("routing.saved", { defaultValue: "智能路由设置已保存" }),
        { closeButton: true },
      );
    },
    onError: (error) => {
      toast.error(
        t("routing.saveFailed", {
          defaultValue: "保存失败",
          error: String(error),
        }),
      );
    },
  });
}
