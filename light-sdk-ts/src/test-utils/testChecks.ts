const solana = require("@solana/web3.js");
import { assert, expect } from "chai";
const token = require("@solana/spl-token");
import * as anchor from "@coral-xyz/anchor";
import { Connection, PublicKey } from "@solana/web3.js";
import { MerkleTreeProgram } from "idls";
import { Program } from "@coral-xyz/anchor";

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
let CONFIRMATION = {
  preflightCommitment: "confirmed",
  commitment: "confirmed",
};

export async function checkMerkleTreeUpdateStateCreated({
  connection,
  merkleTreeUpdateState,
  relayer,
  MerkleTree,
  leavesPdas,
  current_instruction_index,
  merkleTreeProgram,
}: {
  connection: Connection;
  merkleTreeUpdateState: PublicKey;
  relayer: PublicKey;
  MerkleTree: PublicKey;
  leavesPdas: Array<any>;
  current_instruction_index: number;
  merkleTreeProgram: anchor.Program<MerkleTreeProgram>;
}) {
  let x = console.log;
  console.log = () => {};
  var merkleTreeTmpAccountInfo = await connection.getAccountInfo(
    merkleTreeUpdateState,
    "confirmed",
  );
  if (!merkleTreeTmpAccountInfo)
    throw new Error("merkleTreeTmpAccountInfo null");

  assert.equal(
    merkleTreeTmpAccountInfo.owner.toBase58(),
    merkleTreeProgram.programId.toBase58(),
    "merkle tree pda owner wrong after initializing",
  );
  const merkleTreeUpdateStateData =
    await merkleTreeProgram.account.merkleTreeUpdateState.fetch(
      merkleTreeUpdateState,
    );
  // merkleTreeProgram.account.preInsertedLeavesIndex._coder.accounts.decode(
  //   "MerkleTreeUpdateState",
  //   merkleTreeTmpAccountInfo.data,
  // );

  var MerkleTreeAccountInfo = await merkleTreeProgram.account.merkleTree.fetch(
    MerkleTree,
  );

  // console.log("merkleTreeUpdateStateData.leaves ", merkleTreeUpdateStateData.leaves);
  console.log(
    "merkleTreeUpdateStateData.numberOfLeaves ",
    merkleTreeUpdateStateData.numberOfLeaves,
  );
  console.log("leavesPdas.length ", leavesPdas.length);
  console.log(
    "merkleTreeUpdateStateData.currentInstructionIndex ",
    merkleTreeUpdateStateData.currentInstructionIndex,
  );
  console.log("current_instruction_index ", current_instruction_index);

  assert.equal(
    merkleTreeUpdateStateData.relayer.toBase58(),
    relayer.toBase58(),
    "The incorrect signer has been saved",
  );
  assert.equal(
    merkleTreeUpdateStateData.merkleTreePdaPubkey.toBase58(),
    MerkleTree.toBase58(),
    "the incorrect merkle tree pubkey was saved",
  );
  assert.equal(
    merkleTreeUpdateStateData.numberOfLeaves,
    leavesPdas.length,
    "The incorrect number of leaves was saved",
  );
  assert.equal(
    merkleTreeUpdateStateData.currentInstructionIndex.toString(),
    current_instruction_index.toString(),
    "The instruction index is wrong",
  );
  assert.equal(
    MerkleTreeAccountInfo.pubkeyLocked.toBase58(),
    merkleTreeUpdateState.toBase58(),
  );
  // assert(U64.readLE(MerkleTreeAccountInfo.data.slice(16658-8,16658), 0) >= (await connection.getSlot()) - 5, "Lock has not been taken at this or in the 5 prior slots");
  console.log("checkMerkleTreeUpdateStateCreated: success");
  console.log = x;
}

