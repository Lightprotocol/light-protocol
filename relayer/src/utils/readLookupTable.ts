import { PublicKey } from "@solana/web3.js";
import { LOOK_UP_TABLE } from "../config";
// import fs from "fs";
export const readLookupTable = () => {
  //   let file = fs.readFileSync("./lookUpTable.txt", "utf8");
  console.log("reading LOOK_UP_TABLE object..", LOOK_UP_TABLE.LOOK_UP_TABLE);

  //   return new PublicKey(process.env.LOOK_UP_TABLE!);
  return new PublicKey(LOOK_UP_TABLE.LOOK_UP_TABLE!);
};
