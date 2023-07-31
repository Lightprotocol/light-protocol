import * as fs from "fs";
export const readLookUpTableFromFile = () =>
  fs.readFileSync("./lookUpTable.txt", "utf8");
