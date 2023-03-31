import { PublicKey } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";

export type TokenContext = {
  symbol: string;
  decimals: BN;
  tokenAccount: PublicKey;
  isNft: boolean;
  isSol: boolean;
};
