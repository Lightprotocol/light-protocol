import useSWR from "swr";
import { fetcher } from "@/lib/api";
import type { MetricsResponse } from "@/types/forester";

export function useMetrics() {
  return useSWR<MetricsResponse>("/metrics/json", fetcher, {
    refreshInterval: 10000,
    revalidateOnFocus: true,
  });
}
