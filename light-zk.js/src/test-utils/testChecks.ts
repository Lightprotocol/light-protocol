const solana = require("@solana/web3.js");
import { assert } from "chai";
const token = require("@solana/spl-token");
import * as anchor from "@coral-xyz/anchor";
import { Connection, PublicKey } from "@solana/web3.js";
import { MerkleTreeProgram } from "../idls";
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

// let CONFIRMATION = {
//   preflightCommitment: "confirmed",
//   commitment: "confirmed",
// };

export async function checkMerkleTreeUpdateStateCreated({
  connection,
  merkleTreeUpdateState,
  relayer,
  transactionMerkleTree,
  leavesPdas,
  current_instruction_index,
  merkleTreeProgram,
}: {
  connection: Connection;
  merkleTreeUpdateState: PublicKey;
  relayer: PublicKey;
  transactionMerkleTree: PublicKey;
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
      "confirmed",
    );
  // merkleTreeProgram.account.preInsertedLeavesIndex._coder.accounts.decode(
  //   "MerkleTreeUpdateState",
  //   merkleTreeTmpAccountInfo.data,
  // );

  var MerkleTreeAccountInfo =
    await merkleTreeProgram.account.transactionMerkleTree.fetch(
      transactionMerkleTree,
      "confirmed",
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
    transactionMerkleTree.toBase58(),
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
  console.log("checkMerkleTreeUpdateStateCreated: success");
  console.log = x;
}

export async function checkMerkleTreeBatchUpdateSuccess({
  connection,
  merkleTreeUpdateState,
  merkleTreeAccountPrior,
  numberOfLeaves,
  leavesPdas,
  transactionMerkleTree,
  merkleTreeProgram,
}: {
  connection: Connection;
  merkleTreeUpdateState: PublicKey;
  merkleTreeAccountPrior: any;
  numberOfLeaves: number;
  leavesPdas: any;
  transactionMerkleTree: PublicKey;
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

  var merkleTreeAccount =
    await merkleTreeProgram.account.transactionMerkleTree.fetch(
      transactionMerkleTree,
      "confirmed",
    );
  // Merkle tree is locked by merkleTreeUpdateState
  assert.equal(
    merkleTreeAccount.pubkeyLocked.toBase58(),
    new solana.PublicKey(new Uint8Array(32).fill(0)).toBase58(),
  );

  assert.equal(
    merkleTreeAccount.timeLocked.toNumber(),
    0,
    "Lock has not been taken within prior  20 slots",
  );

  let merkle_tree_prior_leaves_index = merkleTreeAccountPrior.nextIndex;
  let merkle_tree_prior_current_root_index =
    merkleTreeAccountPrior.currentRootIndex;
  let current_root_index = merkleTreeAccount.currentRootIndex;

  assert.equal(
    merkle_tree_prior_current_root_index
      .add(new anchor.BN("1"))
      .mod(new anchor.BN(256))
      .toString(),
    current_root_index.toString(),
  );

  assert(
    merkle_tree_prior_leaves_index
      .add(new anchor.BN(numberOfLeaves.toString()))
      .toString() == merkleTreeAccount.nextIndex.toString(),
  );
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
  returnValue: boolean = false,
) {
  for (var i = 0; i < pubkeys.length; i++) {
    var accountInfo = await connection.getAccountInfo(pubkeys[i].pubkey);
    if (!returnValue) assert.equal(accountInfo, null);
    else return accountInfo;
  }
}
