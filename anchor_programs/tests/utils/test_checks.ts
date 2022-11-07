const solana = require("@solana/web3.js");
const {U64, I64} = require('n64');
import { assert, expect } from "chai";
import { BigNumber, providers } from 'ethers'
const light = require('../../light-protocol-sdk');
const token = require('@solana/spl-token')
import * as anchor from "@project-serum/anchor";

export function assert_eq(
  value0: unknown,
  value1: unknown,
  message: string
) {

  if (value0.length !== value1.length) {
    console.log("value0: ", value0)
    console.log("value1: ", value1)
    throw Error("Length of asserted values does not match");
  }
  for (var i = 0; i < value0.length; i++) {
    if (value0[i] !== value1[i]) {
      throw Error(message);
    }
  }

}


/*
*
* Checks:
* owner
* relayer saved as signer
* merkle tree saved
* number of leaves saved correctly
* current instruction index is correct
* merkle tree is locked by updateState account
* lock has been taken less than 5 slots ago
*/
let CONFIRMATION = {preflightCommitment: "finalized", commitment: "finalized"};

export async function checkMerkleTreeUpdateStateCreated({
    connection,
    merkleTreeUpdateState,
    relayer,
    MerkleTree,
    leavesPdas,
    current_instruction_index,
    merkleTreeProgram
  }) {
  var merkleTreeTmpAccountInfo = await connection.getAccountInfo(
    merkleTreeUpdateState, CONFIRMATION
  )

  assert(merkleTreeTmpAccountInfo.owner.toBase58() === merkleTreeProgram.programId.toBase58(), "merkle tree pda owner wrong after initializing")
  const merkleTreeUpdateStateData = merkleTreeProgram.account.preInsertedLeavesIndex._coder.accounts.decode('MerkleTreeUpdateState', merkleTreeTmpAccountInfo.data);

  var MerkleTreeAccountInfo = await merkleTreeProgram.account.merkleTree.fetch(
    MerkleTree
  )

  // console.log("merkleTreeUpdateStateData.leaves ", merkleTreeUpdateStateData.leaves);
  console.log("merkleTreeUpdateStateData.numberOfLeaves ", merkleTreeUpdateStateData.numberOfLeaves);
  console.log("leavesPdas.length ", leavesPdas.length);
  console.log("merkleTreeUpdateStateData.currentInstructionIndex ", merkleTreeUpdateStateData.currentInstructionIndex);
  console.log("current_instruction_index ", current_instruction_index);

  assert(merkleTreeUpdateStateData.relayer.toBase58() == relayer.toBase58(), "The incorrect signer has been saved")
  assert(merkleTreeUpdateStateData.merkleTreePdaPubkey.toBase58() == MerkleTree.toBase58(), "the incorrect merkle tree pubkey was saved")
  assert(merkleTreeUpdateStateData.numberOfLeaves == leavesPdas.length, "The incorrect number of leaves was saved")
  assert(merkleTreeUpdateStateData.currentInstructionIndex == current_instruction_index, "The instruction index is wrong")
  assert(MerkleTreeAccountInfo.pubkeyLocked.toBase58() == merkleTreeUpdateState.toBase58());
  // assert(U64.readLE(MerkleTreeAccountInfo.data.slice(16658-8,16658), 0) >= (await connection.getSlot()) - 5, "Lock has not been taken at this or in the 5 prior slots");
  console.log("checkMerkleTreeUpdateStateCreated: success");

}

