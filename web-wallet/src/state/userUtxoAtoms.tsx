import { atom } from "jotai";
import { LeafAccount, fetchUserUtxos } from "../util/fetchUserUtxos";
import { userAtom, userCreationBlockTimeAtom } from "./userAtoms";
import { Utxo } from "../sdk/src/utxo";
import { storeIndicesInLocalStorage } from "../util/persistIndices";
import {
  LightTransaction,
  leavesAtom,
  nullifierAtom,
  slotsAndIndicesAtom,
  transactionsAtom,
} from "./transactionsAtoms";
import { Token } from "../constants";
import { toFixedHex } from "../sdk/src/utils";

export type UserUtxo = {
  utxo: Utxo;
  spent: boolean;
  token: string;
};

export type ActiveFeeConfig = {
  TOTAL_FEES_WITHDRAWAL: number;
  MINIMUM_SHIELD_AMOUNT: number;
  MAXIMUM_SHIELD_AMOUNT: number;
  TOTAL_FEES_DEPOSIT: number;
  DEPOSIT_COLLATERAL: number;
  DECIMALS: number;
};

export const utxoSyncAtom = atom<boolean>(true);

export const userUtxosAtom = atom<UserUtxo[]>([]);

export const leafBytesToHex = (leafBytes: Uint8Array) => {
  return toFixedHex(leafBytes.slice(0, 32).reverse());
};

function parseLeavesFromTransactions(transactions: LightTransaction[]): {
  leavesSol: string[];
  leavesUsdc: string[];
  sortedLeafAccountBytesSol: LeafAccount[];
  sortedLeafAccountBytesUsdc: LeafAccount[];
  dedupedSortedLeafAccountBytesSol: LeafAccount[];
  dedupedSortedLeafAccountBytesUsdc: LeafAccount[];
} {
  console.time("parseLeavesFromTransactions");
  let sortedLeafAccountBytesSol = [];
  let sortedLeafAccountBytesUsdc = [];
  transactions.map((tx) => {
    if (!tx.leaves.data) {
      // console.log("no leaves data", tx);
    } else {
      const mockAcc = {
        account: {
          data: tx.leaves.data,
        },
      };

      //@ts-ignore
      let leafAccountBytes: LeafAccount = leafAccountToBytes(mockAcc, tx.slot);
      if (tx.token === Token.SOL) {
        sortedLeafAccountBytesSol.push(leafAccountBytes);
      }
      if (tx.token === Token.USDC) {
        sortedLeafAccountBytesUsdc.push(leafAccountBytes);
      }
    }
  });

  var leavesSol: string[] = [];
  sortedLeafAccountBytesSol.sort(
    (a, b) => parseFloat(a.index.u64) - parseFloat(b.index.u64),
  );
  for (var i = 0; i < sortedLeafAccountBytesSol.length; i++) {
    leavesSol.push(
      toFixedHex(sortedLeafAccountBytesSol[i].leaves.left.reverse()),
    );
    leavesSol.push(
      toFixedHex(sortedLeafAccountBytesSol[i].leaves.right.reverse()),
    );
  }

  var leavesUsdc: string[] = [];
  sortedLeafAccountBytesUsdc.sort(
    (a, b) => parseFloat(a.index.u64) - parseFloat(b.index.u64),
  );
  for (var i = 0; i < sortedLeafAccountBytesUsdc.length; i++) {
    leavesUsdc.push(
      toFixedHex(sortedLeafAccountBytesUsdc[i].leaves.left.reverse()),
    );
    leavesUsdc.push(
      toFixedHex(sortedLeafAccountBytesUsdc[i].leaves.right.reverse()),
    );
  }

  let dedupedSortedLeafAccountBytesSol = Array.from(
    new Set(sortedLeafAccountBytesSol),
  );
  let dedupedSortedLeafAccountBytesUsdc = Array.from(
    new Set(sortedLeafAccountBytesUsdc),
  );

  console.timeEnd("parseLeavesFromTransactions");
  return {
    leavesSol,
    leavesUsdc,
    sortedLeafAccountBytesSol,
    sortedLeafAccountBytesUsdc,
    dedupedSortedLeafAccountBytesSol,
    dedupedSortedLeafAccountBytesUsdc,
  };
}

export const fetchedUserUtxosAtom = atom(
  (get) => get(userUtxosAtom),
  async (get, set) => {
    set(utxoSyncAtom, true);
    let user = get(userAtom);
    let {
      dedupedSortedLeafAccountBytesSol,
      dedupedSortedLeafAccountBytesUsdc,
      leavesSol,
      leavesUsdc,
    } = get(leavesAtom);
    let { usdc: usdcSlotsAndIndices, sol: solSlotsAndIndices } =
      get(slotsAndIndicesAtom);
    let nullifiers = get(nullifierAtom);
    let createdAtSlot = get(userCreationBlockTimeAtom).slot;

    if (
      !user ||
      !leavesSol ||
      !leavesUsdc ||
      !dedupedSortedLeafAccountBytesSol ||
      !dedupedSortedLeafAccountBytesUsdc ||
      !nullifiers
    ) {
      set(userUtxosAtom, []);
      set(utxoSyncAtom, false);
      console.log("Could not fetch user utxos");
      return;
    }

    /** initializes background thread which decrypts and filters utxos */
    const userUtxos: UserUtxo[] = await fetchUserUtxos({
      user,
      dedupedLeafAccountsSol: dedupedSortedLeafAccountBytesSol,
      dedupedLeafAccountsUsdc: dedupedSortedLeafAccountBytesUsdc,
      leavesSol,
      leavesUsdc,
      nullifiers,
      createdAtSlot,
      usdcSlotsAndIndices,
      solSlotsAndIndices,
    });
    // TODO: only do this when it changes
    storeIndicesInLocalStorage(
      user.keypairs.localStorageEncryptionKeypair,
      user.connectedWallet,
      userUtxos,
    );

    set(userUtxosAtom, userUtxos);
    set(utxoSyncAtom, false);
  },
);

// derive is isUtxoSyncCompleteAtom from utxoSyncAtom
export const isUtxoSyncCompleteAtom = atom((get) => {
  let utxoSync = get(utxoSyncAtom);
  return !utxoSync;
});

// derive spentUtxosAtom from userUtxosAtom
export const spentUtxosAtom = atom((get) => {
  let userUtxos = get(userUtxosAtom);
  return userUtxos.filter((u) => u.spent === true).map((u) => u.utxo);
});

export const devalidatedUserUtxosAtom = atom(
  (get) => get(userUtxosAtom),
  async (get, set, params: { inUtxos: Utxo[]; outUtxo: UserUtxo }) => {
    let { inUtxos, outUtxo } = params;
    let currentUserUtxos = get(userUtxosAtom);
    const restUtxos = invalidateUserUtxos(currentUserUtxos, inUtxos, outUtxo);
    set(userUtxosAtom, restUtxos);
  },
);

export function invalidateUserUtxos(
  currentUserUtxos: UserUtxo[],
  inUtxos: Utxo[],
  outUtxo: UserUtxo,
) {
  // remove inutxos
  let restUtxos = currentUserUtxos.filter((userUtxo) => {
    let found = false;
    for (let note of inUtxos) {
      if (note.getCommitment()?._hex === userUtxo.utxo.getCommitment()?._hex) {
        found = true;
        break;
      }
    }
    return !found;
  });

  restUtxos.push(outUtxo);
  return restUtxos;
}
