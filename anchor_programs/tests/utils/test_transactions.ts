const solana = require("@solana/web3.js");
const {U64, I64} = require('n64');
import { assert, expect } from "chai";
import { BigNumber, providers } from 'ethers'
const light = require('../../light-protocol-sdk');
import * as anchor from "@project-serum/anchor";
const { SystemProgram } = require('@solana/web3.js');

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
  amount, // 1_000_000_00
  connection,
  merkleTree,
  merkleTreePdaToken,
  userAccount,
  verifierProgram,
  merkleTreeProgram,
  authority,
  preInsertedLeavesIndex,
  merkle_tree_pubkey,
  provider,
  relayerFee,
  lastTx
}) {
  const burnerUserAccount = await newAccountWithLamports(connection)

  let deposit_utxo1 = new light.Utxo(BigNumber.from(amount), Keypair)
  let deposit_utxo2 = new light.Utxo(BigNumber.from(amount), Keypair)

  let inputUtxos = [new light.Utxo(), new light.Utxo()]
  let outputUtxos = [deposit_utxo1, deposit_utxo2 ]

  const data = await light.getProof(
    inputUtxos,
    outputUtxos,
    merkleTree,
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

  let leavesPda = await transact({
    connection: connection,
    ix_data,
    pdas,
    origin: userAccount,
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
    lastTx
  })

  return [leavesPda, outputUtxos, burnerUserAccount, pdas];
}

export async function transact({
  connection,
  ix_data,
  pdas,
  origin,
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
  lastTx
}) {
  if (lastTx == undefined) {
    lastTx = true
  }
  // tx fee in lamports
  let tx_fee = 5000 * PREPARED_INPUTS_TX_COUNT + MILLER_LOOP_TX_COUNT + FINAL_EXPONENTIATION_TX_COUNT + 2* MERKLE_TREE_UPDATE_TX_COUNT;
  var userAccountPriorLastTx;
  let senderAccountBalancePriorLastTx
  if (mode === "deposit") {
    userAccountPriorLastTx = await connection.getAccountInfo(
          origin.publicKey
        )
    senderAccountBalancePriorLastTx = userAccountPriorLastTx.lamports;

  } else if (mode === "withdrawal") {
    userAccountPriorLastTx = await connection.getAccountInfo(
          origin
        )
    senderAccountBalancePriorLastTx = userAccountPriorLastTx.lamports;

  }


  var recipientAccountPriorLastTx = await connection.getAccountInfo(
        recipient
      )

  let recipientBalancePriorLastTx = recipientAccountPriorLastTx != null ? recipientAccountPriorLastTx.lamports : 0;

  if (mode === "deposit") {
    console.log("creating escrow")
    // create escrow account
    const tx = await verifierProgram.methods.createEscrow(
          ix_data.txIntegrityHash,
          new anchor.BN(tx_fee), // does not need to be checked since this tx is signed by the user
          ix_data.fee,
          new anchor.BN(I64.readLE(ix_data.extAmount,0).toString())
        ).accounts(
              {
                signingAddress: signer.publicKey,
                verifierState: pdas.verifierStatePubkey,
                systemProgram: SystemProgram.programId,
                feeEscrowState: pdas.feeEscrowStatePubkey,
                user:           origin.publicKey,
              }
            ).signers([signer, origin]).rpc();

      await checkEscrowAccountCreated({
        connection:connection,
        pdas,
        ix_data,
        user_pubkey: origin.publicKey,
        relayer_pubkey: signer.publicKey,
        tx_fee: new anchor.BN(tx_fee),//check doesn t work
        verifierProgram
      });
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


  if (mode == "deposit" && lastTx) {
    console.log(mode)
    var userAccountInfo = await connection.getAccountInfo(
          pdas.feeEscrowStatePubkey
        )
    const accountAfterUpdate = verifierProgram.account.verifierState._coder.accounts.decode('FeeEscrowState', userAccountInfo.data);

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
  } else if (mode== "withdrawal" && lastTx) {
    console.log(mode)

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
            authority: authority
          }
        ).preInstructions([
          SystemProgram.transfer({
            fromPubkey: signer.publicKey,
            toPubkey: authority,
            lamports: (await connection.getMinimumBalanceForRentExemption(8)) * 2 + 3173760,//(await connection.getMinimumBalanceForRentExemption(256)),
          })
        ]).signers([signer]).rpc()


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
      relayerFee
    })
    console.log("withdrawal success")

  } else {
    console.log("mode not supplied");
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
    new anchor.BN(0) // merkle tree index
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
    // sending 10 additional tx to finish the merkle tree update
  }
  /*
  for (var retry = 0; retry < 10; retry++) {
    try {
      console.log("final tx to insert root")
        await merkleTreeProgram.methods.insertRootMerkleTree(
          new anchor.BN(254))
        .accounts({
          authority: signer.publicKey,
          merkleTreeUpdateState: merkleTreeUpdateState,
          merkleTree: merkle_tree_pubkey
        }).remainingAccounts(
          leavesPdas
        ).signers([signer]).rpc()
        break;
    } catch (e) {
      console.log(e)
      // sending 10 additional tx to finish the merkle tree update
    }
    let arr_retry = []
    for(let ix_id = 0; ix_id < 10; ix_id ++) {

      const transaction = new solana.Transaction();
      transaction.add(
        await merkleTreeProgram.methods.updateMerkleTree(new anchor.BN(i))
        .accounts({
          authority: signer.publicKey,
          // verifierStateAuthority:pdas.verifierStatePubkey,
          merkleTreeUpdateState: merkleTreeUpdateState,
          merkleTree: merkle_tree_pubkey
        }).instruction()
      )
      i+=1;
      transaction.add(
        await merkleTreeProgram.methods.updateMerkleTree(new anchor.BN(i)).accounts({
          authority: signer.publicKey,
          // verifierStateAuthority:pdas.verifierStatePubkey,
          merkleTreeUpdateState: merkleTreeUpdateState,
          merkleTree: merkle_tree_pubkey
        }).instruction()
      )
      i+=1;

      arr_retry.push({tx:transaction, signers: [signer]})
    }
    console.log(`created ${arr.length} Merkle tree update tx`);


    await Promise.all(arr_retry.map(async (tx, index) => {
      try {
        await provider.sendAndConfirm(tx.tx, tx.signers);
      } catch (e) {
        console.log("e: ", e)
      }
    }));
  }
  */

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
