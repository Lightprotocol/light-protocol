// console.log("logs disabled -- remove top two lines in tests/tests.ts to enable logs");
// console.log = () => {}

const solana = require("@solana/web3.js");
const {U64, I64} = require('n64');
import { assert, expect } from "chai";
import { BigNumber, providers } from 'ethers'
const light = require('../../light-protocol-sdk');
import * as anchor from "@project-serum/anchor";
const { SystemProgram } = require('@solana/web3.js');
const token = require('@solana/spl-token')
var _ = require('lodash');

// import {
//   read_and_parse_instruction_data_bytes,
//   parse_instruction_data_bytes,
//   readAndParseAccountDataMerkleTreeTmpState,
//   getPdaAddresses,
//   unpackLeavesAccount,
// } from "./unpack_accounts"

import {
  checkEscrowAccountCreated,
  checkVerifierStateAccountCreated,
  checkFinalExponentiationSuccess,
  checkMerkleTreeUpdateStateCreated,
  checkMerkleTreeBatchUpdateSuccess,
  checkRentExemption,
  assert_eq,
  checkNfInserted
} from "./test_checks";

import {
    DEFAULT_PROGRAMS,
} from "./constants"
const PREPARED_INPUTS_TX_COUNT = 42
const MILLER_LOOP_TX_COUNT = 42
const FINAL_EXPONENTIATION_TX_COUNT = 19
const MERKLE_TREE_UPDATE_TX_COUNT = 38

export const newAccountWithLamports = async (connection,account = new anchor.web3.Account(),lamports = 1e10) => {
  let x = await connection.confirmTransaction(await connection.requestAirdrop(account.publicKey, lamports), {
    commitment: 'comfirmed',
    preflightCommitment: 'comfirmed',
  });
  console.log("newAccountWithLamports ", account.publicKey.toBase58());

  return account;
}

export const newAddressWithLamports = async (connection,address = new anchor.web3.Account().publicKey, lamports = 1e11) => {

  let retries = 30
  await connection.requestAirdrop(address, lamports)
  for (;;) {
    await sleep(500)
    // eslint-disable-next-line eqeqeq
    if (lamports == (await connection.getBalance(address))) {
      console.log(`Airdropped ${lamports} to ${address.toBase58()}`)
      return address
    }
    if (--retries <= 0) {
      break
    }
  }
  throw new Error(`Airdrop of ${lamports} failed`)
}

export const newProgramOwnedAccount = async ({connection, owner, lamports = 0}) => {
  let account = new anchor.web3.Account();
  let payer = new anchor.web3.Account();
  let retry = 0;
  while(retry < 30){
    try{

      await connection.confirmTransaction(await connection.requestAirdrop(payer.publicKey, 1e7), {
        commitment: 'comfirmed',
        preflightCommitment: 'comfirmed',
      })


      const tx = new solana.Transaction().add(
        solana.SystemProgram.createAccount({
          fromPubkey: payer.publicKey,
          newAccountPubkey: account.publicKey,
          space: 0,
          lamports: await connection.getMinimumBalanceForRentExemption(1),
          programId: owner.programId,
        })
      );

      tx.feePayer = payer.publicKey
      tx.recentBlockhash = await connection.getRecentBlockhash();
      // tx.sign([payer])
      // console.log("getMinimumBalanceForRentExemption: ", )
      let x = await solana.sendAndConfirmTransaction(
            connection,
            tx,
            [payer, account],
            {
              commitment: 'confirmed',
              preflightCommitment: 'confirmed',
            },
        );
      return account;
    } catch {}

    retry ++;
  }
  throw "Can't create program account with lamports"
}

