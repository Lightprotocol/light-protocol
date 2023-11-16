import { babyjubjubExt } from "./elgamal";
import { LookupTable } from "./pointEncoding";
const fs = require("fs");

const ProgressBar = require("cli-progress");
const bar = new ProgressBar.SingleBar({
  format:
    "Progress |" +
    "{bar}" +
    "| {percentage}% || {value}/{total} Chunks || Remaining time: {eta_formatted}",
  barCompleteChar: "\u2588",
  barIncompleteChar: "\u2591",
  hideCursor: true,
});

/**
 * Build a lookup table to break the EC discrete log for a 32-bit scalar
 * @param precomputeSize the size of the lookup table to be used --> 2**precomputeSize
 * @returns an object that contains 2**precomputeSize of keys and values
 */
export function precompute(precomputeSize: number, path: string) {
  const range = 32 - precomputeSize;
  const upperBound = BigInt(2) ** BigInt(precomputeSize);

  let lookupTable: LookupTable = {};
  let key: string;

  bar.start(Number(upperBound), 0);

  for (let xhi = BigInt(0); xhi < upperBound; xhi++) {
    key = babyjubjubExt.BASE.multiplyUnsafe(xhi * BigInt(2) ** BigInt(range))
      .toAffine()
      .x.toString();
    lookupTable[key] = xhi.toString(16);
    bar.update(Number(xhi) + 1);
  }
  bar.stop();

  fs.writeFileSync(
    path + `/lookupTableBBJub${precomputeSize}.json`,
    JSON.stringify(lookupTable),
  );
}
