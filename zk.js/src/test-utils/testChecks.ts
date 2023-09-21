const solana = require("@solana/web3.js");
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

  if (
    merkleTreeTmpAccountInfo.owner.toBase58() !==
    merkleTreeProgram.programId.toBase58()
  ) {
    throw new Error(
      `merkle tree pda owner wrong after initializing: expected ${merkleTreeProgram.programId.toBase58()}, got ${merkleTreeTmpAccountInfo.owner.toBase58()}`,
    );
  }

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

  if (merkleTreeUpdateStateData.relayer.toBase58() !== relayer.toBase58()) {
    throw new Error(
      `The incorrect signer has been saved: expected ${relayer.toBase58()}, got ${merkleTreeUpdateStateData.relayer.toBase58()}`,
    );
  }

  if (
    merkleTreeUpdateStateData.merkleTreePdaPubkey.toBase58() !==
    transactionMerkleTree.toBase58()
  ) {
    throw new Error(
      `the incorrect merkle tree pubkey was saved: expected ${transactionMerkleTree.toBase58()}, got ${merkleTreeUpdateStateData.merkleTreePdaPubkey.toBase58()}`,
    );
  }

  if (merkleTreeUpdateStateData.numberOfLeaves !== leavesPdas.length) {
    throw new Error(
      `The incorrect number of leaves was saved: expected ${leavesPdas.length}, got ${merkleTreeUpdateStateData.numberOfLeaves}`,
    );
  }

  if (
    !merkleTreeUpdateStateData.currentInstructionIndex.eq(
      new anchor.BN(current_instruction_index),
    )
  ) {
    throw new Error(
      `The instruction index is wrong: expected ${current_instruction_index}, got ${merkleTreeUpdateStateData.currentInstructionIndex.toString()}`,
    );
  }

  if (
    MerkleTreeAccountInfo.pubkeyLocked.toBase58() !==
    merkleTreeUpdateState.toBase58()
  ) {
    throw new Error(
      `Expected ${merkleTreeUpdateState.toBase58()}, got ${MerkleTreeAccountInfo.pubkeyLocked.toBase58()}`,
    );
  }
  console.log("checkMerkleTreeUpdateStateCreated: success");
  console.log = x;
}

export async function checkMerkleTreeBatchUpdateSuccess({
  connection,
  merkleTreeUpdateState,
  merkleTreeAccountPrior,
  numberOfLeaves,
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

  if (merkleTreeTmpStateAccount !== null) {
    throw new Error(
      "Shielded transaction failed merkleTreeTmpStateAccount is not closed",
    );
  }

  var merkleTreeAccount =
    await merkleTreeProgram.account.transactionMerkleTree.fetch(
      transactionMerkleTree,
      "confirmed",
    );
  // Merkle tree is locked by merkleTreeUpdateState
  if (
    merkleTreeAccount.pubkeyLocked.toBase58() !==
    new solana.PublicKey(new Uint8Array(32).fill(0)).toBase58()
  ) {
    throw new Error(
      `Expected ${new solana.PublicKey(
        new Uint8Array(32).fill(0),
      ).toBase58()}, got ${merkleTreeAccount.pubkeyLocked.toBase58()}`,
    );
  }

  if (merkleTreeAccount.timeLocked.toNumber() !== 0) {
    throw new Error(
      `Lock has not been taken within prior 20 slots: expected 0, got ${merkleTreeAccount.timeLocked.toNumber()}`,
    );
  }

  let merkle_tree_prior_leaves_index = merkleTreeAccountPrior.nextIndex;
  let merkle_tree_prior_current_root_index =
    merkleTreeAccountPrior.currentRootIndex;
  let current_root_index = merkleTreeAccount.currentRootIndex;

  if (
    !merkle_tree_prior_current_root_index
      .add(new anchor.BN("1"))
      .mod(new anchor.BN(256))
      .eq(current_root_index)
  ) {
    throw new Error("Unexpected value for current root index");
  }

  if (
    !merkle_tree_prior_leaves_index
      .add(new anchor.BN(numberOfLeaves))
      .eq(merkleTreeAccount.nextIndex)
  ) {
    throw new Error(
      `Expected ${merkle_tree_prior_leaves_index
        .add(new anchor.BN(numberOfLeaves))
        .toString()}, got ${merkleTreeAccount.nextIndex.toString()}`,
    );
  }
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
    if (!returnValue && accountInfo === null)
      throw new Error("nullifier not inserted");
    else return accountInfo;
  }
}
