import fsExtra from "fs-extra";
import fs from "fs";
export const indexDist = "node ../../dist/src/index.js";


export const fileExists = async (file: fsExtra.PathLike) => {
  return fs.promises
    .access(file, fs.constants.F_OK)
    .then(() => true)
    .catch(() => false);
};

export const sleep = (ms: number) => {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
