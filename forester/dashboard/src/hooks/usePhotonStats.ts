import useSWR from "swr";
import type { PhotonStats } from "@/types/forester";

async function fetchPhotonStats(): Promise<PhotonStats> {
  const res = await fetch("/api/photon-stats", { cache: "no-store" });
  if (!res.ok) {
    const body = await res.json().catch(() => ({}));
    throw new Error(body.error || `HTTP ${res.status}`);
  }
  return res.json();
}

export function usePhotonStats() {
  return useSWR<PhotonStats>("photon-stats", fetchPhotonStats, {
    refreshInterval: 30_000,
    revalidateOnFocus: true,
  });
}