export async function executeUpdateMerkleTreeTransactions({
  signer,
  merkleTreeProgram,
  leavesPdas,
  merkleTree,
  merkleTreeIndex,
  merkle_tree_pubkey,
  connection,
  provider
}) {

var merkleTreeAccountPrior = await merkleTreeProgram.account.merkleTree.fetch(
  merkle_tree_pubkey
)
let merkleTreeUpdateState = (await solana.PublicKey.findProgramAddressSync(
    [Buffer.from(new Uint8Array(signer.publicKey.toBytes())), anchor.utils.bytes.utf8.encode("storage")],
    merkleTreeProgram.programId))[0];
    console.log("here0");

try {

  const tx1 = await merkleTreeProgram.methods.initializeMerkleTreeUpdateState(
    // new anchor.BN(merkleTreeIndex) // merkle tree index
  ).accounts(
    {
      authority: signer.publicKey,
      merkleTreeUpdateState: merkleTreeUpdateState,
      systemProgram: SystemProgram.programId,
      rent: DEFAULT_PROGRAMS.rent,
      merkleTree: merkle_tree_pubkey
    }
  ).remainingAccounts(
    leavesPdas
  ).preInstructions([
    solana.ComputeBudgetProgram.setComputeUnitLimit({units:1_400_000}),
  ]).signers([signer]).rpc({
    commitment: 'finalized',
    preflightCommitment: 'finalized',
  })
} catch(e) {
  console.log(" init Merkle tree update", e);

}
  console.log("here1");

  await checkMerkleTreeUpdateStateCreated({
    connection: connection,
    merkleTreeUpdateState,
    MerkleTree: merkle_tree_pubkey,
    relayer: signer.publicKey,
    leavesPdas,
    current_instruction_index: 1,
    merkleTreeProgram
  })

  await executeMerkleTreeUpdateTransactions({
    signer,
    merkleTreeProgram,
    merkle_tree_pubkey,
    provider,
    merkleTreeUpdateState,
    numberOfTransactions: 251
  })

  await checkMerkleTreeUpdateStateCreated({
    connection: connection,
    merkleTreeUpdateState,
    MerkleTree: merkle_tree_pubkey,
    relayer: signer.publicKey,
    leavesPdas,
    current_instruction_index: 56,
    merkleTreeProgram
  })
  // final tx to insert root
  let success = false;
  try {
      await merkleTreeProgram.methods.insertRootMerkleTree(
        new anchor.BN(254))
      .accounts({
        authority: signer.publicKey,
        merkleTreeUpdateState: merkleTreeUpdateState,
        merkleTree: merkle_tree_pubkey
      }).remainingAccounts(
        leavesPdas
      ).signers([signer]).rpc({
        commitment: 'finalized',
        preflightCommitment: 'finalized',
      })
  } catch (e) {
    console.log(e)
  }

  await checkMerkleTreeBatchUpdateSuccess({
    connection: connection,
    merkleTreeUpdateState: merkleTreeUpdateState,
    merkleTreeAccountPrior,
    numberOfLeaves: leavesPdas.length * 2,
    leavesPdas,
    merkleTree: merkleTree,
    merkle_tree_pubkey: merkle_tree_pubkey,
    merkleTreeProgram
  })
}

export async function executeMerkleTreeUpdateTransactions({
  merkleTreeProgram,
  merkleTreeUpdateState,
  merkle_tree_pubkey,
  provider,
  signer,
  numberOfTransactions
}) {
  let arr = []
  let i = 0;
  // console.log("Sending Merkle tree update transactions: ",numberOfTransactions)
  // the number of tx needs to increase with greater batchsize
  // 29 + 2 * leavesPdas.length is a first approximation
  for(let ix_id = 0; ix_id < numberOfTransactions; ix_id ++) {

    const transaction = new solana.Transaction();
    transaction.add(
      solana.ComputeBudgetProgram.setComputeUnitLimit({units:1_400_000}),

    )
    transaction.add(
      await merkleTreeProgram.methods.updateMerkleTree(new anchor.BN(i))
      .accounts({
        authority: signer.publicKey,
        merkleTreeUpdateState: merkleTreeUpdateState,
        merkleTree: merkle_tree_pubkey
      }).instruction()
    )
    i+=1;
    transaction.add(
      await merkleTreeProgram.methods.updateMerkleTree(new anchor.BN(i)).accounts({
        authority: signer.publicKey,
        merkleTreeUpdateState: merkleTreeUpdateState,
        merkleTree: merkle_tree_pubkey
      }).instruction()
    )
    i+=1;

    arr.push({tx:transaction, signers: [signer]})
  }
  let error
  await Promise.all(arr.map(async (tx, index) => {

  try {
      await provider.sendAndConfirm(tx.tx, tx.signers,{
        commitment: 'finalized',
        preflightCommitment: 'finalized',
      });
  } catch(e) {
      console.log(e);
      error =  e;
  }

  }));
  return error;
}