export async function checkMerkleTreeBatchUpdateSuccess({
  connection,
  merkleTreeUpdateState,
  merkleTreeAccountPrior,
  numberOfLeaves,
  leavesPdas,
  merkle_tree_pubkey,
  merkleTreeProgram,
}: {
  connection: Connection;
  merkleTreeUpdateState: PublicKey;
  merkleTreeAccountPrior: any;
  numberOfLeaves: number;
  leavesPdas: any;
  merkle_tree_pubkey: PublicKey;
  merkleTreeProgram: Program<MerkleTreeProgram>;
}) {
  var merkleTreeTmpStateAccount = await connection.getAccountInfo(
    merkleTreeUpdateState,
    "confirmed",
  );

  assert.equal(
    merkleTreeTmpStateAccount,
    null,
    "Shielded transaction failed merkleTreeTmpStateAccount is not closed",
  );

  var merkleTreeAccount = await merkleTreeProgram.account.merkleTree.fetch(
    merkle_tree_pubkey,
    "confirmed",
  );
  // Merkle tree is locked by merkleTreeUpdateState
  assert.equal(
    merkleTreeAccount.pubkeyLocked.toBase58(),
    new solana.PublicKey(new Uint8Array(32).fill(0)).toBase58(),
  );
  // console.log("merkleTreeAccount.time_locked ", merkleTreeAccount.timeLocked);

  assert.equal(
    merkleTreeAccount.timeLocked.toNumber(),
    0,
    "Lock has not been taken within prior  20 slots",
  );

  let merkle_tree_prior_leaves_index = merkleTreeAccountPrior.nextIndex; //U64.readLE(merkleTreeAccountPrior.data.slice(594, 594 + 8),0);
  let merkle_tree_prior_current_root_index =
    merkleTreeAccountPrior.currentRootIndex; //U64.readLE(merkleTreeAccountPrior.data.slice(594 - 8, 594),0).toNumber()

  let current_root_index = merkleTreeAccount.currentRootIndex; //tU64.readLE(merkleTreeAccount.data.slice(594 - 8, 594),0).toNumber()
  // console.log(
  //   "merkle_tree_prior_current_root_index: ",
  //   merkle_tree_prior_current_root_index,
  // );
  // console.log("current_root_index: ", current_root_index);
  // console.log(
  //   `${merkle_tree_prior_current_root_index.add(
  //     new anchor.BN("1"),
  //   )} == ${current_root_index}`,
  // );

  assert.equal(
    merkle_tree_prior_current_root_index.add(new anchor.BN("1")).toString(),
    current_root_index.toString(),
  );
  let current_root_start_range = 610 + current_root_index.toNumber() * 32;
  let current_root_end_range = 610 + (current_root_index.toNumber() + 1) * 32;
  // console.log(`root: ${BigNumber.from(merkleTreeAccount.data.slice(current_root_start_range, current_root_end_range).reverse()).toHexString()}`)

  // console.log(`prior +${numberOfLeaves} ${merkle_tree_prior_leaves_index.add(new anchor.BN(numberOfLeaves)).toString()}, now ${U64.readLE(merkleTreeAccount.data.slice(594, 594 + 8), 0).toString()}`)
  // // index has increased by numberOfLeaves
  // console.log(`index has increased by numberOfLeaves: ${merkle_tree_prior_leaves_index.add(U64(numberOfLeaves)).toString()}, ${U64.readLE(merkleTreeAccount.data.slice(594, 594 + 8), 0).toString()}`)

  // console.log(
  //   `${merkle_tree_prior_leaves_index.add(new anchor.BN(numberOfLeaves))} == ${
  //     merkleTreeAccount.nextIndex
  //   }`,
  // );

  assert(
    merkle_tree_prior_leaves_index
      .add(new anchor.BN(numberOfLeaves.toString()))
      .toString() == merkleTreeAccount.nextIndex.toString(),
  ); //U64.readLE(merkleTreeAccount.data.slice(594, 594 + 8), 0).toString())

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
}

export async function checkRentExemption({
  connection,
  account,
}: {
  connection: Connection;
  account: any;
}) {
  let requiredBalance = await connection.getMinimumBalanceForRentExemption(
    account.data.length,
  );
  if (account.lamports < requiredBalance) {
    throw Error(
      `Account of size ${account.data.length} not rentexempt balance ${account.lamports} should be${requiredBalance}`,
    );
  }
}

export async function checkNfInserted(
  pubkeys: { isSigner: boolean; isWritatble: boolean; pubkey: PublicKey }[],
  connection: Connection,
) {
  for (var i = 0; i < pubkeys.length; i++) {
    var accountInfo = await connection.getAccountInfo(pubkeys[i].pubkey);

    assert.equal(accountInfo, null);
  }
}
