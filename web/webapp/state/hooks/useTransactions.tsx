"use client";
import { atom, useAtom } from "jotai";
import { userState } from "./useUser";
import { UserIndexedTransaction } from "@lightprotocol/zk.js";
import {
  syncErrorState,
  syncLoadingState,
  syncedState,
} from "../atoms/syncState";

export const transactionHistoryState = atom<
  UserIndexedTransaction[] | undefined
>((get) => get(userState)?.transactionHistory);

export function useTransactions() {
  const [transactions] = useAtom(transactionHistoryState);

  const [, sync] = useAtom(syncedState);
  const [isSyncing] = useAtom(syncLoadingState);
  const [syncError] = useAtom(syncErrorState);

  return {
    transactions,
    sync,
    isSyncing,
    syncError,
  };
}
