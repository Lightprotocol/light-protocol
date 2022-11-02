const solana = require("@solana/web3.js");
const {U64, I64} = require('n64');
import { assert, expect } from "chai";
import { BigNumber, providers } from 'ethers'
const light = require('../../light-protocol-sdk');
const token = require('@solana/spl-token')
import * as anchor from "@project-serum/anchor";

import {
  read_and_parse_instruction_data_bytes,
  parse_instruction_data_bytes,
  readAndParseAccountDataMerkleTreeTmpState,
  getPdaAddresses,
  unpackLeavesAccount,
} from "./unpack_accounts"

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


export async function checkLastTxSuccess({
  connection,
  merkleTreeProgram,
  pdas,
  sender,
  senderAccountBalancePriorLastTx,
  relayer,
  relayerAccountBalancePriorLastTx,
  recipient,
  recipientBalancePriorLastTx,
  ix_data,
  mode,
  pre_inserted_leaves_index,
  relayerFee,
  is_token = false,
  escrowTokenAccount
}){
  var verifierStateAccount = await connection.getAccountInfo(
    pdas.verifierStatePubkey
  )
  assert(verifierStateAccount == null, "Shielded transaction failed verifierStateAccount is not closed")

  var feeEscrowStateAccount = await connection.getAccountInfo(
    pdas.feeEscrowStatePubkey
  )
  assert(feeEscrowStateAccount == null, "Shielded transaction failed feeEscrowStateAccount is not closed")

  var nullifier0Account = await connection.getAccountInfo(
    pdas.nullifier0PdaPubkey
  )
  await checkRentExemption({
    account: nullifier0Account,
    connection: connection
  })

  var nullifier1Account = await connection.getAccountInfo(
    pdas.nullifier0PdaPubkey
  )

  await checkRentExemption({
    account: nullifier1Account,
    connection: connection
  })

  var leavesAccount = await connection.getAccountInfo(
    pdas.leavesPdaPubkey
  )

  let leavesAccountData = unpackLeavesAccount(leavesAccount.data)
  await checkRentExemption({
    account: leavesAccount,
    connection: connection
  })

  assert_eq(leavesAccountData.leafLeft, ix_data.leafLeft, "left leaf not inserted correctly")
  assert_eq(leavesAccountData.leafRight, ix_data.leafRight, "right leaf not inserted correctly")
  assert_eq(leavesAccountData.encryptedUtxos, ix_data.encryptedUtxos, "encryptedUtxos not inserted correctly")
  assert(leavesAccountData.leafType == 7);

  var preInsertedLeavesIndexAccount = await connection.getAccountInfo(
    pre_inserted_leaves_index
  )

  const preInsertedLeavesIndexAccountAfterUpdate = merkleTreeProgram.account.preInsertedLeavesIndex._coder.accounts.decode('PreInsertedLeavesIndex', preInsertedLeavesIndexAccount.data);

  assert(Number(preInsertedLeavesIndexAccountAfterUpdate.nextIndex) == Number(leavesAccountData.leafIndex) + 2)


  if (mode == "deposit" && is_token == false) {
    var recipientAccount = await connection.getAccountInfo(recipient)
    assert(recipientAccount.lamports == (I64(recipientBalancePriorLastTx).add(I64.readLE(ix_data.extAmount, 0))).toString(), "amount not transferred correctly");

  } else if (mode == "deposit" && is_token == true) {

    var feeEscrowTokenAccount = await connection.getAccountInfo(
      escrowTokenAccount
    )
    assert(feeEscrowTokenAccount == null, "Shielded transaction failed feeEscrowTokenAccount is not closed")

      var recipientAccount = await token.getAccount(
      connection,
      recipient,
      token.TOKEN_PROGRAM_ID
    );

    // console.log(`Balance now ${senderAccount.amount} balance beginning ${senderAccountBalancePriorLastTx}`)
    // assert(senderAccount.lamports == (I64(senderAccountBalancePriorLastTx) - I64.readLE(ix_data.extAmount, 0)).toString(), "amount not transferred correctly");

    // console.log(`Balance now ${recipientAccount.amount} balance beginning ${recipientBalancePriorLastTx}`)
    // console.log(`Balance now ${recipientAccount.amount} balance beginning ${(I64(Number(recipientBalancePriorLastTx)) + I64.readLE(ix_data.extAmount, 0)).toString()}`)
    assert(recipientAccount.amount == (I64(Number(recipientBalancePriorLastTx)).add(I64.readLE(ix_data.extAmount, 0))).toString(), "amount not transferred correctly");

  } else if (mode == "withdrawal" && is_token == false) {
    var senderAccount = await connection.getAccountInfo(sender)
    var recipientAccount = await connection.getAccountInfo(recipient)
    // console.log("senderAccount.lamports: ", senderAccount.lamports)
    // console.log("I64(senderAccountBalancePriorLastTx): ", I64(senderAccountBalancePriorLastTx).toString())
    // console.log("Sum: ", ((I64(senderAccountBalancePriorLastTx).add(I64.readLE(ix_data.extAmount, 0))).sub(I64(relayerFee))).toString())

    assert(senderAccount.lamports == ((I64(senderAccountBalancePriorLastTx).add(I64.readLE(ix_data.extAmount, 0))).sub(I64(relayerFee))).toString(), "amount not transferred correctly");

    var recipientAccount = await connection.getAccountInfo(recipient)
    // console.log(`recipientAccount.lamports: ${recipientAccount.lamports} == sum ${((I64(Number(recipientBalancePriorLastTx)).sub(I64.readLE(ix_data.extAmount, 0))).add(I64(relayerFee))).toString()}
    // Number(recipientBalancePriorLastTx): ${Number(recipientBalancePriorLastTx)}
    // relayerFee: ${Number(relayerFee)}
    // `)
    assert(recipientAccount.lamports == ((I64(Number(recipientBalancePriorLastTx)).sub(I64.readLE(ix_data.extAmount, 0)))).toString(), "amount not transferred correctly");
    // var relayerAccount = await connection.getAccountInfo(
    //   relayer
    // )
    // console.log("relayer: ", relayer.toBase58())
    // let rent_verifier = await connection.getMinimumBalanceForRentExemption(5120)
    // // let rent_escrow = await connection.getMinimumBalanceForRentExemption(256)
    // let rent_nullifier = await connection.getMinimumBalanceForRentExemption(0)
    // let rent_leaves = await connection.getMinimumBalanceForRentExemption(256)
    // console.log("rent_verifier: ", rent_verifier)
    // console.log("rent_nullifier: ", rent_nullifier)
    // console.log("rent_leaves: ", rent_leaves)
    //
    // let expectedBalanceRelayer = I64(relayerFee)
    //   .add(I64(Number(relayerAccountBalancePriorLastTx)))
    //   .add(I64(Number(rent_verifier)))
    //   // .add(I64(Number(rent_escrow)))
    //   .sub(I64(Number(rent_nullifier)))
    //   .sub(I64(Number(rent_nullifier)))
    //   .sub(I64(Number(rent_leaves)))
    // console.log("relayerAccountBalancePriorLastTx: ", relayerAccountBalancePriorLastTx)
    // console.log(`${relayerAccount.lamports } == ${expectedBalanceRelayer}`)
    // assert(relayerAccount.lamports == expectedBalanceRelayer.toString())

  }  else if (mode == "withdrawal" && is_token == true) {
    var senderAccount = await token.getAccount(
      connection,
      sender,
      token.TOKEN_PROGRAM_ID
    );
    var recipientAccount = await token.getAccount(
      connection,
      recipient,
      token.TOKEN_PROGRAM_ID
    );

    var relayerAccount = await token.getAccount(
      connection,
      relayer,
      token.TOKEN_PROGRAM_ID
    );
    assert(senderAccount.amount == ((I64(Number(senderAccountBalancePriorLastTx)).add(I64.readLE(ix_data.extAmount, 0))).sub(I64(relayerFee))).toString(), "amount not transferred correctly");
    // console.log(`${recipientAccount.amount}, ${Number(recipientBalancePriorLastTx)} ${I64.readLE(ix_data.extAmount, 0)} ${I64(relayerFee)}`)
    assert(recipientAccount.amount == ((I64(Number(recipientBalancePriorLastTx)).sub(I64.readLE(ix_data.extAmount, 0)))).toString(), "amount not transferred correctly");
    // console.log(`relayerAccount.amount ${relayerAccount.amount} == I64(relayerFee) ${I64(relayerFee)} + ${relayerAccountBalancePriorLastTx}`)
    assert(relayerAccount.amount == (I64(relayerFee).add(I64(Number(relayerAccountBalancePriorLastTx)))).toString())
  } else {
    throw Error("mode not supplied");
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
