// console.log("logs disabled -- remove top two lines in tests/tests.ts to enable logs");
// console.log = () => {}

import { assert } from "chai";
import * as anchor from "@coral-xyz/anchor";
var _ = require("lodash");
import {
  newAccountWithLamports,
  createMintWrapper,
  newAccountWithTokens,
} from "./createAccounts";
import { checkNfInserted } from "./testChecks";
import { Transaction } from "../transaction";
import { MINT } from "../index";
import { Keypair } from "@solana/web3.js";

// security claims
// - only the tokens of the mint included in the zkp can be withdrawn
// - only the amounts of the tokens in ZKP can be withdrawn
// - only the designated relayer can execute the transaction
// - relayer cannot alter recipient, recipientFee, relayer fee
// - amounts can only be withdrawn once
// -
export async function testTransaction({
  transaction,
  deposit = true,
  enabledSignerTest = true,
  provider,
  signer,
  ASSET_1_ORG,
  REGISTERED_VERIFIER_ONE_PDA,
  REGISTERED_VERIFIER_PDA,
}) {
  const origin = await newAccountWithLamports(provider.connection);
  console.log(transaction.verifier.publicInputs);
  console.log(transaction);

  const shieldedTxBackUp: Transaction = _.cloneDeep(transaction);
  console.log(
    "transaction.publicInputs.publicAmount ",
    transaction.publicInputs.publicAmount,
  );

  // Wrong pub amount
  let wrongAmount = new anchor.BN("123213").toArray();
  console.log("wrongAmount ", wrongAmount);

  transaction.publicInputs.publicAmount = Array.from([
    ...new Array(29).fill(0),
    ...wrongAmount,
  ]);
  let e = await transaction.sendTransaction();
  console.log(e);

  console.log(
    "Wrong wrongPubAmount",
    e.logs.includes("Program log: error ProofVerificationFailed"),
  );
  assert(e.logs.includes("Program log: error ProofVerificationFailed") == true);
  transaction.publicInputs.publicAmount = _.cloneDeep(
    shieldedTxBackUp.publicInputs.publicAmount,
  );
  await checkNfInserted(transaction.nullifierPdaPubkeys, provider.connection);
  // Wrong feeAmount
  let wrongFeeAmount = new anchor.BN("123213").toArray();
  console.log("wrongFeeAmount ", wrongFeeAmount);

  transaction.publicInputs.feeAmount = Array.from([
    ...new Array(29).fill(0),
    ...wrongFeeAmount,
  ]);
  e = await transaction.sendTransaction();
  console.log(
    "Wrong feeAmount",
    e.logs.includes("Program log: error ProofVerificationFailed"),
  );
  assert(e.logs.includes("Program log: error ProofVerificationFailed") == true);
  transaction.publicInputs = _.cloneDeep(shieldedTxBackUp.publicInputs);
  await checkNfInserted(transaction.nullifierPdaPubkeys, provider.connection);

  let wrongMint = new anchor.BN("123213").toArray();
  console.log("wrongMint ", ASSET_1_ORG.publicKey.toBase58());
  console.log("transaction.publicInputs ", transaction.publicInputs);
  let relayer = new anchor.web3.Account();
  await createMintWrapper({
    authorityKeypair: signer,
    mintKeypair: ASSET_1_ORG,
    connection: provider.connection,
  });
  transaction.sender = await newAccountWithTokens({
    connection: provider.connection,
    MINT: ASSET_1_ORG.publicKey,
    ADMIN_AUTH_KEYPAIR: signer,
    userAccount: relayer,
    amount: 0,
  });
  e = await transaction.sendTransaction();
  console.log(
    "Wrong wrongMint",
    e.logs.includes("Program log: error ProofVerificationFailed"),
  );
  assert(e.logs.includes("Program log: error ProofVerificationFailed") == true);
  transaction = _.cloneDeep(shieldedTxBackUp);
  await checkNfInserted(transaction.nullifierPdaPubkeys, provider.connection);

  console.log("transaction.sender: ", transaction.sender);

  // Wrong encryptedUtxos
  transaction.encryptedUtxos = new Uint8Array(174).fill(2);
  e = await transaction.sendTransaction();
  console.log(
    "Wrong encryptedUtxos",
    e.logs.includes("Program log: error ProofVerificationFailed"),
  );
  assert(e.logs.includes("Program log: error ProofVerificationFailed") == true);
  transaction.encryptedUtxos = _.cloneDeep(shieldedTxBackUp.encryptedUtxos);
  await checkNfInserted(transaction.nullifierPdaPubkeys, provider.connection);

  // Wrong relayerFee
  // will result in wrong integrity hash
  transaction.relayerFee = new anchor.BN("90");
  e = await transaction.sendTransaction();
  console.log(
    "Wrong relayerFee",
    e.logs.includes("Program log: error ProofVerificationFailed"),
  );
  assert(e.logs.includes("Program log: error ProofVerificationFailed") == true);
  transaction.relayerFee = _.cloneDeep(shieldedTxBackUp.relayerFee);
  await checkNfInserted(transaction.nullifierPdaPubkeys, provider.connection);

  for (var i in transaction.publicInputs.nullifiers) {
    transaction.publicInputs.nullifiers[i] = new Uint8Array(32).fill(2);
    e = await transaction.sendTransaction();
    console.log(
      "Wrong nullifier ",
      i,
      " ",
      e.logs.includes("Program log: error ProofVerificationFailed"),
    );
    assert(
      e.logs.includes("Program log: error ProofVerificationFailed") == true,
    );
    transaction = _.cloneDeep(shieldedTxBackUp);
    await checkNfInserted(transaction.nullifierPdaPubkeys, provider.connection);
  }

  for (var i: Number = 0; i < transaction.publicInputs.leaves.length; i++) {
    // Wrong leafLeft
    transaction.publicInputs.leaves[i][0] = new Uint8Array(32).fill(2);
    e = await transaction.sendTransaction();
    console.log(
      "Wrong leafLeft",
      e.logs.includes("Program log: error ProofVerificationFailed"),
    );
    assert(
      e.logs.includes("Program log: error ProofVerificationFailed") == true,
    );
    transaction.publicInputs.leaves[i] =
      _.cloneDeep(shieldedTxBackUp).publicInputs.leaves[i];
  }
  await checkNfInserted(transaction.nullifierPdaPubkeys, provider.connection);

  //
  // * -------- Checking Accounts -------------
  //
  if (enabledSignerTest) {
    // Wrong signingAddress
    // will result in wrong integrity hash
    // transaction.relayerPubkey = origin.publicKey;
    // transaction.payer = origin;
    // e = await transaction.sendTransaction();
    // console.log("Wrong signingAddress", e.logs.includes('Program log: error ProofVerificationFailed'));
    // assert(e.logs.includes('Program log: error ProofVerificationFailed') == true || e.logs.includes('Program log: AnchorError caused by account: signing_address. Error Code: ConstraintAddress. Error Number: 2012. Error Message: An address constraint was violated.') == true);
    // transaction.relayerPubkey = _.cloneDeep(shieldedTxBackUp.relayerPubkey);
    // transaction.payer = _.cloneDeep(shieldedTxBackUp.payer);
    // await checkNfInserted(  transaction.nullifierPdaPubkeys, provider.connection)
  }

  // probably above
  // Wrong recipient
  // will result in wrong integrity hash
  console.log("Wrong recipient ");

  if (deposit == true) {
    // transaction.recipient = MINT;
    // console.log("transaction.recipient ", transaction.recipient);

    // e = await transaction.sendTransaction();
    // console.log("Wrong recipient", e.logs.includes('Program log: error ProofVerificationFailed'));
    // assert(e.logs.includes('Program log: error ProofVerificationFailed') == true);
    // transaction.recipient = _.cloneDeep(shieldedTxBackUp.recipient);

    console.log("Wrong recipientFee ");
    // Wrong recipientFee
    // will result in wrong integrity hash
    // transaction.recipientFee = Keypair.generate().publicKey;
    console.log("transaction.recipientFee ", transaction.recipientFee);

    e = await transaction.sendTransaction();
    console.log("transaction.recipientFee ", transaction.recipientFee);

    console.log(
      "Wrong recipientFee",
      e.logs.includes("Program log: error ProofVerificationFailed"),
    );
    assert(
      e.logs.includes("Program log: error ProofVerificationFailed") == true,
    );
    transaction.recipientFee = _.cloneDeep(shieldedTxBackUp.recipientFee);
  } else {
    transaction.sender = origin.publicKey;
    e = await transaction.sendTransaction();
    console.log(
      "Wrong sender",
      e.logs.includes("Program log: error ProofVerificationFailed"),
    );
    assert(
      e.logs.includes("Program log: error ProofVerificationFailed") == true,
    );
    transaction.sender = _.cloneDeep(shieldedTxBackUp.sender);
    await checkNfInserted(transaction.nullifierPdaPubkeys, provider.connection);

    console.log("Wrong senderFee ");
    // Wrong recipientFee
    // will result in wrong integrity hash
    transaction.senderFee = origin.publicKey;
    e = await transaction.sendTransaction();
    console.log(e); // 546
    console.log(
      "Wrong senderFee",
      e.logs.includes(
        "Program log: AnchorError thrown in src/light_transaction.rs:696. Error Code: InvalidSenderorRecipient. Error Number: 6011. Error Message: InvalidSenderorRecipient.",
      ),
    );
    assert(
      e.logs.includes(
        "Program log: AnchorError thrown in src/light_transaction.rs:696. Error Code: InvalidSenderorRecipient. Error Number: 6011. Error Message: InvalidSenderorRecipient.",
      ) == true,
    );
    transaction.senderFee = _.cloneDeep(shieldedTxBackUp.senderFee);
    await checkNfInserted(transaction.nullifierPdaPubkeys, provider.connection);
  }

  console.log("Wrong registeredVerifierPda ");
  console.log(
    "transaction.verifier.registeredVerifierPda ",
    transaction.verifier.registeredVerifierPda,
  );
  console.log("REGISTERED_VERIFIER_ONE_PDA ", REGISTERED_VERIFIER_ONE_PDA);

  // Wrong registeredVerifierPda
  if (
    transaction.verifier.registeredVerifierPda.toBase58() ==
    REGISTERED_VERIFIER_ONE_PDA.toBase58()
  ) {
    transaction.verifier.registeredVerifierPda = REGISTERED_VERIFIER_PDA;
  } else {
    transaction.verifier.registeredVerifierPda = REGISTERED_VERIFIER_ONE_PDA;
  }
  console.log("here");

  e = await transaction.sendTransaction();
  console.log("here");

  console.log("Wrong registeredVerifierPda", e);
  assert(
    e.logs.includes(
      "Program log: AnchorError caused by account: registered_verifier_pda. Error Code: ConstraintSeeds. Error Number: 2006. Error Message: A seeds constraint was violated.",
    ) == true,
  );
  transaction.registeredVerifierPda = _.cloneDeep(
    shieldedTxBackUp.registeredVerifierPda,
  );
  await checkNfInserted(transaction.nullifierPdaPubkeys, provider.connection);

  console.log("Wrong authority ");
  // Wrong authority
  transaction.signerAuthorityPubkey = new anchor.web3.Account().publicKey;
  e = await transaction.sendTransaction();
  console.log(e);

  console.log(
    "Wrong authority1 ",
    e.logs.includes(
      "Program log: AnchorError caused by account: authority. Error Code: ConstraintSeeds. Error Number: 2006. Error Message: A seeds constraint was violated.",
    ),
  );
  assert(
    e.logs.includes(
      "Program log: AnchorError caused by account: authority. Error Code: ConstraintSeeds. Error Number: 2006. Error Message: A seeds constraint was violated.",
    ) == true,
  );
  transaction.signerAuthorityPubkey = _.cloneDeep(
    shieldedTxBackUp.signerAuthorityPubkey,
  );
  await checkNfInserted(transaction.nullifierPdaPubkeys, provider.connection);

  // console.log("Wrong preInsertedLeavesIndex ");
  // // Wrong authority
  // transaction.preInsertedLeavesIndex = transaction.tokenAuthority;
  // e = await transaction.sendTransaction();
  // console.log(e);
  // console.log("Wrong preInsertedLeavesIndex", e.logs.includes('Program log: AnchorError caused by account: authority. Error Code: ConstraintSeeds. Error Number: 2006. Error Message: A seeds constraint was violated.'));
  // assert(e.logs.includes('Program log: AnchorError caused by account: authority. Error Code: ConstraintSeeds. Error Number: 2006. Error Message: A seeds constraint was violated.') == true);
  // transaction.preInsertedLeavesIndex = _.cloneDeep(shieldedTxBackUp.preInsertedLeavesIndex);
  for (var j = 0; j < transaction.nullifierPdaPubkeys.length; j++) {
    console.log(
      "transaction.nullifierPdaPubkeys.length ",
      transaction.nullifierPdaPubkeys.length,
    );

    // Wrong authority
    transaction.nullifierPdaPubkeys[j] =
      transaction.nullifierPdaPubkeys[
        (j + 1) % transaction.nullifierPdaPubkeys.length
      ];
    assert(
      transaction.nullifierPdaPubkeys[j] !=
        shieldedTxBackUp.nullifierPdaPubkeys[j],
    );
    e = await transaction.sendTransaction();
    console.log(e);
    console.log(
      "transaction.nullifierPdaPubkeys[i] ",
      transaction.nullifierPdaPubkeys[j],
    );

    console.log(
      "Wrong nullifierPdaPubkeys ",
      j,
      " ",
      e.logs.includes(
        "Program log: Passed-in pda pubkey != on-chain derived pda pubkey.",
      ),
    );
    assert(
      e.logs.includes(
        "Program log: Passed-in pda pubkey != on-chain derived pda pubkey.",
      ) == true,
    );

    transaction.nullifierPdaPubkeys = _.cloneDeep(
      shieldedTxBackUp.nullifierPdaPubkeys,
    );
    assert(
      transaction.nullifierPdaPubkeys[j] ==
        shieldedTxBackUp.nullifierPdaPubkeys[j],
    );

    console.log(
      "transaction.nullifierPdaPubkeys[j] ",
      transaction.nullifierPdaPubkeys[j],
    );
  }
}
