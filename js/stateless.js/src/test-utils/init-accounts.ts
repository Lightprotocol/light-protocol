// TODO: remove!
import { Keypair } from "@solana/web3.js";

export function byteArrayToKeypair(byteArray: number[]): Keypair {
  return Keypair.fromSecretKey(Uint8Array.from(byteArray));
}
export const PAYER_KEYPAIR = byteArrayToKeypair([
  17, 34, 231, 31, 83, 147, 93, 173, 61, 164, 25, 0, 204, 82, 234, 91, 202, 187,
  228, 110, 146, 97, 112, 131, 180, 164, 96, 220, 57, 207, 65, 107, 2, 99, 226,
  251, 88, 66, 92, 33, 25, 216, 211, 185, 112, 203, 212, 238, 105, 144, 72, 121,
  176, 253, 106, 168, 115, 158, 154, 188, 62, 255, 166, 81,
]);
