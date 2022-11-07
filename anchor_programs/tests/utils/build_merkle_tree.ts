import { AccountInfo, Connection, PublicKey } from '@solana/web3.js'
const light = require('../../light-protocol-sdk');
const anchor = require("@project-serum/anchor")
var ffjavascript = require('ffjavascript');
const { unstringifyBigInts, leInt2Buff } = ffjavascript.utils;
const { U64 } = require('n64')
const  MERKLE_TREE_HEIGHT = 18

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


  // /// Sorts leaves and substracts float of index a by index b
  // leaves_to_sort.sort((a, b) => parseFloat(a.leftLeafIndex) - parseFloat(b.leftLeafIndex))
  leave_accounts
  // .filter((pda) => {
  //   // filter for leaves accounts which are inserted
  //   // TODO: move this filter into the fetch statement
  //   // TODO: fix doesn t seem to work
  //   return pda.account.leftLeafIndex.toNumber() < mtFetched.nextIndex.toNumber()
  // })
  .sort((a, b) => a.account.leftLeafIndex.toNumber() - b.account.leftLeafIndex.toNumber());


  const leaves: string[] = []
// console.log("------------------------------------------------------");
//   console.log("[");

  if(leave_accounts.length > 0){
    for (let i: number = 0; i < leave_accounts.length; i++) {
      if (leave_accounts[i].account.leftLeafIndex.toNumber() < mtFetched.nextIndex.toNumber()) {
        leaves.push(new anchor.BN(leave_accounts[i].account.nodeLeft.reverse()).toString()) // .reverse()
        leaves.push(new anchor.BN(leave_accounts[i].account.nodeRight.reverse()).toString())
        // console.log("[ ");
        //
        // console.log(leave_accounts[i].account.nodeLeft.reverse());
        // console.log(",");
        //
        // console.log(leave_accounts[i].account.nodeRight.reverse());
        // console.log(",");
        // console.log("],");

      }
    }
  }
  // console.log("]");
  //
  // console.log("------------------------------------------------------");

  // console.log(leaves[0]);
  // console.log(new anchor.BN(leave_accounts[0].account.nodeLeft));

  let test = poseidonHash.F.toString(poseidonHash([new anchor.BN(leave_accounts[0].account.nodeLeft)]))
  // Works
  // let fetchedMerkleTreeTest = await light.buildMerkelTree(poseidonHash, MERKLE_TREE_HEIGHT, [])
  // console.log("fetchedMerkleTreeTest.root()  ", Array.from(leInt2Buff(unstringifyBigInts(fetchedMerkleTreeTest.root()))) );


  let fetchedMerkleTree = await light.buildMerkelTree(poseidonHash, MERKLE_TREE_HEIGHT, leaves)

  // console.log("fetchedMerkleTree.root()  ", Array.from(leInt2Buff(unstringifyBigInts(fetchedMerkleTree.root()), 32)) );
  // console.log(" mtFetched.roots[mtFetched.currentRootIndex ",  mtFetched.roots[mtFetched.currentRootIndex]);
  // console.log(mtFetched);

  if (Array.from(leInt2Buff(unstringifyBigInts(fetchedMerkleTree.root()), 32)).toString() != mtFetched.roots[mtFetched.currentRootIndex].toString()) {
    throw `building merkle tree from chain failed: root local ${Array.from(leInt2Buff(unstringifyBigInts(fetchedMerkleTree.root()), 32)).toString()} != root fetched ${mtFetched.roots[mtFetched.currentRootIndex]}`;
  }

  return fetchedMerkleTree;
}
