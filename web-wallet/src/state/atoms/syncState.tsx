"use client";
import { MerkleTreeConfig } from "@lightprotocol/zk.js";
import { atom } from "jotai";
import { userState } from "../hooks/useUser";

export const syncLoadingState = atom<boolean>(false);
export const syncErrorState = atom<Error | null>(null);

export const syncedState = atom(
  (get) => get(userState),
  async (get, set) => {
    const user = get(userState);
    if (!user) {
      throw new Error("User is not initialized");
    }

    set(syncLoadingState, true);
    set(syncErrorState, null);

    try {
      /// fetch all UTXOs in one go.
      await user.syncState(
        true,
        user.balance,
        MerkleTreeConfig.getTransactionMerkleTreePda()
      );
      await user.syncState(
        false,
        user.inboxBalance,
        MerkleTreeConfig.getTransactionMerkleTreePda()
      );

      set(userState, user);
      set(syncLoadingState, false);
    } catch (e: any) {
      set(syncErrorState, e);
      set(syncLoadingState, false);
    }
  }
);
