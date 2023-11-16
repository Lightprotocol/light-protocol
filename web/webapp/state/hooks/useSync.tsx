import { useAtom } from "jotai";
import {
  syncErrorState,
  syncLoadingState,
  syncedState,
} from "../atoms/syncState";

export function useSync() {
  const [, sync] = useAtom(syncedState);
  const [isSyncing] = useAtom(syncLoadingState);
  const [syncError] = useAtom(syncErrorState);

  return {
    sync,
    isSyncing,
    syncError,
  };
}
