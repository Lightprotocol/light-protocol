const solana = require("@solana/web3.js");
const {U64, I64} = require('n64');
import { assert, expect } from "chai";
import { BigNumber, providers } from 'ethers'
const light = require('../../light-protocol-sdk');
import * as anchor from "@project-serum/anchor";
const { SystemProgram } = require('@solana/web3.js');
const token = require('@solana/spl-token')

import {
  read_and_parse_instruction_data_bytes,
  parse_instruction_data_bytes,
  readAndParseAccountDataMerkleTreeTmpState,
  getPdaAddresses,
  unpackLeavesAccount,
} from "./unpack_accounts"

import {
  checkEscrowAccountCreated,
  checkVerifierStateAccountCreated,
  checkFinalExponentiationSuccess,
  checkLastTxSuccess,
  checkMerkleTreeUpdateStateCreated,
  checkMerkleTreeBatchUpdateSuccess,
  checkRentExemption,
  assert_eq
} from "./test_checks";

import {
    DEFAULT_PROGRAMS,
} from "./constants"
const PREPARED_INPUTS_TX_COUNT = 42
const MILLER_LOOP_TX_COUNT = 42
const FINAL_EXPONENTIATION_TX_COUNT = 19
const MERKLE_TREE_UPDATE_TX_COUNT = 38

export const newAccountWithLamports = async (connection,account = new anchor.web3.Account(),lamports = 1e13) => {
  await connection.confirmTransaction(await connection.requestAirdrop(account.publicKey, lamports))
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
      await connection.confirmTransaction(
        await connection.requestAirdrop(payer.publicKey, 1e13)
      )

      const tx = new solana.Transaction().add(
        solana.SystemProgram.createAccount({
          fromPubkey: payer.publicKey,
          newAccountPubkey: account.publicKey,
          space: 0,
          lamports: await connection.getMinimumBalanceForRentExemption(0),
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
              commitment: 'singleGossip',
              preflightCommitment: 'singleGossip',
            },
        );
      return account;
    } catch {}

    retry ++;
  }
  throw "Can't create program account with lamports"
}

export async function deposit({
  Keypair,
  encryptionKeypair,
  MINT,
  amount, // 1_000_000_00
  connection,
  merkleTree,
  merkleTreeIndex,
  merkleTreePdaToken,
  userAccount,
  userAccountToken,
  verifierProgram,
  merkleTreeProgram,
  authority,
  preInsertedLeavesIndex,
  merkle_tree_pubkey,
  provider,
  relayerFee,
  lastTx = true,
  is_token = false,
  rent
}) {
  const burnerUserAccount = await newAccountWithLamports(connection)
  throw "fn deposit still creates old utxos";
  let deposit_utxo1 = new light.Utxo(BigNumber.from(amount), Keypair)
  let deposit_utxo2 = new light.Utxo(BigNumber.from(amount), Keypair)

  let inputUtxos = [new light.Utxo(), new light.Utxo()]
  let outputUtxos = [deposit_utxo1, deposit_utxo2 ]
  var merkleTreeIndex
  if (!is_token) {
    merkleTreeIndex = 0
  } else if (is_token) {
    merkleTreeIndex = 1
  }
  const data = await light.getProof(
    inputUtxos,
    outputUtxos,
    merkleTree,
    merkleTreeIndex,
    merkle_tree_pubkey.toBytes(),
    deposit_utxo1.amount.add(deposit_utxo2.amount),
    U64(0),
    merkleTreePdaToken.toBase58(),
    burnerUserAccount.publicKey.toBase58(),
    'DEPOSIT',
    encryptionKeypair
  )
  let ix_data = parse_instruction_data_bytes(data);
  let pdas = getPdaAddresses({
    tx_integrity_hash: ix_data.txIntegrityHash,
    nullifier0: ix_data.nullifier0,
    nullifier1: ix_data.nullifier1,
    leafLeft: ix_data.leafLeft,
    merkleTreeProgram,
    verifierProgram
  })
  if (is_token == true) {
    await token.approve(
      provider.connection,
      userAccount,
      userAccountToken,
      authority, //delegate
      userAccount.publicKey, // owner
      I64.readLE(ix_data.extAmount,0).toNumber(), // amount
      []
    )
  }
  let leavesPda = await transact({
    connection: connection,
    MINT,
    ix_data,
    pdas,
    origin: userAccount,
    origin_token: userAccountToken,
    signer: burnerUserAccount,
    recipient: merkleTreePdaToken,
    batch_insert: true,
    mode: "deposit",
    verifierProgram,
    merkleTreeProgram,
    merkleTreePdaToken,
    authority,
    preInsertedLeavesIndex,
    merkle_tree_pubkey,
    provider,
    relayerFee,
    lastTx,
    is_token,
    createEscrow: true,
    rent
  })

  return [leavesPda, outputUtxos, burnerUserAccount, pdas];
}

