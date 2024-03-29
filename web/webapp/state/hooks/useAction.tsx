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
export const compressLoadingState = atom(false);
export const decompressLoadingState = atom(false);

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
    },
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
      /// Same for compress, decompress
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
  },
);

export const compressState = atom(
  null,
  async (
    get,
    set,
    {
      token,
      recipient = undefined,
      publicAmountSpl = undefined,
      publicAmountSol = undefined,
      confirmOptions = undefined,
      senderTokenAccount = undefined,
    }: {
      token: string;
      recipient?: string | undefined;
      publicAmountSol?: string | undefined;
      publicAmountSpl?: string | undefined;
      confirmOptions?: ConfirmOptions | undefined;
      senderTokenAccount?: PublicKey | undefined;
    },
  ) => {
    set(compressLoadingState, true);
    actionNotification(`Compressing ${token}`);
    const { user } = get(userState);
    if (!user) {
      throw new Error("User is not initialized");
    }
    try {
      await user.compress({
        token,
        recipient,
        publicAmountSpl,
        publicAmountSol,
        confirmOptions,
        senderTokenAccount,
      });

      /// FIX: in indexer, should use webhook to stream new txs in real time.
      /// sleep prevents fetching non-up2date txs state
      await sleep(8000);

      // FIX: the user class doesnt shallow update after compression, therefore we have to "force update" the user here
      // fix this by removing the user class and managing balance/history state manually. this makes it more predictable.
      set(userState, { user, timestamp: Date.now() });
      actionNotification(`Compress successful`, NotifType.success, 3000);
    } catch (e: any) {
      console.error(e);
      throw e;
    } finally {
      set(compressLoadingState, false);
      delayedModalClose();
    }
  },
);

export const decompressState = atom(
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
    },
  ) => {
    set(decompressLoadingState, true);
    actionNotification(`Decompressing ${token}`);

    const { user } = get(userState);
    if (!user) {
      throw new Error("User is not initialized");
    }

    try {
      await user.decompress({
        token,
        recipient,
        publicAmountSol,
        publicAmountSpl,
        confirmOptions,
      });

      /// FIX: in indexer, should use webhook to stream new txs in real time.
      /// sleep prevents fetching non-up2date txs state
      await sleep(8000);

      set(userState, { user, timestamp: Date.now() });
      actionNotification(`Decompress successful`, NotifType.success, 3000);
    } catch (e: any) {
      console.error(e);
      throw e;
    } finally {
      set(decompressLoadingState, false);
      delayedModalClose();
    }
  },
);

export function useAction() {
  const [, transfer] = useAtom(transferState);
  const [, compress] = useAtom(compressState);
  const [, decompress] = useAtom(decompressState);
  const [transferLoading] = useAtom(transferLoadingState);
  const [compressLoading] = useAtom(compressLoadingState);
  const [decompressLoading] = useAtom(decompressLoadingState);

  const loading = transferLoading || compressLoading || decompressLoading;

  return {
    transfer,
    compress,
    decompress,
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
  duration?: number,
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
