"use client";
import { atom, useAtom } from "jotai";
import { userState } from "./useUser";

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
      recipient,
      publicAmountSpl,
      publicAmountSol,
      appUtxo,
      confirmOptions,
    }
  ) => {
    const user = get(userState);
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
