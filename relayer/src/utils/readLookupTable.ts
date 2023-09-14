import { PublicKey } from "@solana/web3.js";
import { getLookUpTableVar } from "../config";
export const readLookupTable = () => {
  console.log("reading _LOOK_UP_TABLE var...", getLookUpTableVar());

  return new PublicKey(getLookUpTableVar()!);
};
