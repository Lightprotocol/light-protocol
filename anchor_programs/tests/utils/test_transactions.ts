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

  await Promise.all(arr.map(async (tx, index) => {

  try {
      await provider.sendAndConfirm(tx.tx, tx.signers,{
        commitment: 'finalized',
        preflightCommitment: 'finalized',
      });
  } catch(e) {
      console.log(e);

  }

  }));
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
