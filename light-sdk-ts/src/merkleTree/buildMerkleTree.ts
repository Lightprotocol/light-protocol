import { AccountInfo, Connection, PublicKey } from '@solana/web3.js'
import {MerkleTree } from './merkleTree';
const anchor = require("@project-serum/anchor")
var ffjavascript = require('ffjavascript');
const { unstringifyBigInts, leInt2Buff } = ffjavascript.utils;
const  MERKLE_TREE_HEIGHT = 18


// TODO try to merge into merkle tree class
export const buildMerkleTree = async function ({connection, config, merkleTreePubkey,merkleTreeProgram, poseidonHash}:{
  connection: Connection,
  config: any, //NetworkConfig,
  merkleTreePubkey: PublicKey, // pubkey to bytes
  merkleTreeProgram: any,
  poseidonHash: any
}) {
  let mtFetched = await merkleTreeProgram.account.merkleTree.fetch(merkleTreePubkey)
  // Fetch all the accounts owned by the specified program id
  const leave_accounts: Array<{
    pubkey: PublicKey
    account: AccountInfo<Buffer>
  }> = await merkleTreeProgram.account.twoLeavesBytesPda.all()

  leave_accounts
  .sort((a, b) => a.account.leftLeafIndex.toNumber() - b.account.leftLeafIndex.toNumber());

  const leaves: string[] = []
  if(leave_accounts.length > 0){
    for (let i: number = 0; i < leave_accounts.length; i++) {
      if (leave_accounts[i].account.leftLeafIndex.toNumber() < mtFetched.nextIndex.toNumber()) {
        leaves.push(new anchor.BN(leave_accounts[i].account.nodeLeft.reverse()).toString()) // .reverse()
        leaves.push(new anchor.BN(leave_accounts[i].account.nodeRight.reverse()).toString())
      }
    }
  }

  let fetchedMerkleTree = new MerkleTree(MERKLE_TREE_HEIGHT,poseidonHash, leaves)

  if (Array.from(leInt2Buff(unstringifyBigInts(fetchedMerkleTree.root()), 32)).toString() != mtFetched.roots[mtFetched.currentRootIndex].toString()) {
    throw `building merkle tree from chain failed: root local ${Array.from(leInt2Buff(unstringifyBigInts(fetchedMerkleTree.root()), 32)).toString()} != root fetched ${mtFetched.roots[mtFetched.currentRootIndex]}`;
  }

  return fetchedMerkleTree;
}
