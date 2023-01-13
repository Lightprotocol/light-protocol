import { BN } from "@coral-xyz/anchor";
import { merkleTreeProgram, MERKLE_TREE_KEY } from "./constants";
import { Connection, PublicKey, SystemProgram } from "@solana/web3.js";
import { MerkleTreeConfig } from "./merkleTree";
import { MINT } from "./test-utils/constants_system_verifier";
const { keccak_256 } = require("@noble/hashes/sha3");

export function hashAndTruncateToCircuit(data: Uint8Array) {
  return new BN(
    keccak_256
      .create({ dkLen: 32 })
      .update(Buffer.from(data))
      .digest()
      .slice(1, 32),
    undefined,
    "be"
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
  let pubkey = await mtConf.getSplPoolPda(poolType, asset);

  let registeredAssets =
    await mtConf.merkleTreeProgram.account.registeredAssetPool.fetch(
      pubkey.pda
    );
  return registeredAssets.index;
}

// TODO: fetch from chain
// TODO: separate testing variables from prod env
export const assetLookupTable: PublicKey[] = [SystemProgram.programId, MINT];

export function getAssetIndex(assetPubkey: PublicKey): BN {
  return new BN(assetLookupTable.indexOf(assetPubkey));
}

export function fetchAssetByIdLookUp(assetIndex: BN): PublicKey {
  console.log("assetIndex ", assetIndex);

  return assetLookupTable[assetIndex.toNumber()];
}
