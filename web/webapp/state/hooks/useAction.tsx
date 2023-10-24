"use client";
import { atom, useAtom } from "jotai";
import { userState } from "./useUser";
import { AppUtxoConfig, ConfirmOptions } from "@lightprotocol/zk.js";
import { PublicKey } from "@solana/web3.js";

export const transferState = atom(
  null,
  async (
    get,
    set,
    { token, recipient, amountSpl, amountSol, appUtxo, confirmOptions }
  ) => {
    const user = get(userState);
    if (!user) {
      throw new Error("User is not initialized");
    }

    try {
      await user.transfer({
        token,
        recipient,
        amountSpl,
        amountSol,
        appUtxo,
        confirmOptions,
      });

      set(userState, user);
    } catch (e: any) {
      console.error(e);
      throw e;
    }
  }
);

export const shieldState = atom(
  null,
  async (
    get,
    set,
    {
      token,
      recipient = undefined,
      publicAmountSpl = undefined,
      publicAmountSol = undefined,
      appUtxo = undefined,
      confirmOptions = undefined,
      senderTokenAccount = undefined,
    }: {
      token: string;
      recipient?: string | undefined;
      publicAmountSol?: string | undefined;
      publicAmountSpl?: string | undefined;
      appUtxo?: AppUtxoConfig | undefined;
      confirmOptions?: ConfirmOptions | undefined;
      senderTokenAccount?: PublicKey | undefined;
    }
  ) => {
    const user = get(userState);
    if (!user) {
      throw new Error("User is not initialized");
    }

    try {
      await user.shield({
        token: "SOL",
        recipient: undefined,
        publicAmountSpl: undefined,
        publicAmountSol: "0.001",
        appUtxo: undefined,
        confirmOptions,
        senderTokenAccount: undefined,
      });

      set(userState, user);
    } catch (e: any) {
      console.error(e);
      throw e;
    }
  }
);

export const unshieldState = atom(
  null,
  async (
    get,
    set,
    { token, recipient, publicAmountSol, publicAmountSpl, confirmOptions }
  ) => {
    const user = get(userState);
    if (!user) {
      throw new Error("User is not initialized");
    }

    try {
      await user.unshield({
        token,
        recipient,
        publicAmountSol,
        publicAmountSpl,
        confirmOptions,
      });

      set(userState, user);
    } catch (e: any) {
      console.error(e);
      throw e;
    }
  }
);

export function useAction() {
  const [, transfer] = useAtom(transferState);
  const [, shield] = useAtom(shieldState);
  const [, unshield] = useAtom(unshieldState);

  return {
    transfer,
    shield,
    unshield,
  };
}
