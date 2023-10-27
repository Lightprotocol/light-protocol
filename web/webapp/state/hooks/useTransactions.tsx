"use client";
import { atom, useAtom } from "jotai";
import { userState } from "./useUser";
import { UserIndexedTransaction } from "@lightprotocol/zk.js";
import { useSync } from "./useSync";

export const transactionHistoryState = atom<
  UserIndexedTransaction[] | undefined
>((get) => get(userState)?.user?.transactionHistory);

export function useTransactions() {
  const [transactions] = useAtom(transactionHistoryState);
  const syncState = useSync();

  return {
    ...syncState,
    transactions,
  };
}