export async function checkMerkleTreeBatchUpdateSuccess({
  connection,
  merkleTreeUpdateState,
  merkleTreeAccountPrior,
  numberOfLeaves,
  leavesPdas,
  merkleTree,
  merkle_tree_pubkey,
  merkleTreeProgram
}) {

  var merkleTreeTmpStateAccount = await connection.getAccountInfo(
        merkleTreeUpdateState
      )

  assert(merkleTreeTmpStateAccount === null, "Shielded transaction failed merkleTreeTmpStateAccount is not closed")

  var merkleTreeAccount = await merkleTreeProgram.account.merkleTree.fetch(merkle_tree_pubkey)
  // Merkle tree is locked by merkleTreeUpdateState
  assert(merkleTreeAccount.pubkeyLocked.toBase58()== new solana.PublicKey(new Uint8Array(32).fill(0)).toBase58());
  console.log("merkleTreeAccount.time_locked ", merkleTreeAccount.timeLocked);

  assert(merkleTreeAccount.timeLocked == 0, "Lock has not been taken within prior  20 slots");

  let merkle_tree_prior_leaves_index = merkleTreeAccountPrior.nextIndex; //U64.readLE(merkleTreeAccountPrior.data.slice(594, 594 + 8),0);
  let merkle_tree_prior_current_root_index = merkleTreeAccountPrior.currentRootIndex; //U64.readLE(merkleTreeAccountPrior.data.slice(594 - 8, 594),0).toNumber()
  console.log(merkleTreeAccount);

  let current_root_index = merkleTreeAccount.currentRootIndex; //tU64.readLE(merkleTreeAccount.data.slice(594 - 8, 594),0).toNumber()
  console.log("merkle_tree_prior_current_root_index: ", merkle_tree_prior_current_root_index)
  console.log("current_root_index: ", current_root_index)
  console.log(`${merkle_tree_prior_current_root_index.add(new anchor.BN("1")) } == ${current_root_index}`);

  assert(merkle_tree_prior_current_root_index.add(new anchor.BN("1")).toString() === current_root_index.toString())
  let current_root_start_range = 610 + current_root_index * 32;
  let current_root_end_range = 610 + (current_root_index + 1) * 32;
  // console.log(`root: ${BigNumber.from(merkleTreeAccount.data.slice(current_root_start_range, current_root_end_range).reverse()).toHexString()}`)

  // console.log(`prior +${numberOfLeaves} ${merkle_tree_prior_leaves_index.add(new anchor.BN(numberOfLeaves)).toString()}, now ${U64.readLE(merkleTreeAccount.data.slice(594, 594 + 8), 0).toString()}`)
  // // index has increased by numberOfLeaves
  // console.log(`index has increased by numberOfLeaves: ${merkle_tree_prior_leaves_index.add(U64(numberOfLeaves)).toString()}, ${U64.readLE(merkleTreeAccount.data.slice(594, 594 + 8), 0).toString()}`)
  console.log("numberOfLeaves: ", numberOfLeaves);

  console.log(`${merkle_tree_prior_leaves_index.add(new anchor.BN(numberOfLeaves))} == ${merkleTreeAccount.nextIndex}`);

  assert(merkle_tree_prior_leaves_index.add(new anchor.BN(numberOfLeaves)).toString() == merkleTreeAccount.nextIndex.toString())//U64.readLE(merkleTreeAccount.data.slice(594, 594 + 8), 0).toString())

  // let leavesPdasPubkeys = []
  // leavesPdas.map( (pda) => { leavesPdasPubkeys.push(pda.pubkey) })
  // var leavesAccounts = await merkleTreeProgram.account.twoLeavesBytesPda.fetchMultiple(
  //   leavesPdasPubkeys
  //     )
  // let leaves_to_sort = []
  // leavesAccounts.map((acc) => {
  //   // Checking that all leaves have been marked as inserted.
  //   assert(leavesAccounts.isInserted == true);
  //     leaves_to_sort.push(leavesAccounts);
  //   });
  // leaves_to_sort.sort((a, b) => parseFloat(a.left_leaf_index) - parseFloat(b.left_leaf_index));
  // let numberOfLeavesPdas = 0
  // for (var i = Number(merkle_tree_prior_leaves_index); i < Number(merkle_tree_prior_leaves_index) + Number(numberOfLeaves); i+=2) {
  //   merkleTree.update(i, BigNumber.from(leaves_to_sort[numberOfLeavesPdas].leaves.slice(0,32).reverse()))
  //   merkleTree.update(i + 1, BigNumber.from(leaves_to_sort[numberOfLeavesPdas].leaves.slice(32,64).reverse()))
  //   numberOfLeavesPdas++;
  // }
  //
  // // Comparing root from chain with locally updated merkle tree.
  // assert(BigNumber.from(merkleTreeAccount.data.slice(current_root_start_range, current_root_end_range).reverse()).toHexString(),
  //   merkleTree.root().toHexString()
  // )
  // // Comparing locally generated root with merkle tree built from leaves fetched from chain.
  // assert(merkleTree.root().toHexString() == (await light.buildMerkelTree(connection, merkle_tree_pubkey.toBytes())).root().toHexString());

}

export async function checkRentExemption({
  connection,
  account
}) {
  let requiredBalance = await connection.getMinimumBalanceForRentExemption(account.data.length);
  if (account.lamports  < requiredBalance) {
    throw Error(`Account of size ${account.data.length} not rentexempt balance ${account.lamports} should be${requiredBalance}`)
  }

}

export async function  checkNfInserted(pubkeys, connection) {
  for (var i = 0; i < pubkeys.length; i++) {
    var accountInfo = await connection.getAccountInfo(
      pubkeys[i]
    )

    assert(accountInfo == null);
  }

}
