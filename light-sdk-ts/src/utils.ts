import { BN } from "@project-serum/anchor"
import { ethers } from "ethers"
import { merkleTreeProgram, MERKLE_TREE_KEY, MINT } from "./constants";
var ffjavascript = require('ffjavascript');
const { unstringifyBigInts, leInt2Buff } = ffjavascript.utils;
import {Connection, PublicKey, SystemProgram} from '@solana/web3.js';
import { MerkleTreeConfig } from "./merkleTree";

// TODO: get rid of ether dep
export function hashAndTruncateToCircuit(data: Uint8Array) {
    return new BN(leInt2Buff(unstringifyBigInts(ethers.utils.keccak256(data).toString()), 32).slice(0,31), undefined, 'le')
}


// TODO: add pooltype
export async function getAssetLookUpId({
  connection,
  asset
} : {
  asset: PublicKey,
  connection: Connection,
  // poolType?: Uint8Array
}): Promise<any> {
  let poolType = new Uint8Array(32).fill(0);
  let mtConf = new MerkleTreeConfig({connection, merkleTreePubkey: MERKLE_TREE_KEY})
  let pubkey = await mtConf.getSplPoolPda(poolType,asset);
  
  let registeredAssets = await merkleTreeProgram.account.registeredAssetPool.fetch(pubkey.pda);
 
  return registeredAssets.index;
}


export function fetchAssetByIdLookUp({
  assetIndex
} : {
  assetIndex: BN,
}): PublicKey {
  // TODO: find smarter way to do this maybe query from account
  console.log("here ", assetIndex);
  let poolType = new Uint8Array(32).fill(0);
  if (assetIndex.toString() == '0' ) {
    return MINT;
  } else if (assetIndex.toString() == '1' ) {
    return SystemProgram.programId;
  } else {
    throw `no entry for index ${assetIndex}`;
  }

  // let registeredAssets = await merkleTreeProgram.account.registeredAssetPool.all();
  // // console.log("registeredAssets ", registeredAssets.publictoBase58());

  // return registeredAssets.index;
  // let x 
  // registeredAssets.map((a)=> {
  //   console.log("a.account.assetPoolPubkey.toBase58() ", a.account.index.toString());
  //   console.log("asset.toBase58() ", assetIndex.toString());

  //   if(a.account.index.toString() == assetIndex.toString()) {
  //     console.log("returned ", a.account.pubkey);
  //     console.log("returned ", a);

  //     x = a.account.assetPoolPubkey;
  // }});
  // return x;
}