export async function newAccountWithTokens ({
  connection,
  MINT,
  ADMIN_AUTH_KEYPAIR,
  userAccount,
  amount
}) {
  // const tokenAccount = await token.getAssociatedTokenAddress(
  //     MINT,
  //     userAccount.publicKey,
  //     false,
  //     token.TOKEN_PROGRAM_ID,
  //     token.ASSOCIATED_TOKEN_PROGRAM_ID
  // );
  // console.log("tokenAccount ", tokenAccount);
  let tokenAccount
  try {
    console.log("userAccount.publicKey: ", userAccount.publicKey.toBase58());

    // var tokenAccount = await token.createAssociatedTokenAccount(
    //   connection,
    //   userAccount,
    //   MINT,
    //   userAccount.publicKey
    // );
    // console.log(ADMIN_AUTH_KEYPAIR.publicKey.toBase58());
    // console.log(tokenAccount.toBase58());
    // console.log(MINT);
    //
    // const transaction = new solana.Transaction().add(
    //     token.createAssociatedTokenAccountInstruction(
    //         ADMIN_AUTH_KEYPAIR.publicKey,
    //         tokenAccount,
    //         userAccount.publicKey,
    //         MINT,
    //         // token.TOKEN_PROGRAM_ID,
    //         // token.ASSOCIATED_TOKEN_PROGRAM_ID
    //     )
    // );
    // console.log(transaction);
    // await solana.sendAndConfirmTransaction(connection, transaction, [ADMIN_AUTH_KEYPAIR]);

    let space = token.ACCOUNT_SIZE
    console.log(MINT);
    console.log(ADMIN_AUTH_KEYPAIR.publicKey.toBase58());

    tokenAccount = await   token.createAccount(
        connection,
        ADMIN_AUTH_KEYPAIR,
        MINT,
        userAccount.publicKey,
        // userAccount
      )

      console.log(tokenAccount);

  } catch (e) {
    console.log(e);
  }
  console.log("fere");

  try{

    await token.mintTo(
      connection,
      ADMIN_AUTH_KEYPAIR,
      MINT,
      tokenAccount,
      ADMIN_AUTH_KEYPAIR.publicKey,
      amount,
      []
    );
  } catch (e) {
    console.log(e);

  }

 return tokenAccount;
}

export async function createMint({authorityKeypair, mintKeypair = new anchor.web3.Account(),nft = false, decimals = 2, provider}) {
  if (nft == true) {
    decimals = 0;
  }
  // await provider.connection.confirmTransaction(await provider.connection.requestAirdrop(mintKeypair.publicKey, 1_000_000, {preflightCommitment: "confirmed", commitment: "confirmed"}));

  try {
    let space = token.MINT_SIZE

    let txCreateAccount = new solana.Transaction().add(
      SystemProgram.createAccount({
        fromPubkey: authorityKeypair.publicKey,
        lamports: provider.connection.getMinimumBalanceForRentExemption(space),
        newAccountPubkey: mintKeypair.publicKey,
        programId: token.TOKEN_PROGRAM_ID,
        space: space

      })
    )

    let res = await solana.sendAndConfirmTransaction(provider.connection, txCreateAccount, [authorityKeypair, mintKeypair], {commitment: "finalized", preflightCommitment: 'finalized',});

    let mint = await token.createMint(
      provider.connection,
      authorityKeypair,
      authorityKeypair.publicKey,
      null, // freez auth
      decimals, //2,
      mintKeypair
    );
    // assert(MINT.toBase58() == mint.toBase58());
    console.log("mintKeypair.publicKey: ", mintKeypair.publicKey.toBase58());
    return mintKeypair.publicKey;
  } catch(e) {
    console.log(e)
  }

}

