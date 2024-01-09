"use client";
import {
  Provider,
  Rpc,
  User,
  Wallet,
  confirmConfig,
} from "@lightprotocol/zk.js";
import { Connection, PublicKey } from "@solana/web3.js";
import { atom, useAtom } from "jotai";

export const userState = atom<{ user: User | null; timestamp: number }>({
  user: null,
  timestamp: Date.now(),
});
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
      const rpc = await Rpc.initFromUrl(process.env.NEXT_PUBLIC_LIGHT_RPC_URL!);

      const provider = await Provider.init({
        rpc,
        wallet,
        confirmConfig,
        url: connection.rpcEndpoint,
        versionedTransactionLookupTable: new PublicKey(
          process.env.NEXT_PUBLIC_LOOK_UP_TABLE!
        ),
      });
      const user = await User.init({ provider, skipFetchBalance: true });

      await user.getTransactionHistory(true);

      set(userState, { user, timestamp: Date.now() });

      set(loadingState, false);
    } catch (e: any) {
      console.error(e);
      set(errorState, e);
      set(loadingState, false);
    }
  }
);

export function useUser() {
  const [{ user }] = useAtom(userState);
  const [, initUser] = useAtom(initializedUser);
  const [isLoading, setIniting] = useAtom(loadingState);
  const [error] = useAtom(errorState);

  return { user, initUser, isLoading, error, setIniting };
}
