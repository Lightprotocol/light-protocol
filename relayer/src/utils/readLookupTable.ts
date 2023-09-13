import { PublicKey } from "@solana/web3.js";
export const readLookupTable = () => new PublicKey(process.env.LOOK_UP_TABLE!);
