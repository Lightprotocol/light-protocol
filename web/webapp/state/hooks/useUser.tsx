"use client";
import {
  ADMIN_AUTH_KEYPAIR,
  Provider,
  Relayer,
  TOKEN_ACCOUNT_FEE,
  User,
  Wallet,
  confirmConfig,
} from "@lightprotocol/zk.js";
import { Connection, PublicKey } from "@solana/web3.js";
import { atom, useAtom } from "jotai";
import { BN } from "@project-serum/anchor";

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
      console.log("init relayer from url");
      // const relayer = await Relayer.initFromUrl(
      //   "https://v3-devnet-relayer-7x44q.ondigitalocean.app"
      // );
      const relayer = new Relayer(
        new PublicKey("EkXDLi1APzu6oxJbg5Hnjb24kfKauJp1xCb5FAUMxf9D"),
        new PublicKey("AV3LnV78ezsEBZebNeMPtEcH1hmvSfUBC5Xbyrzqbt44"),
        new BN(10_000),
        TOKEN_ACCOUNT_FEE,
        "https://v3-devnet-relayer-7x44q.ondigitalocean.app"
      );

      console.log("relayer", relayer);
      console.log("conn", connection.rpcEndpoint);
      console.log("init provider");
      const provider = await Provider.init({
        relayer,
        wallet: ADMIN_AUTH_KEYPAIR,
        // connection,
        confirmConfig,
        url: connection.rpcEndpoint,
        versionedTransactionLookupTable: new PublicKey(
          "GF2TtYjrWsH9g12kHmm5KiDqt4MFqpzf7zoQoJGfVgfW"
        ),
      });
      console.log("user.init");
      const user = await User.init({ provider, skipFetchBalance: true });

      const history = await user.getTransactionHistory(true);
      const balance = await user.getBalance(false);
      const utxos = user.getAllUtxos();

      // console.log("history", history);
      // console.log("balance", balance);
      // console.log("utxos", utxos);

      set(userState, user);
      set(loadingState, false);
    } catch (e: any) {
      console.error(e);
      set(errorState, e);
      set(loadingState, false);
    }
  }
);

export function useUser() {
  const [user] = useAtom(userState);
  const [, initUser] = useAtom(initializedUser);
  const [isLoading, setIniting] = useAtom(loadingState);
  const [error] = useAtom(errorState);

  return { user, initUser, isLoading, error, setIniting };
}
