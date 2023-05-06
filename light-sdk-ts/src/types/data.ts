import { BN } from "@coral-xyz/anchor";

export type IndexedTransactionData = {
  publicAmountSpl: Uint8Array;
  publicAmountSol: Uint8Array;
  leaves: number[][];
  encryptedUtxos: any[];
  nullifiers: any[];
  relayerFee: BN;
  firstLeafIndex: BN;
  tx: any;
};

export type IndexedTransactionDecodedData = {
  data: IndexedTransactionData;
};
