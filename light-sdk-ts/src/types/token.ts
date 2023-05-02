import { PublicKey } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";

export type TokenData = {
  symbol: string;
  decimals: BN;
  isNft: boolean;
  isNative: boolean;
  mint: PublicKey;
};
