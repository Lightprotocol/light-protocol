"use client";
import { atom, useAtom } from "jotai";
import { userState } from "./useUser";
import { AppUtxoConfig, ConfirmOptions, sleep } from "@lightprotocol/zk.js";
import { PublicKey } from "@solana/web3.js";
import { notifications } from "@mantine/notifications";
import { modals } from "@mantine/modals";

/// This assumes that we're never running two actions at
/// the same time. otherwise it will throw a race condition.
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
    actionNotification(`Sending ${token}`);

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

      /// FIX: in indexer, should use webhook to stream new txs in real time.
      /// sleep prevents fetching non-up2date txs state
      /// Same for shield, unshield
      await sleep(5000);

      set(userState, { user, timestamp: Date.now() });
      actionNotification(`Transfer successful`, NotifType.success, 3000);
    } catch (e: any) {
      console.error("transferState error:", e);
      throw e;
    } finally {
      set(transferLoadingState, false);
      delayedModalClose();
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
    actionNotification(`Shielding ${token}`);

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

      await sleep(8000);

      // FIX: the user class doesnt shallow update after shields, therefore we have to "force update" the user here
      // fix this by removing the user class and managing balance/history state manually. this makes it more predictable.
      set(userState, { user, timestamp: Date.now() });
      actionNotification(`Shield successful`, NotifType.success, 3000);
    } catch (e: any) {
      console.error(e);
      throw e;
    } finally {
      set(shieldLoadingState, false);
      delayedModalClose();
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
    actionNotification(`Unshielding ${token}`);

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

      await sleep(8000);

      set(userState, { user, timestamp: Date.now() });
      actionNotification(`Unshield successful`, NotifType.success, 3000);
    } catch (e: any) {
      console.error(e);
      throw e;
    } finally {
      set(unshieldLoadingState, false);
      delayedModalClose();
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

enum NotifType {
  success = "green",
  info = "blue",
  error = "red",
}

const actionNotification = (
  title: string,
  type?: NotifType,
  duration?: number
) => {
  notifications.show({
    title,
    message: "",
    color: type || NotifType.info,
    autoClose: duration || 5000,
  });
};

const delayedModalClose = (delay?: number) => {
  window.setTimeout(() => {
    modals.closeAll();
  }, delay || 1000);
};
