import { Utxo } from "utxo";

export type CachedUserState = {
  utxos: Utxo[];
  seed: string;
};
