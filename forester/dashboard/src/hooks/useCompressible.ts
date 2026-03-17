import useSWR from "swr";
import { fetcher } from "@/lib/api";
import type { CompressibleResponse } from "@/types/forester";

export function useCompressible() {
  return useSWR<CompressibleResponse>("/compressible", fetcher, {
    refreshInterval: 5000,
    revalidateOnFocus: true,
  });
}
