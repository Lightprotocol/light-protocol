"use client";
import { atom, useAtom } from "jotai";
import { userState } from "./useUser";
import { AppUtxoConfig, ConfirmOptions, User } from "@lightprotocol/zk.js";
import { PublicKey } from "@solana/web3.js";

export const transferLoadingState = atom(false);
export const shieldLoadingState = atom(false);
export const unshieldLoadingState = atom(false);

export const transferState = atom(
  null,
  async (
    get,
    set,
    {
      token,
      recipient,
      amountSpl,
      amountSol,
      appUtxo,
      confirmOptions,
    }: {
      token: string;
      recipient: string;
      amountSpl?: string;
      amountSol?: string;
      appUtxo?: AppUtxoConfig | undefined;
      confirmOptions?: any;
    }
  ) => {
    set(transferLoadingState, true);

    const { user } = get(userState);
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

      set(userState, { user, timestamp: Date.now() });
    } catch (e: any) {
      console.error("transferState error:", e);
      throw e;
    } finally {
      set(transferLoadingState, false);
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
    set(shieldLoadingState, true);
    const { user } = get(userState);
    if (!user) {
      throw new Error("User is not initialized");
    }

    try {
      await user.shield({
        token,
        recipient,
        publicAmountSpl,
        publicAmountSol,
        appUtxo,
        confirmOptions,
        senderTokenAccount,
      });

      // FIX: the user class doesnt shallow update after shields, therefore we have to "force update" the user here
      // fix this by removing the user class and managing balance/history state manually. this makes it more predictable.
      set(userState, { user, timestamp: Date.now() });
    } catch (e: any) {
      console.error(e);
      throw e;
    } finally {
      set(shieldLoadingState, false);
    }
  }
);

export const unshieldState = atom(
  null,
  async (
    get,
    set,
    {
      token,
      recipient,
      publicAmountSpl = undefined,
      publicAmountSol = undefined,
      confirmOptions = undefined,
    }: {
      token: string;
      recipient: PublicKey;
      publicAmountSol?: string | undefined;
      publicAmountSpl?: string | undefined;
      confirmOptions?: ConfirmOptions | undefined;
    }
  ) => {
    set(unshieldLoadingState, true);
    const { user } = get(userState);
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

      set(userState, { user, timestamp: Date.now() });
    } catch (e: any) {
      console.error(e);
      throw e;
    } finally {
      set(unshieldLoadingState, false);
    }
  }
);

export function useAction() {
  const [, transfer] = useAtom(transferState);
  const [, shield] = useAtom(shieldState);
  const [, unshield] = useAtom(unshieldState);
  const [transferLoading] = useAtom(transferLoadingState);
  const [shieldLoading] = useAtom(shieldLoadingState);
  const [unshieldLoading] = useAtom(unshieldLoadingState);

  const loading = transferLoading || shieldLoading || unshieldLoading;

  return {
    transfer,
    shield,
    unshield,
    loading,
  };
}
