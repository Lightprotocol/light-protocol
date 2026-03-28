import type { PublicKey } from "@solana/web3.js";
import type { SplInterface } from "./spl-interface";

export interface LoadOptions {
  splInterfaces?: SplInterface[];
  wrap?: boolean;
  delegatePubkey?: PublicKey;
}
