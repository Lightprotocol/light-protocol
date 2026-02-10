import useSWR from "swr";
import { fetcher } from "@/lib/api";
import type { ForesterStatus } from "@/types/forester";

export function useForesterStatus() {
  return useSWR<ForesterStatus>("/status", fetcher, {
    refreshInterval: (data) => {
      if (data?.slots_until_next_light_slot) {
        const ms = data.slots_until_next_light_slot * 0.46 * 1000 + 500;
        return Math.max(2000, Math.min(ms, 10000));
      }
      return 10000;
    },
    revalidateOnFocus: true,
  });
}
