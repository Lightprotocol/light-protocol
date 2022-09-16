import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { VerifierProgram } from "../target/types/verifier_program";
import { AttackerProgram } from "../target/types/attacker_program";
const { SystemProgram } = require('@solana/web3.js');

import {
  checkEscrowAccountCreated,
  checkVerifierStateAccountCreated,
  checkFinalExponentiationSuccess,
  checkLastTxSuccess,
  checkMerkleTreeUpdateStateCreated,
  checkMerkleTreeBatchUpdateSuccess,
  checkRentExemption,
  assert_eq
} from "./utils/test_checks";

import {
  read_and_parse_instruction_data_bytes,
  parse_instruction_data_bytes,
  readAndParseAccountDataMerkleTreeTmpState,
  await getPdaAddresses,
  unpackLeavesAccount,
} from "./utils/unpack_accounts"
import {
  deposit,
  transact,
  executeXComputeTransactions,
  executeUpdateMerkleTreeTransactions,
  newAccountWithLamports,
  newProgramOwnedAccount,
  newAddressWithLamports,
  executeMerkleTreeUpdateTransactions
} from "./utils/test_transactions";

import { findProgramAddressSync } from "@project-serum/anchor/dist/cjs/utils/pubkey";
import fs from 'fs';
const solana = require("@solana/web3.js");
const {U64, I64} = require('n64');
import nacl from "tweetnacl";
import { BigNumber, providers } from 'ethers'
// const { poseidonHash } = require('./utils/poseidonHash')
const {
  amount,
  encryptionKeypair,
  externalAmountBigNumber,
  publicKey,
  inputUtxoAmount,
  outputUtxoAmount,
  // relayerFee,
  testInputUtxo,
  testOutputUtxo
} = require ('./utils/testUtxos');
import _ from "lodash";

import { use as chaiUse } from "chai";

import { assert, expect } from "chai";

import { BigNumber } from "ethers";
/** BigNumber to hex string of specified length */
const toFixedHex = function (number: any, length: number = 32) {
    let result =
      "0x" +
      (number instanceof Buffer
        ? number.toString("hex")
        : BigNumber.from(number).toHexString().replace("0x", "")
      ).padStart(length * 2, "0");
    if (result.indexOf("-") > -1) {
      result = "-" + result.replace("-", "");
    }
    return result;
  }


const MERKLE_TREE_SIGNER_AUTHORITY = new solana.PublicKey([59, 42, 227, 2, 155, 13, 249, 77, 6, 97, 72, 159, 190, 119, 46, 110, 226, 42, 153, 232, 210, 107, 116, 255, 63, 213, 216, 18, 94, 128, 155, 225])

// const Utxo = require("./utils/utxo");
// const prepareTransaction = require("./utils/prepareTransaction");
// const MerkleTree = require("./utils/merkleTree");

// const light = require('@darjusch/light-protocol-sdk');
const light = require('../light-protocol-sdk');



