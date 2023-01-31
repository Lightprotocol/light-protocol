import { BN } from "@coral-xyz/anchor";
import { confirmConfig, merkleTreeProgram, MERKLE_TREE_KEY } from "./constants";
import { Connection, PublicKey, SystemProgram } from "@solana/web3.js";
import { MerkleTreeConfig, SolMerkleTree } from "./merkleTree";
import { MINT } from "./test-utils/constants_system_verifier";
import { LightInstance } from "transaction";
import * as anchor from "@coral-xyz/anchor";
import { initLookUpTableFromFile, setUpMerkleTree } from "./test-utils/index";
const { keccak_256 } = require("@noble/hashes/sha3");
const circomlibjs = require("circomlibjs");

export async function getLightInstance() {
  const poseidon = await circomlibjs.buildPoseidonOpt();
  anchor.setProvider(anchor.AnchorProvider.env());
  const provider = anchor.AnchorProvider.local(
    "http://127.0.0.1:8899",
    confirmConfig,
  );
  // get hc value
  const LOOK_UP_TABLE = await initLookUpTableFromFile(provider);

  // TODO: move to a seperate function
  const merkletreeIsInited = await provider.connection.getAccountInfo(
    MERKLE_TREE_KEY,
  );
  if (!merkletreeIsInited) {
    await setUpMerkleTree(provider);
    console.log("merkletree inited");
  }
  let mt = new SolMerkleTree({
    pubkey: MERKLE_TREE_KEY,
    poseidon: poseidon,
  });
  console.log("building merkletree...");
  mt = await SolMerkleTree.build({
    pubkey: MERKLE_TREE_KEY,
    poseidon: poseidon,
  });
  console.log("building merkletree done");
  const lightInstance: LightInstance = {
    solMerkleTree: mt,
    lookUpTable: LOOK_UP_TABLE,
    provider,
  };

  return lightInstance;
}

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
  let pubkey = await mtConf.getSplPoolPda(poolType, asset);

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
