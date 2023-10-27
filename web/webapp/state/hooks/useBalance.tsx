"use client";
import { atom, useAtom } from "jotai";
import { userState } from "./useUser";
import { useSync } from "./useSync";

export const utxosState = atom((get) => get(userState)?.user?.getUtxoInbox);
export const balanceState = atom(
  (get) => get(userState)?.user?.balance.tokenBalances
);
export function useBalance() {
  const [inboxBalance] = useAtom(utxosState);
  const [balance] = useAtom(balanceState);
  const syncState = useSync();

  return {
    ...syncState,
    inboxBalance,
    balance,
  };
}
