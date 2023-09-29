import { atom } from "jotai";
import { getTimeGroups, getUserTransactions } from "../util/transactionHistory";
import { userUtxosAtom } from "./userUtxoAtoms";
import axios from "axios";
import {
  MERKLE_TREE_PDA_PUBKEY,
  MERKLE_TREE_PDA_PUBKEY_USDC,
  PROGRAM_ID,
  RELAYER_URL,
  RPC_GPA,
  Token,
} from "../constants";
import { leafAccountToBytes } from "../util/helpers";
import { LeafAccount } from "../util/fetchUserUtxos";
import { toFixedHex } from "../sdk/src/utils";
import { Connection, PublicKey } from "@solana/web3.js";

export type LightTransaction = {
  amount: number;
  blockTime: number;
  slot: number;
  signer: string;
  signature: string;
  to: string;
  from: string;
  type: string;
  owner: string;
  accountKeys: string[];
  leaves: {
    pda: string;
    commitments: string[];
    data: number[];
    index?: string;
  };
  token: string;
  nullifiers: string[];
};

export const transactionsAtom = atom<LightTransaction[]>([]);

export const fetchedTransactionsAtom = atom(
  (get) => get(transactionsAtom),
  async (get, set) => {
    let res = await axios.get(`${RELAYER_URL}/history`);
    if (!res.data) {
      console.log("Could not fetch recent tx");
      return;
    }
    let fetchedTransactions: LightTransaction[] = res.data.transactions;
    console.log(
      "fetchedTransactions last: ",
      fetchedTransactions[fetchedTransactions.length - 1],
    );

    let filteredTxs = fetchedTransactions.filter((tx) => {
      return tx.blockTime > 0;
    });
    // TODO: add merkletree root check to assert that the leaves data is complete && isComplete check for each transaction (if not, fetch the missing ones)
    // TODO implement cache (hash) to not re-set state if no new transactions
    set(transactionsAtom, filteredTxs);
  },
);

export const mostRecentTxBlockTimeAtom = atom(
  (get) => get(fetchedTransactionsAtom)[0]?.blockTime,
);

// derivedAtom for just the slots based on txs
export const slotsAndIndicesAtom = atom((get) => {
  const transactions = get(transactionsAtom);
  if (!transactions) {
    console.log("@@@@slotsAndIndicesAtom no transactions");
    return { usdc: [], sol: [] };
  }
  let [sol, usdc] = getSlotsAndIndices(transactions);
  return { usdc, sol };
});

function getSlotsAndIndices(transactions: LightTransaction[]): any[] {
  const usdcSlotsAndIndices = [];
  const solSlotsAndIndices = [];
  transactions.forEach((tx) => {
    if (!tx.slot || !tx.leaves || !tx.token || tx.leaves.index === null) return;
    let slot = tx.slot;
    let index = tx.leaves.index;
    let token = tx.token;
    if (token === Token.SOL) {
      solSlotsAndIndices.push({ slot, index });
    } else if (token === Token.USDC) {
      usdcSlotsAndIndices.push({ slot, index });
    }
  });
  return [solSlotsAndIndices, usdcSlotsAndIndices];
}

// leavesAtom
export const leavesAtom = atom({
  leavesSol: [],
  leavesUsdc: [],
  dedupedSortedLeafAccountBytesSol: [],
  dedupedSortedLeafAccountBytesUsdc: [],
});

// Turn this into a getLeavesAtom
export const fetchLeavesAtom = atom(
  (get) => get(fetchedTransactionsAtom),
  async (get, set) => {
    const {
      leavesSol,
      leavesUsdc,
      dedupedSortedLeafAccountBytesSol,
      dedupedSortedLeafAccountBytesUsdc,
    } = await parseAndSortLeavesWithGPA();

    set(leavesAtom, {
      leavesSol,
      leavesUsdc,
      dedupedSortedLeafAccountBytesSol,
      dedupedSortedLeafAccountBytesUsdc,
    });
  },
);
//
export const nullifierAtom = atom<string[]>([]);

