import * as fs from "fs";
export const readLookupTable = () =>
  fs.readFileSync("./lookUpTable.txt", "utf8");