export async function transact({
  connection,
  ix_data,
  pdas,
  origin,
  origin_token,
  MINT,
  signer,
  recipient,
  relayer_recipient,
  batch_insert,
  mode,
  verifierProgram,
  merkleTreeProgram,
  authority,
  preInsertedLeavesIndex,
  merkle_tree_pubkey,
  provider,
  relayerFee,
  lastTx,
  createEscrow = true,
  is_token,
  rent
}) {
  if (lastTx == undefined) {
    lastTx = true
  }
  if (is_token == undefined) {
    is_token = false
  }
  // tx fee in lamports
  let tx_fee = 5000 * PREPARED_INPUTS_TX_COUNT + MILLER_LOOP_TX_COUNT + FINAL_EXPONENTIATION_TX_COUNT + 2* MERKLE_TREE_UPDATE_TX_COUNT;
  var userAccountPriorLastTx;
  let senderAccountBalancePriorLastTx
  let escrowTokenAccount
  let recipientBalancePriorLastTx

  console.log("mode: " ,mode, "is_token: ", is_token);

  if (mode === "deposit"&& is_token == false) {
    userAccountPriorLastTx = await connection.getAccountInfo(
          origin.publicKey
        )
    senderAccountBalancePriorLastTx = userAccountPriorLastTx.lamports;
    var recipientAccountPriorLastTx = await connection.getAccountInfo(
          recipient
        )
    recipientBalancePriorLastTx = recipientAccountPriorLastTx != null ? recipientAccountPriorLastTx.lamports : 0;

  } else if (mode === "deposit"&& is_token == true) {
    senderAccountBalancePriorLastTx = (await token.getAccount(
      provider.connection,
      origin_token,
      token.TOKEN_PROGRAM_ID
    )).amount;
    recipientBalancePriorLastTx = (await token.getAccount(
      provider.connection,
      recipient,
      token.TOKEN_PROGRAM_ID
    )).amount;
  } else if (mode === "withdrawal"&& is_token == false) {

    userAccountPriorLastTx = await connection.getAccountInfo(
          origin
        )

    senderAccountBalancePriorLastTx = userAccountPriorLastTx.lamports;
    var recipientAccountPriorLastTx = await connection.getAccountInfo(
          recipient
        )

    recipientBalancePriorLastTx = recipientAccountPriorLastTx != null ? recipientAccountPriorLastTx.lamports : 0;

  } else if (mode === "withdrawal"&& is_token == true) {

    senderAccountBalancePriorLastTx = (await token.getAccount(
      provider.connection,
      origin_token,
      token.TOKEN_PROGRAM_ID
    )).amount;
    recipientBalancePriorLastTx = (await token.getAccount(
      provider.connection,
      recipient,
      token.TOKEN_PROGRAM_ID
    )).amount;
  }

  if (mode === "deposit"&& createEscrow == true && is_token == false) {
    console.log("creating escrow")
    // create escrow account
    const tx = await verifierProgram.methods.createEscrow(
          ix_data.txIntegrityHash,
          new anchor.BN(tx_fee), // does not need to be checked since this tx is signed by the user
          ix_data.fee,
          new anchor.BN(I64.readLE(ix_data.extAmount,0).toString()),
          new anchor.BN(0)
        ).accounts(
              {
                signingAddress: signer.publicKey,
                verifierState: pdas.verifierStatePubkey,
                systemProgram: SystemProgram.programId,
                feeEscrowState: pdas.feeEscrowStatePubkey,
                user:           origin.publicKey,
                systemProgram: SystemProgram.programId,
                token_program: token.TOKEN_PROGRAM_ID,
                tokenAuthority: authority
              }
            ).signers([signer, origin]).rpc();

      await checkEscrowAccountCreated({
        connection:connection,
        pdas,
        ix_data,
        user_pubkey: origin.publicKey,
        relayer_pubkey: signer.publicKey,
        tx_fee: new anchor.BN(tx_fee),//check doesn t work
        verifierProgram,
        rent
      });
  } else if (mode === "deposit"&& createEscrow == true && is_token == true) {
    escrowTokenAccount = await solana.PublicKey.createWithSeed(
      signer.publicKey,
      "escrow",
      token.TOKEN_PROGRAM_ID,
    );
    let tokenAuthority = solana.PublicKey.findProgramAddressSync(
        [anchor.utils.bytes.utf8.encode("spl")],
        verifierProgram.programId
      )[0];
    let amount = U64.readLE(ix_data.extAmount, 0).toNumber()

     try {
       const tx = await verifierProgram.methods.createEscrow(
             ix_data.txIntegrityHash,
             new anchor.BN(tx_fee), // does not need to be checked since this tx is signed by the user
             ix_data.fee,
             new anchor.BN(amount),
             new anchor.BN(1)
       ).accounts(
           {
             feeEscrowState: pdas.feeEscrowStatePubkey,
             verifierState:  pdas.verifierStatePubkey,
             signingAddress: signer.publicKey,
             user:           origin.publicKey,
             systemProgram:  SystemProgram.programId,
             tokenProgram:  token.TOKEN_PROGRAM_ID,
             tokenAuthority: authority//tokenAuthority
           }
         ).remainingAccounts([
           { isSigner: false, isWritable: true, pubkey: origin_token},
           { isSigner: false, isWritable: true, pubkey:escrowTokenAccount }
         ]).preInstructions([
           SystemProgram.createAccountWithSeed({
             basePubkey:signer.publicKey,
             seed: anchor.utils.bytes.utf8.encode("escrow"),
             fromPubkey: signer.publicKey,
             newAccountPubkey: escrowTokenAccount,
             space: token.ACCOUNT_SIZE,
             lamports: await provider.connection.getMinimumBalanceForRentExemption(token.ACCOUNT_SIZE),
             programId: token.TOKEN_PROGRAM_ID
           }),
           token.createInitializeAccountInstruction(
            escrowTokenAccount, //new account
            MINT, // mint
            authority,
            tokenAuthority, //owner
          )
        ]).signers([signer, origin]).transaction();
        tx.instructions[1].programId = token.TOKEN_PROGRAM_ID
        await provider.sendAndConfirm(tx, [signer, origin]);
     } catch (e) {
       console.log("e createEscrow", e)
     }
  }
  try  {
      const tx = await verifierProgram.methods.createVerifierState(
          ix_data.proofAbc,
          ix_data.rootHash,
          ix_data.amount,
          ix_data.txIntegrityHash,
          ix_data.nullifier0,
          ix_data.nullifier1,
          ix_data.leafRight,
          ix_data.leafLeft,
          ix_data.recipient,
          ix_data.extAmount,
          ix_data.relayer,
          ix_data.fee,
          ix_data.encryptedUtxos,
          ix_data.merkleTreeIndex
          ).accounts(
              {
                signingAddress: signer.publicKey,
                verifierState: pdas.verifierStatePubkey,
                systemProgram: SystemProgram.programId,
                merkleTree: merkle_tree_pubkey,
                programMerkleTree:  merkleTreeProgram.programId,
              }
          ).signers([signer]).rpc()

  } catch(e) {
    console.log(e)
    process.exit()
  }

  checkVerifierStateAccountCreated({
    connection:connection,
    pda: pdas.verifierStatePubkey,
    ix_data,
    relayer_pubkey:signer.publicKey
  })
  console.log("Verifier State Account created");

  await executeXComputeTransactions({
    number_of_transactions: PREPARED_INPUTS_TX_COUNT + MILLER_LOOP_TX_COUNT + FINAL_EXPONENTIATION_TX_COUNT + 1 - 4 ,// final exp executes 4 to many
    signer: signer,
    pdas: pdas,
    program: verifierProgram,
    provider:provider
  })
  checkFinalExponentiationSuccess({
    pda: pdas.verifierStatePubkey,
    connection: connection,
    ix_data,
    verifierProgram
  })

  console.log("Compute Instructions Executed");
  let tokenAuthority = solana.PublicKey.findProgramAddressSync(
      [anchor.utils.bytes.utf8.encode("spl")],
      merkleTreeProgram.programId
    )[0];

  if (mode == "deposit" && lastTx && is_token == true) {
    console.log(`mode ${mode}, token: ${is_token}`)

    let escrowTokenAccount = await solana.PublicKey.createWithSeed(
      signer.publicKey,
      "escrow",
      token.TOKEN_PROGRAM_ID,
    );
    var userAccountInfo = await connection.getAccountInfo(
          pdas.feeEscrowStatePubkey
    )

    const accountAfterUpdate = verifierProgram.account.verifierState._coder.accounts.decode('FeeEscrowState', userAccountInfo.data);
    try {

    const txLastTransaction = await verifierProgram.methods.lastTransactionDeposit(
          ).accounts(
              {
                signingAddress: signer.publicKey,
                verifierState: pdas.verifierStatePubkey,
                systemProgram: SystemProgram.programId,
                programMerkleTree: merkleTreeProgram.programId,
                rent: DEFAULT_PROGRAMS.rent,
                nullifier0Pda: pdas.nullifier0PdaPubkey,
                nullifier1Pda: pdas.nullifier1PdaPubkey,
                twoLeavesPda: pdas.leavesPdaPubkey,
                escrowPda: pdas.escrowPdaPubkey,
                merkleTreePdaToken: recipient,
                userAccount: origin.publicKey,
                merkleTree: merkle_tree_pubkey,
                feeEscrowState: pdas.feeEscrowStatePubkey,
                merkleTreeProgram:  merkleTreeProgram.programId,
                preInsertedLeavesIndex: preInsertedLeavesIndex,
                authority: authority
              }
            ).remainingAccounts([
              { isSigner: false, isWritable: true, pubkey:escrowTokenAccount },       //
            ]).preInstructions([
              SystemProgram.transfer({
                fromPubkey: signer.publicKey,
                toPubkey: authority,
                lamports: (await connection.getMinimumBalanceForRentExemption(8)) * 2 + 3173760, //(await connection.getMinimumBalanceForRentExemption(256)),
              })
            ]).signers([signer]).rpc()
      } catch(e){
        console.log(e)
      }
      console.log("checkLastTxSuccess")
      await checkLastTxSuccess({
        connection,
        pdas,
        sender: pdas.feeEscrowStatePubkey,
        senderAccountBalancePriorLastTx,
        relayer: signer.publicKey,
        recipient: recipient,
        recipientBalancePriorLastTx,
        ix_data,
        mode,
        merkleTreeProgram,
        pre_inserted_leaves_index: preInsertedLeavesIndex,
        relayerFee,
        is_token: true,
        escrowTokenAccount
      })
  } else if (mode == "deposit" && lastTx) {
    console.log(mode)
    var userAccountInfo = await connection.getAccountInfo(
          pdas.feeEscrowStatePubkey
        )
    const accountAfterUpdate = verifierProgram.account.verifierState._coder.accounts.decode('FeeEscrowState', userAccountInfo.data);
    try {
      const txLastTransaction = await verifierProgram.methods.lastTransactionDeposit(
            ).accounts(
                {
                  signingAddress: signer.publicKey,
                  verifierState: pdas.verifierStatePubkey,
                  // merkleTreeUpdateState:pdas.merkleTreeUpdateState,
                  systemProgram: SystemProgram.programId,
                  programMerkleTree: merkleTreeProgram.programId,
                  rent: DEFAULT_PROGRAMS.rent,
                  nullifier0Pda: pdas.nullifier0PdaPubkey,
                  nullifier1Pda: pdas.nullifier1PdaPubkey,
                  twoLeavesPda: pdas.leavesPdaPubkey,
                  escrowPda: pdas.escrowPdaPubkey,
                  merkleTreePdaToken: recipient,
                  userAccount: origin.publicKey,
                  merkleTree: merkle_tree_pubkey,
                  feeEscrowState: pdas.feeEscrowStatePubkey,
                  merkleTreeProgram:  merkleTreeProgram.programId,
                  preInsertedLeavesIndex: preInsertedLeavesIndex,
                  authority: authority
                }
              ).preInstructions([
                SystemProgram.transfer({
                  fromPubkey: signer.publicKey,
                  toPubkey: authority,
                  lamports: (await connection.getMinimumBalanceForRentExemption(8)) * 2 + 3173760, //(await connection.getMinimumBalanceForRentExemption(256)),
                })
              ]).signers([signer]).rpc()


    } catch(e) {
      console.log(e)
    }

      await checkLastTxSuccess({
        connection,
        pdas,
        sender:origin.publicKey,
        senderAccountBalancePriorLastTx,
        relayer: signer.publicKey,
        recipient: recipient,
        recipientBalancePriorLastTx,
        ix_data,
        mode,
        merkleTreeProgram,
        pre_inserted_leaves_index: preInsertedLeavesIndex,
        relayerFee
      })
  } else if (mode== "withdrawal" && lastTx && is_token == true) {

    let relayerAccountBalancePriorLastTx = (await token.getAccount(
      connection,
      relayer_recipient,
      token.TOKEN_PROGRAM_ID
    )).amount;

    console.log(mode, is_token)

    try {
    const txLastTransaction = await verifierProgram.methods.lastTransactionWithdrawal(
      ).accounts(
          {
            signingAddress: signer.publicKey,
            nullifier0Pda: pdas.nullifier0PdaPubkey,
            nullifier1Pda: pdas.nullifier1PdaPubkey,
            twoLeavesPda: pdas.leavesPdaPubkey,
            verifierState: pdas.verifierStatePubkey,
            programMerkleTree: merkleTreeProgram.programId,
            systemProgram: SystemProgram.programId,
            rent: DEFAULT_PROGRAMS.rent,
            merkleTreePdaToken: origin_token,
            merkleTree: merkle_tree_pubkey,
            recipient:  recipient,
            relayerRecipient: relayer_recipient,
            preInsertedLeavesIndex: preInsertedLeavesIndex,
            authority: authority,
            tokenAuthority,
            tokenProgram: token.TOKEN_PROGRAM_ID
          }
        ).preInstructions([
          SystemProgram.transfer({
            fromPubkey: signer.publicKey,
            toPubkey: authority,
            lamports: (await connection.getMinimumBalanceForRentExemption(8)) * 2 + 3173760,//(await connection.getMinimumBalanceForRentExemption(256)),
          })
        ]).signers([signer]).rpc()
        console.log("success")
      } catch(e) {
        console.log(e)
      }
      await checkLastTxSuccess({
        connection,
        pdas,
        sender:origin_token,
        relayer:relayer_recipient,
        relayerAccountBalancePriorLastTx,
        senderAccountBalancePriorLastTx,
        recipient: recipient,
        recipientBalancePriorLastTx,
        ix_data,
        mode,
        merkleTreeProgram,
        pre_inserted_leaves_index: preInsertedLeavesIndex,
        relayerFee,
        is_token: true
      })

    console.log("token withdrawal success")

  } else if (mode== "withdrawal" && lastTx) {
      console.log(mode)
      let relayerAccountBalancePriorLastTx = (await connection.getAccountInfo(signer.publicKey)).lamports
      const txLastTransaction = await verifierProgram.methods.lastTransactionWithdrawal(
        ).accounts(
            {
              signingAddress: signer.publicKey,
              verifierState: pdas.verifierStatePubkey,
              systemProgram: SystemProgram.programId,
              programMerkleTree: merkleTreeProgram.programId,
              rent: DEFAULT_PROGRAMS.rent,
              nullifier0Pda: pdas.nullifier0PdaPubkey,
              nullifier1Pda: pdas.nullifier1PdaPubkey,
              twoLeavesPda: pdas.leavesPdaPubkey,
              merkleTreePdaToken: origin,
              merkleTree: merkle_tree_pubkey,
              recipient:  recipient,
              relayerRecipient: relayer_recipient.publicKey,
              preInsertedLeavesIndex: preInsertedLeavesIndex,
              authority: authority,
              tokenAuthority,
              tokenProgram: token.TOKEN_PROGRAM_ID
            }
          ).preInstructions([
            SystemProgram.transfer({
              fromPubkey: signer.publicKey,
              toPubkey: authority,
              lamports: (await connection.getMinimumBalanceForRentExemption(8)) * 2 + 3173760,//(await connection.getMinimumBalanceForRentExemption(256)),
            })
          ]).signers([signer]).transaction()
        await provider.sendAndConfirm(txLastTransaction, [signer])

      await checkLastTxSuccess({
        connection,
        pdas,
        sender:origin,
        relayer:signer.publicKey,
        senderAccountBalancePriorLastTx,
        recipient: recipient,
        recipientBalancePriorLastTx,
        ix_data,
        mode,
        merkleTreeProgram,
        pre_inserted_leaves_index: preInsertedLeavesIndex,
        relayerFee,
        relayerAccountBalancePriorLastTx
      })
      console.log("withdrawal success")

    } else {
    console.log("lastTx ", lastTx);
  }

  return pdas.leavesPdaPubkey;
}

