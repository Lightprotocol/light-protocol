const solana = require("@solana/web3.js");
const {U64, I64} = require('n64');
import { assert, expect } from "chai";
import { BigNumber, providers } from 'ethers'
const light = require('../../light-protocol-sdk');
const token = require('@solana/spl-token')

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

export async function checkEscrowAccountCreated({
  connection,
  pdas,
  user_pubkey,
  relayer_pubkey,
  ix_data,
  tx_fee,
  verifierProgram,
  is_token = false,
  escrowTokenAccount,
  rent
}) {
  var escrowAccountInfo = await connection.getAccountInfo(
    pdas.feeEscrowStatePubkey
  )
  const accountAfterUpdate = verifierProgram.account.verifierState._coder.accounts.decode('FeeEscrowState', escrowAccountInfo.data);
  if (!is_token) {
    // console.log(`${(escrowAccountInfo.lamports ).toString()} rent ${rent} tx_fee ${tx_fee} ${U64.readLE(ix_data.extAmount, 0).toString()}`)
    //
    // console.log(`${(escrowAccountInfo.lamports - rent - tx_fee).toString()} ${U64.readLE(ix_data.extAmount, 0).toString()}`)
    assert((escrowAccountInfo.lamports - rent - tx_fee).toString() == U64.readLE(ix_data.extAmount, 0).toString(), "incorrect amount transferred");
  } else {
    // console.log(` ${escrowAccountInfo.lamports - rent}, ${tx_fee}`)
    // console.log(` ${escrowAccountInfo.lamports} rent ${ rent}, ${tx_fee}`)
assert((escrowAccountInfo.lamports - rent).toString() == tx_fee.toString(), "incorrect tx fee transferred");

    let escrowTokenAccountInfo = await token.getAccount(
      connection,
      escrowTokenAccount,
      token.TOKEN_PROGRAM_ID
    );
    assert(escrowTokenAccountInfo.amount.toString() ==  U64.readLE(ix_data.extAmount, 0).toString());
  }

  // console.log(`accountAfterUpdate.txFee.toString(): ${accountAfterUpdate.txFee.toString()} vs ${tx_fee.toString()}`)
  assert(accountAfterUpdate.txFee.toString() == tx_fee.toString(), "tx_fee insert wrong");
  assert(accountAfterUpdate.relayerFee.toString() == U64.readLE(ix_data.fee, 0).toString(), "relayer_fee insert wrong");
  assert(accountAfterUpdate.relayerPubkey.toBase58() == relayer_pubkey.toBase58(), "relayer_pubkey insert wrong");
  assert(accountAfterUpdate.verifierStatePubkey.toBase58() == pdas.verifierStatePubkey.toBase58(), "verifierStatePubkey insert wrong");
  assert(accountAfterUpdate.userPubkey.toBase58() == user_pubkey.toBase58(), "user_pubkey insert wrong");
  assert(Number(accountAfterUpdate.creationSlot) <= await connection.getSlot(), "Slot set wrong");
  assert(Number(accountAfterUpdate.creationSlot) > (await connection.getSlot()) - 5, "Slot set outside of 5 block tolerance");

  var verifierStateInfo = await connection.getAccountInfo(
    pdas.verifierStatePubkey
  )
  const verifierStateInfoUpdate = verifierProgram.account.verifierState._coder.accounts.decode('VerifierState', verifierStateInfo.data);
  assert(verifierStateInfoUpdate.signingAddress.toBase58() == relayer_pubkey.toBase58(), "relayer_pubkey insert wrong");
}

export async function checkVerifierStateAccountCreated({connection, pda, ix_data, relayer_pubkey}) {
  var userAccountInfo = await connection.getAccountInfo(pda)

  const accountAfterUpdate = verifierProgram.account.verifierState._coder.accounts.decode('VerifierState', userAccountInfo.data);

  assert_eq(accountAfterUpdate.rootHash, ix_data.rootHash, "rootHash insert wrong");
  assert_eq(accountAfterUpdate.amount, ix_data.amount, "amount insert wrong");
  assert_eq(accountAfterUpdate.txIntegrityHash, ix_data.txIntegrityHash, "txIntegrityHash insert wrong");
  assert_eq(accountAfterUpdate.extAmount, ix_data.extAmount, "extAmount insert wrong");
  assert_eq(accountAfterUpdate.signingAddress, relayer_pubkey, "relayer insert wrong");
  assert_eq(accountAfterUpdate.fee, ix_data.relayer_fee, "fee insert wrong");
  assert_eq(accountAfterUpdate.merkleTreeIndex, ix_data.merkleTreeIndex[0], "merkleTreeIndex insert wrong");
  assert(accountAfterUpdate.currentInstructionIndex == 1, "Current instruction update updated wrong");
}