export const fetchNullifiersAtom = atom(
  (get) => get(fetchedTransactionsAtom),
  async (get, set) => {
    const RPC_connection = new Connection(RPC_GPA); // config
    var nullifierAccounts = await RPC_connection.getProgramAccounts(
      PROGRAM_ID,
      {
        commitment: "processed",
        filters: [{ dataSize: 2 }],
      },
    );
    let nullifierPubkeys = [];
    nullifierAccounts.map((acc) =>
      nullifierPubkeys.push(acc.pubkey.toBase58()),
    );
    set(nullifierAtom, nullifierPubkeys);
  },
);

async function parseAndSortLeavesWithGPA(): Promise<{
  leavesSol: string[];
  leavesUsdc: string[];
  sortedLeafAccountBytesSol: LeafAccount[];
  sortedLeafAccountBytesUsdc: LeafAccount[];
  dedupedSortedLeafAccountBytesSol: LeafAccount[];
  dedupedSortedLeafAccountBytesUsdc: LeafAccount[];
}> {
  let sortedLeafAccountBytesSol = [];
  let sortedLeafAccountBytesUsdc = [];
  if (!RPC_GPA) throw new Error("RPC_GPA undefined");
  const RPC_connection = new Connection(RPC_GPA); // config
  var leafAccounts = await RPC_connection.getProgramAccounts(PROGRAM_ID, {
    filters: [{ dataSize: 106 + 222 }],
    commitment: "confirmed",
  });
  await Promise.all(
    leafAccounts.map(async (acc) => {
      let leafAccountBytes: LeafAccount = leafAccountToBytes(acc, null);
      const token = leafAccountBytes.merkletreePubkey.publicKey.toBase58();
      if (token === MERKLE_TREE_PDA_PUBKEY_USDC.toBase58()) {
        sortedLeafAccountBytesUsdc.push(leafAccountBytes);
      }
      if (token === MERKLE_TREE_PDA_PUBKEY.toBase58()) {
        sortedLeafAccountBytesSol.push(leafAccountBytes);
      }
    }),
  );

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

  console.log(
    "dedupedSortedLeafAccountBytesUsdc",
    dedupedSortedLeafAccountBytesUsdc.length,
  );
  console.log(
    "dedupedSortedLeafAccountBytesSol",
    dedupedSortedLeafAccountBytesSol.length,
  );
  return {
    leavesSol,
    leavesUsdc,
    sortedLeafAccountBytesSol,
    sortedLeafAccountBytesUsdc,
    dedupedSortedLeafAccountBytesSol,
    dedupedSortedLeafAccountBytesUsdc,
  };
}

// Call only after fetchedTransactionsAtom is called
export const userTransactionsAtom = atom((get) => {
  let userUtxos = get(userUtxosAtom);
  const transactions = get(transactionsAtom);
  console.log("userutxos", userUtxos);
  const userTransactions = getUserTransactions({
    spentUtxos: userUtxos, //.filter((u) => u.spent === true),
    transactions,
  });
  //TODO: add cache to run this only if changes.
  return userTransactions;
});

export const groupedTransactionsAtom = atom((get) => {
  let userTransactions = get(userTransactionsAtom);
  console.log("userTransactions", userTransactions);
  if (userTransactions.length === 0) return [];
  let {
    last60Minutes,
    last30DaysByDay,
    last24HoursByHour,
    lastYearByMonth,
    older,
  } = getTimeGroups(userTransactions, Date.now()); // TODO: change time
  return [
    { name: "last60Minutes", transactions: last60Minutes },
    { name: "last24HoursByHour", transactions: last24HoursByHour }, //24
    { name: "last30DaysByDay", transactions: last30DaysByDay }, // 30
    { name: "lastYearByMonth", transactions: lastYearByMonth }, // 12
    { name: "older", transactions: older },
  ];
});

// derive shieldsAtom from transactionsAtom
export const shieldsAtom = atom((get) => {
  let transactions = get(transactionsAtom);
  let shields = transactions.filter((tx) => tx.type === "shield");
  return shields;
});

// derive unshieldsAtom from transactionsAtom
export const unshieldsAtom = atom((get) => {
  let transactions = get(transactionsAtom);
  let unshields = transactions.filter((tx) => tx.type === "unshield");
  return unshields;
});
