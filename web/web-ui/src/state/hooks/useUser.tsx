"use client";
import { Provider, User, Wallet, confirmConfig } from "@lightprotocol/zk.js";
import { Connection } from "@solana/web3.js";
import { atom, useAtom } from "jotai";

export const userState = atom<User | null>(null);
export const loadingState = atom<boolean>(false);
export const errorState = atom<Error | null>(null);

export const initializedUser = atom(
  (get) => get(userState),
  async (
    _get,
    set,
    { connection, wallet }: { connection: Connection; wallet: Wallet }
  ) => {
    set(loadingState, true);

    try {
      const provider = await Provider.init({
        wallet,
        connection,
        confirmConfig,
      });
      const user = await User.init({ provider, skipFetchBalance: true });

      const history = await user.getTransactionHistory(true);
      const balance = await user.getBalance(false);
      const utxos = user.getAllUtxos();

      console.log("history", history);
      console.log("balance", balance);
      console.log("utxos", utxos);

      set(userState, user);
      set(loadingState, false);
    } catch (e: any) {
      set(errorState, e);
      set(loadingState, false);
    }
  }
);

export function useUser() {
  const [user] = useAtom(userState);
  const [, initUser] = useAtom(initializedUser);
  const [isLoading] = useAtom(loadingState);
  const [error] = useAtom(errorState);

  return { user, initUser, isLoading, error };
}