export async function checkFinalExponentiationSuccess({connection, pda, ix_data, verifierProgram}) {
  var userAccountInfo = await connection.getAccountInfo(pda)

  const accountAfterUpdate = verifierProgram.account.verifierState._coder.accounts.decode('VerifierState', userAccountInfo.data);
  const expectedFinalExponentiation = [13, 20, 220, 48, 182, 120, 53, 125, 152, 139, 62, 176, 232, 173, 161, 27, 199, 178, 181, 210,
    207, 12, 31, 226, 117, 34, 203, 42, 129, 155, 124, 4, 74, 96, 27, 217, 48, 42, 148, 168, 6,
    119, 169, 247, 46, 190, 170, 218, 19, 30, 155, 251, 163, 6, 33, 200, 240, 56, 181, 71, 190,
    185, 150, 46, 24, 32, 137, 116, 44, 29, 56, 132, 54, 119, 19, 144, 198, 175, 153, 55, 114, 156,
    57, 230, 65, 71, 70, 238, 86, 54, 196, 116, 29, 31, 34, 13, 244, 92, 128, 167, 205, 237, 90,
    214, 83, 188, 79, 139, 32, 28, 148, 5, 73, 24, 222, 225, 96, 225, 220, 144, 206, 160, 39, 212,
    236, 105, 224, 26, 109, 240, 248, 215, 57, 215, 145, 26, 166, 59, 107, 105, 35, 241, 12, 220,
    231, 99, 222, 16, 70, 254, 15, 145, 213, 144, 245, 245, 16, 57, 118, 17, 197, 122, 198, 218,
    172, 47, 146, 34, 216, 204, 49, 48, 229, 127, 153, 220, 210, 237, 236, 179, 225, 209, 27, 134,
    12, 13, 157, 100, 165, 221, 163, 15, 66, 184, 168, 229, 19, 201, 213, 152, 52, 134, 51, 44, 62,
    205, 18, 54, 25, 43, 152, 134, 102, 193, 88, 24, 131, 133, 89, 188, 39, 182, 165, 15, 73, 254,
    232, 143, 212, 58, 200, 141, 195, 231, 84, 25, 191, 212, 81, 55, 78, 37, 184, 196, 132, 91, 75,
    252, 189, 70, 10, 212, 139, 181, 80, 22, 228, 225, 237, 242, 147, 105, 106, 67, 183, 108, 138,
    95, 239, 254, 108, 253, 219, 89, 205, 123, 192, 36, 108, 23, 132, 6, 30, 211, 239, 242, 40, 10,
    116, 229, 111, 202, 188, 91, 147, 216, 77, 114, 225, 10, 10, 215, 128, 121, 176, 45, 6, 204,
    140, 58, 228, 53, 147, 108, 226, 232, 87, 34, 216, 43, 148, 128, 164, 111, 3, 153, 136, 168,
    12, 244, 202, 102, 156, 2, 97, 0, 248, 206, 63, 188, 82, 152, 24, 13, 236, 8, 210, 5, 93, 122,
    98, 26, 211, 204, 79, 221, 153, 36, 42, 134, 215, 200, 5, 40, 211, 180, 56, 196, 102, 146, 136,
    197, 107, 119, 171, 184, 54, 117, 40, 163, 31, 1, 197, 17];
    assert_eq(accountAfterUpdate.fBytes2, expectedFinalExponentiation, "Final Exponentiation failed");
    assert(accountAfterUpdate.computing_final_exponentiation == false, "Current instruction update updated wrong");
    assert(accountAfterUpdate.computing_miller_loop == false, "Current instruction update updated wrong");
    assert(accountAfterUpdate.computing_prepared_inputs == false, "Current instruction update updated wrong");
    assert(accountAfterUpdate.last_transaction == true, "Current instruction update updated wrong");
    assert(accountAfterUpdate.last_transaction == true, "Current instruction update updated wrong");
    assert_eq(accountAfterUpdate.rootHash, ix_data.rootHash, "rootHash insert wrong");
    assert_eq(accountAfterUpdate.amount, ix_data.amount, "amount insert wrong");
    assert_eq(accountAfterUpdate.txIntegrityHash, ix_data.txIntegrityHash, "txIntegrityHash insert wrong");
    assert_eq(accountAfterUpdate.extAmount, ix_data.extAmount, "extAmount insert wrong");
    assert_eq(accountAfterUpdate.signingAddress, relayer_pubkey, "relayer insert wrong");
    assert_eq(accountAfterUpdate.fee, ix_data.relayer_fee, "fee insert wrong");
    assert_eq(accountAfterUpdate.merkleTreeIndex, ix_data.merkleTreeIndex[0], "merkleTreeIndex insert wrong");

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
    merkleTreeUpdateState
  )

  assert(merkleTreeTmpAccountInfo.owner.toBase58() === merkleTreeProgram.programId.toBase58(), "merkle tree pda owner wrong after initializing")
  const merkleTreeUpdateStateData = merkleTreeProgram.account.preInsertedLeavesIndex._coder.accounts.decode('MerkleTreeUpdateState', merkleTreeTmpAccountInfo.data);

  var MerkleTreeAccountInfo = await connection.getAccountInfo(
    MerkleTree
  )
  assert(merkleTreeUpdateStateData.relayer.toBase58() == relayer.toBase58(), "The incorrect signer has been saved")
  assert(merkleTreeUpdateStateData.merkleTreePdaPubkey.toBase58()== MerkleTree.toBase58(), "the incorrect merkle tree pubkey was saved")
  assert(merkleTreeUpdateStateData.numberOfLeaves== leavesPdas.length, "The incorrect number of leaves was saved")
  assert(merkleTreeUpdateStateData.currentInstructionIndex== current_instruction_index, "The instruction index is wrong")
  assert(new solana.PublicKey(MerkleTreeAccountInfo.data.slice(16658-40,16658-8)).toBase58()== merkleTreeUpdateState.toBase58());
  assert(U64.readLE(MerkleTreeAccountInfo.data.slice(16658-8,16658), 0) >= (await connection.getSlot()) - 5, "Lock has not been taken at this or in the 5 prior slots");
}