const sleep = (ms) => {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

import { MerkleTreeProgram, IDL } from "../target/types/merkle_tree_program";

import { publicKey, u64, u128, } from '@project-serum/borsh';
import { struct, u8, u16, u32, blob } from 'buffer-layout';

const constants:any = {};

const TYPE_PUBKEY = { array: [ 'u8', 32 ] };
const TYPE_SEED = {defined: "&[u8]"};
const TYPE_INIT_DATA = { array: [ 'u8', 642 ] };
// IDL.constants.map((item) => {
//   if(_.isEqual(item.type, TYPE_SEED)) {
//     constants[item.name] = item.value.replace("b\"", "").replace("\"", "");
//   } else //if(_.isEqual(item.type, TYPE_PUBKEY) || _.isEqual(item.type, TYPE_INIT_DATA))
//   {
//     constants[item.name] = JSON.parse(item.value)
//   }
// });






const PRIVATE_KEY = [
  17, 34, 231, 31, 83, 147, 93, 173, 61, 164, 25, 0, 204, 82, 234, 91, 202, 187, 228, 110, 146,
  97, 112, 131, 180, 164, 96, 220, 57, 207, 65, 107, 2, 99, 226, 251, 88, 66, 92, 33, 25, 216,
  211, 185, 112, 203, 212, 238, 105, 144, 72, 121, 176, 253, 106, 168, 115, 158, 154, 188, 62,
  255, 166, 81,
];
import { ASSOCIATED_TOKEN_PROGRAM_ID, TOKEN_PROGRAM_ID } from '@solana/spl-token';
const MERKLE_TREE_INIT_AUTHORITY = [2, 99, 226, 251, 88, 66, 92, 33, 25, 216, 211, 185, 112, 203, 212, 238, 105, 144, 72, 121, 176,
  253, 106, 168, 115, 158, 154, 188, 62, 255, 166, 81,
];
const ADMIN_AUTH_KEY = new solana.PublicKey(new Uint8Array(MERKLE_TREE_INIT_AUTHORITY));
const ADMIN_AUTH_KEYPAIR = solana.Keypair.fromSecretKey(new Uint8Array(PRIVATE_KEY));
const MERKLE_TREE_ACC_BYTES_0 = new Uint8Array([
    242, 149, 147, 41, 62, 228, 214, 222, 231, 159, 167, 195, 10, 226, 182, 153, 84, 80, 249, 150,
    131, 112, 150, 225, 133, 131, 32, 149, 69, 188, 94, 13,
]);
const MERKLE_TREE_KP = solana.Keypair.fromSeed(MERKLE_TREE_ACC_BYTES_0);

const MERKLE_TREE_KEY = MERKLE_TREE_KP.publicKey;

const MERKLE_TREE_SIZE = 16658;

const MERKLE_TREE_TOKEN_ACC_BYTES_0 = new Uint8Array([
    123, 30, 128, 110, 93, 171, 2, 242, 20, 194, 175, 25, 246, 98, 182, 99, 31, 110, 119, 163, 68,
    179, 244, 89, 176, 19, 93, 136, 149, 231, 179, 213,
]);

export const AUTHORITY_SEED = anchor.utils.bytes.utf8.encode("AUTHORITY_SEED")
export const DEFAULT_PROGRAMS = {
  systemProgram: solana.SystemProgram.programId,
  tokenProgram: TOKEN_PROGRAM_ID,
  associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
  rent: solana.SYSVAR_RENT_PUBKEY,
  clock: solana.SYSVAR_CLOCK_PUBKEY,
};

// const PROGRAM_LAYOUT = struct([
//   u32('isInitialized'),
//   publicKey('programDataAddress'),
// ]);

var IX_DATA;
var SIGNER;
var UNREGISTERED_MERKLE_TREE;
var UNREGISTERED_MERKLE_TREE_PDA_TOKEN;
var UNREGISTERED_PRE_INSERTED_LEAVES_INDEX;

describe("verifier_program", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const provider = anchor.AnchorProvider.local();//anchor.getProvider();
  const connection = provider.connection;
  const verifierProgram = anchor.workspace.VerifierProgram as Program<VerifierProgram>;
  const merkleTreeProgram = anchor.workspace.MerkleTreeProgram as Program<MerkleTreeProgram>;
  const attackerProgram = anchor.workspace.AttackerProgram as Program<AttackerProgram>;
  const [REGISTERED_VERIFIER_KEY] = solana.PublicKey.findProgramAddressSync(
      [verifierProgram.programId.toBuffer()],
      merkleTreeProgram.programId
    );
  // const [AUTHORITY_CONFIG_KEY] = solana.PublicKey.findProgramAddressSync([Buffer.from(AUTHORITY_SEED)], merkleTreeProgram.programId);
  const PRE_INSERTED_LEAVES_INDEX = solana.PublicKey.findProgramAddressSync(
      [MERKLE_TREE_KEY.toBuffer()],
      merkleTreeProgram.programId
    )[0];
  const MERKLE_TREE_PDA_TOKEN = solana.PublicKey.findProgramAddressSync(
      [MERKLE_TREE_KEY.toBuffer(), anchor.utils.bytes.utf8.encode("MERKLE_TREE_PDA_TOKEN")],
      merkleTreeProgram.programId
    )[0];

  const AUTHORITY = solana.PublicKey.findProgramAddressSync(
      [merkleTreeProgram.programId.toBuffer()],
      verifierProgram.programId)[0];

  var RENT_ESCROW

it("Initialize Merkle Tree", async () => {
  await newAccountWithLamports(
    provider.connection,
    ADMIN_AUTH_KEYPAIR
  )
  RENT_ESCROW = await provider.connection.getMinimumBalanceForRentExemption(256);

  await provider.connection.requestAirdrop(ADMIN_AUTH_KEY, 1_000_000_000_000)
  var ADMIN_AUTH_KEYPAIRAccountInfo = await provider.connection.getAccountInfo(
        ADMIN_AUTH_KEYPAIR.publicKey
      )

  try {
    const tx = await merkleTreeProgram.methods.initializeNewMerkleTreeSol().accounts({
      authority: ADMIN_AUTH_KEY,
      merkleTree: MERKLE_TREE_KEY,
      preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
      merkleTreePdaToken: MERKLE_TREE_PDA_TOKEN,
      ...DEFAULT_PROGRAMS
    })
    .preInstructions([
      SystemProgram.createAccount({
        fromPubkey: ADMIN_AUTH_KEY,
        newAccountPubkey: MERKLE_TREE_KEY,
        space: MERKLE_TREE_SIZE,
        lamports: await provider.connection.getMinimumBalanceForRentExemption(MERKLE_TREE_SIZE),
        programId: merkleTreeProgram.programId,
      })
    ])
    .signers([ADMIN_AUTH_KEYPAIR, MERKLE_TREE_KP])
    .rpc();

  } catch(e) {
    console.log("e: ", e)
  }
  var merkleTreeAccountInfo = await provider.connection.getAccountInfo(
        MERKLE_TREE_KEY
      )
  // assert_eq(constants.INIT_BYTES_MERKLE_TREE_18,
  //   merkleTreeAccountInfo.data.slice(0,constants.INIT_BYTES_MERKLE_TREE_18.length)
  // )
  if (merkleTreeAccountInfo.data.length !== MERKLE_TREE_SIZE) {
    throw "merkle tree pda size wrong after initializing";

  }
  if (merkleTreeAccountInfo.owner.toBase58() !== merkleTreeProgram.programId.toBase58()) {
    throw "merkle tree pda owner wrong after initializing";
  }
  var merkleTreeIndexAccountInfo = await provider.connection.getAccountInfo(
        PRE_INSERTED_LEAVES_INDEX
      )
  assert(merkleTreeIndexAccountInfo != null, "merkleTreeIndexAccountInfo not initialized")
  UNREGISTERED_MERKLE_TREE = new anchor.web3.Account()
  UNREGISTERED_MERKLE_TREE_PDA_TOKEN = solana.PublicKey.findProgramAddressSync(
      [UNREGISTERED_MERKLE_TREE.publicKey.toBuffer(), anchor.utils.bytes.utf8.encode("MERKLE_TREE_PDA_TOKEN")],
      merkleTreeProgram.programId
    )[0];

  UNREGISTERED_PRE_INSERTED_LEAVES_INDEX = solana.PublicKey.findProgramAddressSync(
      [UNREGISTERED_MERKLE_TREE.publicKey.toBuffer()],
      merkleTreeProgram.programId
    )[0];
  try {
    const tx = await merkleTreeProgram.methods.initializeNewMerkleTreeSol().accounts({
      authority: ADMIN_AUTH_KEY,
      merkleTree: UNREGISTERED_MERKLE_TREE.publicKey,
      preInsertedLeavesIndex: UNREGISTERED_PRE_INSERTED_LEAVES_INDEX,
      merkleTreePdaToken: UNREGISTERED_MERKLE_TREE_PDA_TOKEN,
      ...DEFAULT_PROGRAMS
    })
    .preInstructions([
      SystemProgram.createAccount({
        fromPubkey: ADMIN_AUTH_KEY,
        newAccountPubkey: UNREGISTERED_MERKLE_TREE.publicKey,
        space: MERKLE_TREE_SIZE,
        lamports: await provider.connection.getMinimumBalanceForRentExemption(MERKLE_TREE_SIZE),
        programId: merkleTreeProgram.programId,
      })
    ])
    .signers([ADMIN_AUTH_KEYPAIR, UNREGISTERED_MERKLE_TREE])
    .rpc();
  } catch(e) {
    console.log(e)
  }
});


it("Dynamic Shielded transaction", async () => {
    const userAccount = await newAccountWithLamports(provider.connection) // new anchor.web3.Account()
    while(true) {
      const recipientWithdrawal = new anchor.web3.Account();//await newAccountWithLamports(provider.connection) // new anchor.web3.Account()

      var leavesPdas = []
      var utxos = []

      //
      // *
      // * test deposit
      // *
      //

      let merkleTree = await light.buildMerkelTree(provider.connection, MERKLE_TREE_KEY.toBytes());
      // below 900k gives errors of leaving an account non rentexempt
      let relayerFee = U64(Math.floor(Math.random() * 80_000) + 900_000);
      console.log("relayerFee: ", relayerFee.toString())
      let Keypair = new light.Keypair()
      let nr_leaves = Math.floor(Math.random() * 16);
      console.log("nr_leaves: ", nr_leaves)
      for (var i= 0; i < nr_leaves; i++) {
        console.log("i: ", i, ": ", nr_leaves)
        let res = await deposit({
          Keypair,
          encryptionKeypair,
          amount: Math.floor(Math.random() * 1_000_000_000) + relayerFee.toNumber(),// amount
          connection: provider.connection,
          merkleTree,
          merkleTreePdaToken: MERKLE_TREE_PDA_TOKEN,
          userAccount,
          verifierProgram,
          merkleTreeProgram,
          authority: AUTHORITY,
          preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
          merkle_tree_pubkey: MERKLE_TREE_KEY,
          provider,
          relayerFee,
          lastTx: true,
          rent: RENT_ESCROW
        })
        leavesPdas.push({ isSigner: false, isWritable: true, pubkey: res[0]})
        utxos.push(res[1])
      }

      await executeUpdateMerkleTreeTransactions({
        connection: provider.connection,
        signer:userAccount,
        merkleTreeProgram: merkleTreeProgram,
        leavesPdas,
        merkleTree,
        merkle_tree_pubkey: MERKLE_TREE_KEY,
        provider
      });


      // *
      // * test withdrawal
      // *
      // *
      // *

      // new lightTransaction
      // generate utxos
      //
      var leavesPdasWithdrawal = []
      const merkleTreeWithdrawal = await light.buildMerkelTree(provider.connection, MERKLE_TREE_KEY.toBytes());
      let spendUtxoIndex = Math.floor(Math.random() * nr_leaves)
      let deposit_utxo1 = utxos[spendUtxoIndex][0];
      let deposit_utxo2 = utxos[spendUtxoIndex][1];
      deposit_utxo1.index = merkleTreeWithdrawal._layers[0].indexOf(deposit_utxo1.getCommitment()._hex)
      deposit_utxo2.index = merkleTreeWithdrawal._layers[0].indexOf(deposit_utxo2.getCommitment()._hex)

      let relayer = await newAccountWithLamports(provider.connection);
      let relayer_recipient = new anchor.web3.Account();

      let inputUtxosWithdrawal = []
      if (deposit_utxo1.index == 1) {
        inputUtxosWithdrawal = [deposit_utxo1, new light.Utxo()] // 38241198
      } else {
        inputUtxosWithdrawal = [deposit_utxo2, new light.Utxo()] // 38241198
      }
      let outputUtxosWithdrawal = [new light.Utxo(), new light.Utxo() ]

      const externalAmountBigNumber: BigNumber = BigNumber.from(relayerFee.toString())
      .add(
        outputUtxosWithdrawal.reduce(
          (sum, utxo) => sum.add(utxo.amount),
          BigNumber.from(0),
        ),
      )
      .sub(
        inputUtxosWithdrawal.reduce((sum, utxo) => sum.add(utxo.amount), BigNumber.from(0)),
      )
      console.log("externalAmountBigNumber ", externalAmountBigNumber.toString())
      var dataWithdrawal = await light.getProof(
        inputUtxosWithdrawal,
        outputUtxosWithdrawal,
        merkleTreeWithdrawal,
        0,
        MERKLE_TREE_KEY.toBytes(),
        externalAmountBigNumber,
        relayerFee,
        recipientWithdrawal.publicKey.toBase58(),
        relayer.publicKey.toBase58(),
        'WITHDRAWAL',
        encryptionKeypair
      )

      let ix_dataWithdrawal = parse_instruction_data_bytes(dataWithdrawal);
      let pdasWithdrawal = await getPdaAddresses({
        tx_integrity_hash: ix_dataWithdrawal.txIntegrityHash,
        nullifier0: ix_dataWithdrawal.nullifier0,
        nullifier1: ix_dataWithdrawal.nullifier1,
        leafLeft: ix_dataWithdrawal.leafLeft,
        merkleTreeProgram,
        verifierProgram
      })
      let balanceWithdrawal = await provider.connection.getAccountInfo(MERKLE_TREE_PDA_TOKEN)
      console.log("balanceWithdrawal.lamports ", balanceWithdrawal.lamports)
      let recipientWithdrawal1 = await provider.connection.getAccountInfo(recipientWithdrawal.publicKey)
      console.log("recipientWithdrawal1.lamports ", recipientWithdrawal1)
      let relayer_recipientWithdrawal1 = await provider.connection.getAccountInfo(relayer_recipient.publicKey)
      console.log("recipientWithdrawal1.lamports ", relayer_recipientWithdrawal1)
      let resWithdrawalTransact
      try {
        resWithdrawalTransact = await transact({
          connection: provider.connection,
          ix_data: ix_dataWithdrawal,
          pdas: pdasWithdrawal,
          origin: MERKLE_TREE_PDA_TOKEN,
          signer: relayer,
          recipient: recipientWithdrawal.publicKey,
          relayer_recipient,
          mode: "withdrawal",
          verifierProgram,
          merkleTreeProgram,
          authority: AUTHORITY,
          preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
          merkle_tree_pubkey: MERKLE_TREE_KEY,
          provider,
          relayerFee
        })
      } catch(e) {
        console.log("withdrawal: ", e)
      }

      leavesPdasWithdrawal.push({
        isSigner: false,
        isWritable: true,
        pubkey: resWithdrawalTransact
      })
      await executeUpdateMerkleTreeTransactions({
        connection: provider.connection,
        signer:relayer,
        merkleTreeProgram,
        leavesPdas: leavesPdasWithdrawal,
        merkleTree: merkleTreeWithdrawal,
        merkle_tree_pubkey: MERKLE_TREE_KEY,
        provider
      });
    }
  })


});
