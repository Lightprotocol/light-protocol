import { BN } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { Utxo } from "../utxo";

/**
 * We keep spent UTXOs in a separate type,
 * because we need to keep Balance up2date at
 * any time, and syncing spent UTXOs is expensive.
 */
// TODO: add history (spentutxos)
export type Balance = {
  // key is token
  // includes only unspent UTXOs
  tokenBalances: Map<string, TokenBalance>;
  // TODO: add programBalances
};

export type TokenBalance = {
  amount: BN;
  lamports: BN; // rent
  data: TokenData;
  utxos: Utxo[];
};

// from ctx
export type TokenData = {
  symbol: string;
  decimals: BN;
  isNft: boolean;
  isNative: boolean;
  mint: PublicKey;
};

export type SerializedTokenBalance = {
  mint: string;
  utxos: { utxo: string; index?: number }[];
};