export async function checkMerkleTreeBatchUpdateSuccess({
  connection,
  merkleTreeUpdateState,
  merkleTreeAccountPrior,
  numberOfLeaves,
  leavesPdas,
  merkleTree,
  merkle_tree_pubkey
}) {

  var merkleTreeTmpStateAccount = await connection.getAccountInfo(
        merkleTreeUpdateState
      )

  assert(merkleTreeTmpStateAccount === null, "Shielded transaction failed merkleTreeTmpStateAccount is not closed")

  var merkleTreeAccount = await connection.getAccountInfo(merkle_tree_pubkey)
  // Merkle tree is locked by merkleTreeUpdateState
  assert(new solana.PublicKey(merkleTreeAccount.data.slice(16658-40,16658-8)).toBase58()== new solana.PublicKey(new Uint8Array(32).fill(0)).toBase58());
  assert(U64.readLE(merkleTreeAccount.data.slice(16658-8,16658), 0) == 0, "Lock has not been taken within prior  20 slots");

  let merkle_tree_prior_leaves_index = U64.readLE(merkleTreeAccountPrior.data.slice(594, 594 + 8),0);
  let merkle_tree_prior_current_root_index = U64.readLE(merkleTreeAccountPrior.data.slice(594 - 8, 594),0).toNumber()

  let current_root_index = U64.readLE(merkleTreeAccount.data.slice(594 - 8, 594),0).toNumber()
  // console.log("merkle_tree_prior_current_root_index: ", merkle_tree_prior_current_root_index)
  // console.log("current_root_index: ", current_root_index)
  assert(merkle_tree_prior_current_root_index + 1 == current_root_index)
  let current_root_start_range = 610 + current_root_index * 32;
  let current_root_end_range = 610 + (current_root_index + 1) * 32;
  // console.log(`root: ${BigNumber.from(merkleTreeAccount.data.slice(current_root_start_range, current_root_end_range).reverse()).toHexString()}`)
  //
  // console.log(`prior +${numberOfLeaves} ${merkle_tree_prior_leaves_index.add(U64(numberOfLeaves)).toString()}, now ${U64.readLE(merkleTreeAccount.data.slice(594, 594 + 8), 0).toString()}
  // `)
  // index has increased by numberOfLeaves
  // console.log(`index has increased by numberOfLeaves: ${merkle_tree_prior_leaves_index.add(U64(numberOfLeaves)).toString()}, ${U64.readLE(merkleTreeAccount.data.slice(594, 594 + 8), 0).toString()}`)
  assert(merkle_tree_prior_leaves_index.add(U64(numberOfLeaves)).toString() == U64.readLE(merkleTreeAccount.data.slice(594, 594 + 8), 0).toString())

  let leavesPdasPubkeys = []
  leavesPdas.map( (pda) => { leavesPdasPubkeys.push(pda.pubkey) })
  var leavesAccounts = await connection.getMultipleAccountsInfo(
    leavesPdasPubkeys
      )
  let leaves_to_sort = []
  leavesAccounts.map((acc) => {
    // Checking that all leaves have been marked as inserted.
    assert(acc.data[1] == 4);
      leaves_to_sort.push({
        index: U64(acc.data.slice(2, 10)).toString(),
        leaves: acc.data.slice(10, 74),
      });
    });
  leaves_to_sort.sort((a, b) => parseFloat(a.index) - parseFloat(b.index));
  let numberOfLeavesPdas = 0
  for (var i = Number(merkle_tree_prior_leaves_index); i < Number(merkle_tree_prior_leaves_index) + Number(numberOfLeaves); i+=2) {
    merkleTree.update(i, BigNumber.from(leaves_to_sort[numberOfLeavesPdas].leaves.slice(0,32).reverse()))
    merkleTree.update(i + 1, BigNumber.from(leaves_to_sort[numberOfLeavesPdas].leaves.slice(32,64).reverse()))
    numberOfLeavesPdas++;
  }

  // Comparing root from chain with locally updated merkle tree.
  assert(BigNumber.from(merkleTreeAccount.data.slice(current_root_start_range, current_root_end_range).reverse()).toHexString(),
    merkleTree.root().toHexString()
  )
  // Comparing locally generated root with merkle tree built from leaves fetched from chain.
  assert(merkleTree.root().toHexString() == (await light.buildMerkelTree(connection, merkle_tree_pubkey.toBytes())).root().toHexString());

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