export async function executeXComputeTransactions({
  number_of_transactions,
  signer,
  pdas,
  program,
  provider
}) {
  let arr = []
  // console.log(`sending ${number_of_transactions} transactions`)
  // console.log(`verifierState ${pdas.verifierStatePubkey}`)
  // console.log(`merkleTreeUpdateState ${pdas.merkleTreeUpdateState}`)

  for (var i = 0; i < number_of_transactions; i++) {

    let bump = new anchor.BN(i)
    const tx1 = await program.methods.compute(
            bump
          ).accounts(
              {
                signingAddress: signer.publicKey,
                verifierState: pdas.verifierStatePubkey,
              }
            ).signers([signer])
          .transaction();
      tx1.feePayer = signer.publicKey;
      arr.push({tx:tx1, signers: [signer]})

    }
    await Promise.all(arr.map(async (tx, index) => {
    await provider.sendAndConfirm(tx.tx, tx.signers);
  }));
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

var merkleTreeAccountPrior = await connection.getAccountInfo(
  merkle_tree_pubkey
)
let merkleTreeUpdateState = solana.PublicKey.findProgramAddressSync(
    [Buffer.from(new Uint8Array(signer.publicKey.toBytes())), anchor.utils.bytes.utf8.encode("storage")],
    merkleTreeProgram.programId)[0];


const tx1 = await merkleTreeProgram.methods.initializeMerkleTreeUpdateState(
    new anchor.BN(merkleTreeIndex) // merkle tree index
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
      ).signers([signer]).rpc()

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
      ).signers([signer]).rpc()
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
    merkle_tree_pubkey: merkle_tree_pubkey
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

  await Promise.all(arr.map(async (tx, index) => {

    await provider.sendAndConfirm(tx.tx, tx.signers);

  }));
}
export async function createVerifierState({
  provider,
  ix_data,
  relayer,
  pdas,
  merkleTree,
  merkleTreeProgram,
  verifierProgram
}) {
  try  {
    const tx = await verifierProgram.methods.createVerifierState(
      ix_data.proofAbc,
      ix_data.rootHash,
      ix_data.amount,
      ix_data.txIntegrityHash,
      ix_data.nullifier0,
      ix_data.nullifier1,
      ix_data.leafRight,
      ix_data.leafLeft,
      ix_data.recipient,
      ix_data.extAmount,
      ix_data.relayer,
      ix_data.fee,
      ix_data.encryptedUtxos,
      ix_data.merkleTreeIndex
    ).accounts(
      {
        signingAddress: relayer.publicKey,
        verifierState: pdas.verifierStatePubkey,
        systemProgram: SystemProgram.programId,
        merkleTree,
        programMerkleTree:  merkleTreeProgram.programId,
      }
    ).signers([relayer]).transaction()
    await provider.sendAndConfirm(tx, [relayer])
  } catch(e) {
    console.log(e)
    process.exit()
  }

  checkVerifierStateAccountCreated({
    connection: provider.connection,
    pda: pdas.verifierStatePubkey,
    ix_data,
    relayer_pubkey:relayer.publicKey
  })

}
export async function newAccountWithTokens ({
  connection,
  MINT,
  ADMIN_AUTH_KEYPAIR,
  userAccount,
  amount
}) {

  var tokenAccount = await token.getOrCreateAssociatedTokenAccount(
     connection,
     userAccount,
     MINT,
     userAccount.publicKey
 );

 await token.mintTo(
   connection,
   ADMIN_AUTH_KEYPAIR,
   MINT,
   tokenAccount.address,
   ADMIN_AUTH_KEYPAIR.publicKey,
   amount,
   []
 );

 return tokenAccount.address;
}
