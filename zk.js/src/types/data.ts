import { BN } from "@coral-xyz/anchor";

export type IndexedTransactionData = {
  publicAmountSpl: Uint8Array;
  publicAmountSol: Uint8Array;
  leaves: number[][];
  encryptedUtxos: number[];
  nullifiers: number[][];
  rpcFee: BN;
  firstLeafIndex: BN;
  tx: any;
  message: number[];
};

export type IndexedTransactionDecodedData = {
  data: IndexedTransactionData;
};
