import { BN } from "@coral-xyz/anchor";

export type IndexedTransactionData = {
  publicAmountSpl: Uint8Array;
  publicAmountSol: Uint8Array;
  leaves: BN[];
  encryptedUtxos: any[];
  nullifiers: any[];
  relayerFee: BN;
};

export type IndexedTransactionDecodedData = {
  data: IndexedTransactionData;
};