// security claims
// - only the tokens of the mint included in the zkp can be withdrawn
// - only the amounts of the tokens in ZKP can be withdrawn
// - only the designated relayer can execute the transaction
// - relayer cannot alter recipient, recipientFee, relayer fee
// - amounts can only be withdrawn once
// -
export async function testTransaction({SHIELDED_TRANSACTION, deposit = true, enabledSignerTest = true, provider, signer, ASSET_1_ORG, REGISTERED_VERIFIER_ONE_PDA, REGISTERED_VERIFIER_PDA}) {
  const origin = await newAccountWithLamports(provider.connection)

  const shieldedTxBackUp = _.cloneDeep(SHIELDED_TRANSACTION);
  console.log("SHIELDED_TRANSACTION.proofData.publicInputs.publicAmount ", SHIELDED_TRANSACTION.proofData.publicInputs.publicAmount);

  // Wrong pub amount
  let wrongAmount = new anchor.BN("123213").toArray()
  console.log("wrongAmount ", wrongAmount);

  SHIELDED_TRANSACTION.proofData.publicInputs.publicAmount = Array.from([...new Array(29).fill(0), ...wrongAmount]);
  let e = await SHIELDED_TRANSACTION.sendTransaction();
  console.log(e);

  console.log("Wrong wrongPubAmount", e.logs.includes('Program log: error ProofVerificationFailed'));
  assert(e.logs.includes('Program log: error ProofVerificationFailed') == true);
  SHIELDED_TRANSACTION.proofData = _.cloneDeep(shieldedTxBackUp.proofData);
  await checkNfInserted(  SHIELDED_TRANSACTION.nullifierPdaPubkeys, provider.connection)
  // Wrong feeAmount
  let wrongFeeAmount = new anchor.BN("123213").toArray()
  console.log("wrongFeeAmount ", wrongFeeAmount);

  SHIELDED_TRANSACTION.proofData.publicInputs.feeAmount = Array.from([...new Array(29).fill(0), ...wrongFeeAmount]);
  e = await SHIELDED_TRANSACTION.sendTransaction();
  console.log("Wrong feeAmount", e.logs.includes('Program log: error ProofVerificationFailed'));
  assert(e.logs.includes('Program log: error ProofVerificationFailed') == true);
  SHIELDED_TRANSACTION.proofData = _.cloneDeep(shieldedTxBackUp.proofData);
  await checkNfInserted(  SHIELDED_TRANSACTION.nullifierPdaPubkeys, provider.connection)

  let wrongMint = new anchor.BN("123213").toArray()
  console.log("wrongMint ", wrongMint);
  console.log("SHIELDED_TRANSACTION.proofData.publicInputs ", SHIELDED_TRANSACTION.proofData.publicInputs);
  let relayer = new anchor.web3.Account();
  await createMint({
    authorityKeypair: signer,
    mintKeypair: ASSET_1_ORG,
    provider
  })
  SHIELDED_TRANSACTION.sender = await newAccountWithTokens({connection: provider.connection,
  MINT: ASSET_1_ORG.publicKey,
  ADMIN_AUTH_KEYPAIR: signer,
  userAccount: relayer,
  amount: 0})
  e = await SHIELDED_TRANSACTION.sendTransaction();
  console.log("Wrong wrongMint", e.logs.includes('Program log: error ProofVerificationFailed'));
  assert(e.logs.includes('Program log: error ProofVerificationFailed') == true);
  SHIELDED_TRANSACTION.sender = _.cloneDeep(shieldedTxBackUp.sender);
  await checkNfInserted(  SHIELDED_TRANSACTION.nullifierPdaPubkeys, provider.connection)

  // Wrong encryptedOutputs
  SHIELDED_TRANSACTION.proofData.encryptedOutputs = new Uint8Array(174).fill(2);
  e = await SHIELDED_TRANSACTION.sendTransaction();
  console.log("Wrong encryptedOutputs", e.logs.includes('Program log: error ProofVerificationFailed'));
  assert(e.logs.includes('Program log: error ProofVerificationFailed') == true);
  SHIELDED_TRANSACTION.proofData = _.cloneDeep(shieldedTxBackUp.proofData);
  await checkNfInserted(  SHIELDED_TRANSACTION.nullifierPdaPubkeys, provider.connection)

  // Wrong relayerFee
  // will result in wrong integrity hash
  SHIELDED_TRANSACTION.relayerFee = new anchor.BN("90");
  e = await SHIELDED_TRANSACTION.sendTransaction();
  console.log("Wrong relayerFee", e.logs.includes('Program log: error ProofVerificationFailed'));
  assert(e.logs.includes('Program log: error ProofVerificationFailed') == true);
  SHIELDED_TRANSACTION.relayerFee = _.cloneDeep(shieldedTxBackUp.relayerFee);
  await checkNfInserted(  SHIELDED_TRANSACTION.nullifierPdaPubkeys, provider.connection)

  for (var i in SHIELDED_TRANSACTION.proofData.publicInputs.nullifiers) {
    SHIELDED_TRANSACTION.proofData.publicInputs.nullifiers[i] = new Uint8Array(32).fill(2);
    e = await SHIELDED_TRANSACTION.sendTransaction();
    console.log("Wrong nullifier ", i, " ", e.logs.includes('Program log: error ProofVerificationFailed'));
    assert(e.logs.includes('Program log: error ProofVerificationFailed') == true);
    SHIELDED_TRANSACTION.proofData = _.cloneDeep(shieldedTxBackUp.proofData);
    await checkNfInserted(  SHIELDED_TRANSACTION.nullifierPdaPubkeys, provider.connection)

  }

  for (var i = 0; i < SHIELDED_TRANSACTION.proofData.publicInputs.leaves.length; i++) {
    // Wrong leafLeft
    SHIELDED_TRANSACTION.proofData.publicInputs.leaves[i] = new Uint8Array(32).fill(2);
    e = await SHIELDED_TRANSACTION.sendTransaction();
    console.log("Wrong leafLeft", e.logs.includes('Program log: error ProofVerificationFailed'));
    assert(e.logs.includes('Program log: error ProofVerificationFailed') == true);
    SHIELDED_TRANSACTION.proofData = _.cloneDeep(shieldedTxBackUp.proofData);
  }
  await checkNfInserted(  SHIELDED_TRANSACTION.nullifierPdaPubkeys, provider.connection)

  /**
  * -------- Checking Accounts -------------
  **/
  if (enabledSignerTest) {
    // Wrong signingAddress
    // will result in wrong integrity hash
    SHIELDED_TRANSACTION.relayerPubkey = origin.publicKey;
    SHIELDED_TRANSACTION.payer = origin;
    e = await SHIELDED_TRANSACTION.sendTransaction();
    console.log("Wrong signingAddress", e.logs.includes('Program log: error ProofVerificationFailed'));
    assert(e.logs.includes('Program log: error ProofVerificationFailed') == true || e.logs.includes('Program log: AnchorError caused by account: signing_address. Error Code: ConstraintAddress. Error Number: 2012. Error Message: An address constraint was violated.') == true);
    SHIELDED_TRANSACTION.relayerPubkey = _.cloneDeep(shieldedTxBackUp.relayerPubkey);
    SHIELDED_TRANSACTION.payer = _.cloneDeep(shieldedTxBackUp.payer);
    await checkNfInserted(  SHIELDED_TRANSACTION.nullifierPdaPubkeys, provider.connection)

  }

  // Wrong recipient
  // will result in wrong integrity hash
  console.log("Wrong recipient ");

  if (deposit == true) {

    SHIELDED_TRANSACTION.recipient = origin.publicKey;
    e = await SHIELDED_TRANSACTION.sendTransaction();
    console.log("Wrong recipient", e.logs.includes('Program log: error ProofVerificationFailed'));
    assert(e.logs.includes('Program log: error ProofVerificationFailed') == true);
    SHIELDED_TRANSACTION.recipient = _.cloneDeep(shieldedTxBackUp.recipient);

    console.log("Wrong recipientFee ");
    // Wrong recipientFee
    // will result in wrong integrity hash
    SHIELDED_TRANSACTION.recipientFee = origin.publicKey;
    e = await SHIELDED_TRANSACTION.sendTransaction();
    console.log("Wrong recipientFee", e.logs.includes('Program log: error ProofVerificationFailed'));
    assert(e.logs.includes('Program log: error ProofVerificationFailed') == true);
    SHIELDED_TRANSACTION.recipientFee = _.cloneDeep(shieldedTxBackUp.recipientFee);
  } else {
    SHIELDED_TRANSACTION.sender = origin.publicKey;
    e = await SHIELDED_TRANSACTION.sendTransaction();
    console.log("Wrong sender", e.logs.includes('Program log: error ProofVerificationFailed'));
    assert(e.logs.includes('Program log: error ProofVerificationFailed') == true);
    SHIELDED_TRANSACTION.sender = _.cloneDeep(shieldedTxBackUp.sender);
    await checkNfInserted(  SHIELDED_TRANSACTION.nullifierPdaPubkeys, provider.connection)

    console.log("Wrong senderFee ");
    // Wrong recipientFee
    // will result in wrong integrity hash
    SHIELDED_TRANSACTION.senderFee = origin.publicKey;
    e = await SHIELDED_TRANSACTION.sendTransaction();
    console.log(e); // 546
    console.log("Wrong senderFee", e.logs.includes('Program log: AnchorError thrown in src/light_transaction.rs:696. Error Code: InvalidSenderorRecipient. Error Number: 6011. Error Message: InvalidSenderorRecipient.'));
    assert(e.logs.includes('Program log: AnchorError thrown in src/light_transaction.rs:696. Error Code: InvalidSenderorRecipient. Error Number: 6011. Error Message: InvalidSenderorRecipient.') == true);
    SHIELDED_TRANSACTION.senderFee = _.cloneDeep(shieldedTxBackUp.senderFee);
    await checkNfInserted(  SHIELDED_TRANSACTION.nullifierPdaPubkeys, provider.connection)

  }

  console.log("Wrong registeredVerifierPda ");
  // Wrong registeredVerifierPda
  if (SHIELDED_TRANSACTION.registeredVerifierPda.toBase58() == REGISTERED_VERIFIER_ONE_PDA.toBase58()) {
    SHIELDED_TRANSACTION.registeredVerifierPda = REGISTERED_VERIFIER_PDA

  } else {

    SHIELDED_TRANSACTION.registeredVerifierPda = REGISTERED_VERIFIER_ONE_PDA
  }
  e = await SHIELDED_TRANSACTION.sendTransaction();
  console.log("Wrong registeredVerifierPda",e);
  assert(e.logs.includes('Program log: AnchorError caused by account: registered_verifier_pda. Error Code: ConstraintSeeds. Error Number: 2006. Error Message: A seeds constraint was violated.') == true);
  SHIELDED_TRANSACTION.registeredVerifierPda = _.cloneDeep(shieldedTxBackUp.registeredVerifierPda);
  await checkNfInserted(  SHIELDED_TRANSACTION.nullifierPdaPubkeys, provider.connection)

  console.log("Wrong authority ");
  // Wrong authority
  SHIELDED_TRANSACTION.signerAuthorityPubkey = new anchor.web3.Account().publicKey;
  e = await SHIELDED_TRANSACTION.sendTransaction();
  console.log(e);

  console.log("Wrong authority1 ", e.logs.includes('Program log: AnchorError caused by account: authority. Error Code: ConstraintSeeds. Error Number: 2006. Error Message: A seeds constraint was violated.'));
  assert(e.logs.includes('Program log: AnchorError caused by account: authority. Error Code: ConstraintSeeds. Error Number: 2006. Error Message: A seeds constraint was violated.') == true);
  SHIELDED_TRANSACTION.signerAuthorityPubkey = _.cloneDeep(shieldedTxBackUp.signerAuthorityPubkey);
  await checkNfInserted(  SHIELDED_TRANSACTION.nullifierPdaPubkeys, provider.connection)

  // console.log("Wrong preInsertedLeavesIndex ");
  // // Wrong authority
  // SHIELDED_TRANSACTION.preInsertedLeavesIndex = SHIELDED_TRANSACTION.tokenAuthority;
  // e = await SHIELDED_TRANSACTION.sendTransaction();
  // console.log(e);
  // console.log("Wrong preInsertedLeavesIndex", e.logs.includes('Program log: AnchorError caused by account: authority. Error Code: ConstraintSeeds. Error Number: 2006. Error Message: A seeds constraint was violated.'));
  // assert(e.logs.includes('Program log: AnchorError caused by account: authority. Error Code: ConstraintSeeds. Error Number: 2006. Error Message: A seeds constraint was violated.') == true);
  // SHIELDED_TRANSACTION.preInsertedLeavesIndex = _.cloneDeep(shieldedTxBackUp.preInsertedLeavesIndex);
  for (var i = 0; i < SHIELDED_TRANSACTION.nullifierPdaPubkeys.length; i++) {
    console.log("SHIELDED_TRANSACTION.nullifierPdaPubkeys.length ", SHIELDED_TRANSACTION.nullifierPdaPubkeys.length);

    // Wrong authority
    SHIELDED_TRANSACTION.nullifierPdaPubkeys[i] = SHIELDED_TRANSACTION.nullifierPdaPubkeys[(i + 1) % (SHIELDED_TRANSACTION.nullifierPdaPubkeys.length)];
    assert(SHIELDED_TRANSACTION.nullifierPdaPubkeys[i] != shieldedTxBackUp.nullifierPdaPubkeys[i]);
    e = await SHIELDED_TRANSACTION.sendTransaction();
    console.log(e);

    console.log("Wrong nullifierPdaPubkeys ", i," ", e.logs.includes('Program log: Passed-in pda pubkey != on-chain derived pda pubkey.'));
    assert(e.logs.includes('Program log: Passed-in pda pubkey != on-chain derived pda pubkey.') == true);
    SHIELDED_TRANSACTION.nullifierPdaPubkeys[i] = _.cloneDeep(shieldedTxBackUp.nullifierPdaPubkeys[i]);
  }
}
