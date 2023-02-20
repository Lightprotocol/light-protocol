import { BN } from "@coral-xyz/anchor";
import { confirmConfig, merkleTreeProgram, MERKLE_TREE_KEY } from "./constants";
import { Connection, PublicKey, SystemProgram } from "@solana/web3.js";
import { MerkleTreeConfig, SolMerkleTree } from "./merkleTree";
import { MINT } from "./test-utils/constants_system_verifier";
import * as anchor from "@coral-xyz/anchor";
import { initLookUpTableFromFile, setUpMerkleTree } from "./test-utils/index";
const { keccak_256 } = require("@noble/hashes/sha3");
const circomlibjs = require("circomlibjs");

export function hashAndTruncateToCircuit(data: Uint8Array) {
  return new BN(
    keccak_256
      .create({ dkLen: 32 })
      .update(Buffer.from(data))
      .digest()
      .slice(1, 32),
    undefined,
    "be",
  );
}

// TODO: add pooltype
export async function getAssetLookUpId({
  connection,
  asset,
}: {
  asset: PublicKey;
  connection: Connection;
  // poolType?: Uint8Array
}): Promise<any> {
  let poolType = new Uint8Array(32).fill(0);
  let mtConf = new MerkleTreeConfig({
    connection,
    merkleTreePubkey: MERKLE_TREE_KEY,
  });
  let pubkey = await mtConf.getSplPoolPda(asset, poolType);

  let registeredAssets =
    await mtConf.merkleTreeProgram.account.registeredAssetPool.fetch(
      pubkey.pda,
    );
  return registeredAssets.index;
}

// TODO: fetch from chain
// TODO: separate testing variables from prod env
export const assetLookupTable: string[] = [
  SystemProgram.programId.toBase58(),
  MINT.toBase58(),
];

export function getAssetIndex(assetPubkey: PublicKey): BN {
  return new BN(assetLookupTable.indexOf(assetPubkey.toBase58()));
}

export function fetchAssetByIdLookUp(assetIndex: BN): PublicKey {
  return new PublicKey(assetLookupTable[assetIndex.toNumber()]);
}

export const arrToStr = (uint8arr: Uint8Array) =>
  "LPx" + Buffer.from(uint8arr.buffer).toString("hex");

export const strToArr = (str: string) =>
  new Uint8Array(Buffer.from(str.slice(3), "hex"));

// export var logger = (function () {
//   var oldConsoleLog: any = null;
//   var pub = {};

//   //@ts-ignore
//   pub.enableLogger = function enableLogger() {
//     if (oldConsoleLog == null) return;

//     console.log = oldConsoleLog;
//   };

//   //@ts-ignore
//   pub.disableLogger = function disableLogger() {
//     oldConsoleLog = console.log;
//     window["console"]["log"] = function () {};
//   };

//   return pub;
// })();
