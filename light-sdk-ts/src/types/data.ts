import BN from "bn.js";

export type Data = {
  publicAmountSpl: Uint8Array;
  publicAmountSol: Uint8Array;
  leaves: BN[];
  encryptedUtxos: any[];
  nullifiers: any[];
  relayerFee: BN;
};

export type DecodedData = {
  data: Data;
};
