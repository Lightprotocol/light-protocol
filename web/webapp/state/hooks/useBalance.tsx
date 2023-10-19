"use client";
import { atom, useAtom } from "jotai";
import {
  syncedState,
  syncLoadingState,
  syncErrorState,
} from "../atoms/syncState";
import { userState } from "./useUser";

export const utxosState = atom((get) => get(userState)?.getUtxoInbox);
export const balanceState = atom(
  (get) => get(userState)?.balance.tokenBalances
);
export function useBalance() {
  const [inboxBalance] = useAtom(utxosState);
  const [balance] = useAtom(balanceState);

  const [, sync] = useAtom(syncedState);
  const [isSyncing] = useAtom(syncLoadingState);
  const [syncError] = useAtom(syncErrorState);

  return {
    sync,
    isSyncing,
    syncError,
    inboxBalance,
    balance,
  };
}
