import { PublicKey } from "@solana/web3.js";
import { getLookUpTableVar } from "../config";
export const readLookupTable = () => {
  console.log("reading LOOK_UP_TABLE object..", getLookUpTableVar());

  return new PublicKey(getLookUpTableVar()!);
};
