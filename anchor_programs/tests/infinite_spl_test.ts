import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { VerifierProgram } from "../target/types/verifier_program";
import { AttackerProgram } from "../target/types/attacker_program";
const { SystemProgram } = require('@solana/web3.js');
import { MerkleTreeProgram } from "../target/types/merkle_tree_program";
import { findProgramAddressSync } from "@project-serum/anchor/dist/cjs/utils/pubkey";
const solana = require("@solana/web3.js");
const {U64, I64} = require('n64');
import nacl from "tweetnacl";
import { BigNumber, providers } from 'ethers'
const light = require('../light-protocol-sdk');
import _ from "lodash";
import { assert, expect } from "chai";
const token = require('@solana/spl-token')

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
  newAccountWithTokens,
  executeMerkleTreeUpdateTransactions,
  createVerifierState
} from "./utils/test_transactions";

const {
  amount,
  encryptionKeypair,
  externalAmountBigNumber,
  publicKey,
  inputUtxoAmount,
  outputUtxoAmount,
  relayerFee,
  testInputUtxo,
  testOutputUtxo
} = require ('./utils/testUtxos');

import {
  MERKLE_TREE_KEY,
  DEFAULT_PROGRAMS,
  ADMIN_AUTH_KEYPAIR,
  ADMIN_AUTH_KEY,
  MERKLE_TREE_SIZE,
  MERKLE_TREE_KP,
  MERKLE_TREE_SIGNER_AUTHORITY,
  PRIVATE_KEY
  } from "./utils/constants";


var IX_DATA;
var SIGNER;
var UNREGISTERED_MERKLE_TREE;
var UNREGISTERED_MERKLE_TREE_PDA_TOKEN;
var UNREGISTERED_PRE_INSERTED_LEAVES_INDEX;
var UTXOS;
var MERKLE_TREE_OLD;

var MERKLE_TREE_USDC
var MERKLE_TREE_PDA_TOKEN_USDC
var PRE_INSERTED_LEAVES_INDEX_USDC
var MINT
var RENT_ESCROW
var RENT_VERIFIER
var RENT_TOKEN_ACCOUNT
describe("random token test", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const provider = anchor.AnchorProvider.local();//anchor.getProvider();
  const connection = provider.connection;
  const verifierProgram = anchor.workspace.VerifierProgram as Program<VerifierProgram>;
  const merkleTreeProgram = anchor.workspace.MerkleTreeProgram as Program<MerkleTreeProgram>;

  const [REGISTERED_VERIFIER_KEY] = solana.PublicKey.findProgramAddressSync(
      [verifierProgram.programId.toBuffer()],
      merkleTreeProgram.programId
    );
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




  // it("Initialize Merkle Tree", async () => {
  //   await newAccountWithLamports(
  //     provider.connection,
  //     ADMIN_AUTH_KEYPAIR
  //   )
  //   await provider.connection.requestAirdrop(ADMIN_AUTH_KEY, 1_000_000_000_000)
  //   var ADMIN_AUTH_KEYPAIRAccountInfo = await provider.connection.getAccountInfo(
  //         ADMIN_AUTH_KEYPAIR.publicKey
  //       )
  //
  //   try {
  //     const tx = await merkleTreeProgram.methods.initializeNewMerkleTreeSol().accounts({
  //       authority: ADMIN_AUTH_KEY,
  //       merkleTree: MERKLE_TREE_KEY,
  //       preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
  //       merkleTreePdaToken: MERKLE_TREE_PDA_TOKEN,
  //       ...DEFAULT_PROGRAMS
  //     })
  //     .preInstructions([
  //       SystemProgram.createAccount({
  //         fromPubkey: ADMIN_AUTH_KEY,
  //         newAccountPubkey: MERKLE_TREE_KEY,
  //         space: MERKLE_TREE_SIZE,
  //         lamports: await provider.connection.getMinimumBalanceForRentExemption(MERKLE_TREE_SIZE),
  //         programId: merkleTreeProgram.programId,
  //       })
  //     ])
  //     .signers([ADMIN_AUTH_KEYPAIR, MERKLE_TREE_KP])
  //     .rpc();
  //
  //   } catch(e) {
  //     console.log("e: ", e)
  //   }
  //   var merkleTreeAccountInfo = await provider.connection.getAccountInfo(
  //         MERKLE_TREE_KEY
  //       )
  //   // assert_eq(constants.INIT_BYTES_MERKLE_TREE_18,
  //   //   merkleTreeAccountInfo.data.slice(0,constants.INIT_BYTES_MERKLE_TREE_18.length)
  //   // )
  //   if (merkleTreeAccountInfo.data.length !== MERKLE_TREE_SIZE) {
  //     throw "merkle tree pda size wrong after initializing";
  //
  //   }
  //   if (merkleTreeAccountInfo.owner.toBase58() !== merkleTreeProgram.programId.toBase58()) {
  //     throw "merkle tree pda owner wrong after initializing";
  //   }
  //   var merkleTreeIndexAccountInfo = await provider.connection.getAccountInfo(
  //         PRE_INSERTED_LEAVES_INDEX
  //       )
  //   assert(merkleTreeIndexAccountInfo != null, "merkleTreeIndexAccountInfo not initialized")
  //   UNREGISTERED_MERKLE_TREE = new anchor.web3.Account()
  //   UNREGISTERED_MERKLE_TREE_PDA_TOKEN = solana.PublicKey.findProgramAddressSync(
  //       [UNREGISTERED_MERKLE_TREE.publicKey.toBuffer(), anchor.utils.bytes.utf8.encode("MERKLE_TREE_PDA_TOKEN")],
  //       merkleTreeProgram.programId
  //     )[0];
  //
  //   UNREGISTERED_PRE_INSERTED_LEAVES_INDEX = solana.PublicKey.findProgramAddressSync(
  //       [UNREGISTERED_MERKLE_TREE.publicKey.toBuffer()],
  //       merkleTreeProgram.programId
  //     )[0];
  //   try {
  //     const tx = await merkleTreeProgram.methods.initializeNewMerkleTreeSol().accounts({
  //       authority: ADMIN_AUTH_KEY,
  //       merkleTree: UNREGISTERED_MERKLE_TREE.publicKey,
  //       preInsertedLeavesIndex: UNREGISTERED_PRE_INSERTED_LEAVES_INDEX,
  //       merkleTreePdaToken: UNREGISTERED_MERKLE_TREE_PDA_TOKEN,
  //       ...DEFAULT_PROGRAMS
  //     })
  //     .preInstructions([
  //       SystemProgram.createAccount({
  //         fromPubkey: ADMIN_AUTH_KEY,
  //         newAccountPubkey: UNREGISTERED_MERKLE_TREE.publicKey,
  //         space: MERKLE_TREE_SIZE,
  //         lamports: await provider.connection.getMinimumBalanceForRentExemption(MERKLE_TREE_SIZE),
  //         programId: merkleTreeProgram.programId,
  //       })
  //     ])
  //     .signers([ADMIN_AUTH_KEYPAIR, UNREGISTERED_MERKLE_TREE])
  //     .rpc();
  //   } catch(e) {
  //     console.log(e)
  //   }
  // });

  it("Initialize Token Merkle tree", async () => {
    await newAccountWithLamports(
      provider.connection,
      ADMIN_AUTH_KEYPAIR
    )
    await provider.connection.requestAirdrop(ADMIN_AUTH_KEY, 1_000_000_000_000)
    var ADMIN_AUTH_KEYPAIRAccountInfo = await provider.connection.getAccountInfo(
      ADMIN_AUTH_KEYPAIR.publicKey
    )

    MERKLE_TREE_USDC= await solana.PublicKey.createWithSeed(
        ADMIN_AUTH_KEY,
        "usdc",
        merkleTreeProgram.programId,
      )
    MERKLE_TREE_PDA_TOKEN_USDC  = solana.PublicKey.findProgramAddressSync(
          [MERKLE_TREE_USDC.toBytes(), anchor.utils.bytes.utf8.encode("merkle_tree_pda_token")],
          merkleTreeProgram.programId
        )[0];
    PRE_INSERTED_LEAVES_INDEX_USDC = solana.PublicKey.findProgramAddressSync(
        [MERKLE_TREE_USDC.toBuffer()],
        merkleTreeProgram.programId
      )[0];
    RENT_ESCROW = await provider.connection.getMinimumBalanceForRentExemption(256);
    RENT_VERIFIER = await provider.connection.getMinimumBalanceForRentExemption(5 * 1024);
    RENT_TOKEN_ACCOUNT = await provider.connection.getMinimumBalanceForRentExemption(token.ACCOUNT_SIZE)

    console.log("MERKLE_TREE_USDC: ", MERKLE_TREE_USDC.toBase58())

    console.log("MERKLE_TREE_USDC: ", Array.prototype.slice.call(MERKLE_TREE_USDC.toBytes()))
    console.log("MERKLE_TREE_PDA_TOKEN_USDC: ", MERKLE_TREE_PDA_TOKEN_USDC.toBase58())
    console.log("MERKLE_TREE_PDA_TOKEN_USDC: ", Array.prototype.slice.call(MERKLE_TREE_PDA_TOKEN_USDC.toBytes()))

    const signer = await newAccountWithLamports(provider.connection)

    await provider.connection.requestAirdrop(signer.publicKey, 1_000_000_000_000)
    let tokenAuthority = solana.PublicKey.findProgramAddressSync(
        [anchor.utils.bytes.utf8.encode("spl")],
        merkleTreeProgram.programId
      )[0];
    // create new token
    try {
    console.log()
    MINT = await token.createMint(
        provider.connection,
        ADMIN_AUTH_KEYPAIR,
        ADMIN_AUTH_KEYPAIR.publicKey,
        null,
        2
    );
  } catch(e) {
    console.log(e)
  }

    try {
      const tx = await merkleTreeProgram.methods.initializeNewMerkleTreeSpl(
      ).accounts({
        authority: ADMIN_AUTH_KEYPAIR.publicKey,
        merkleTree: MERKLE_TREE_USDC,
        preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX_USDC,
        merkleTreePdaToken: MERKLE_TREE_PDA_TOKEN_USDC,
        tokenProgram:token.TOKEN_PROGRAM_ID,
        systemProgram: DEFAULT_PROGRAMS.systemProgram,
        mint: MINT,
        tokenAuthority: tokenAuthority,
        rent: DEFAULT_PROGRAMS.rent
      })
      .preInstructions([
        SystemProgram.createAccountWithSeed({
          basePubkey:ADMIN_AUTH_KEY,
          seed: anchor.utils.bytes.utf8.encode("usdc"),
          fromPubkey: ADMIN_AUTH_KEY,
          newAccountPubkey: MERKLE_TREE_USDC,
          space: MERKLE_TREE_SIZE,
          lamports: await provider.connection.getMinimumBalanceForRentExemption(MERKLE_TREE_SIZE),
          programId: merkleTreeProgram.programId,
        })
      ])
      .signers([ADMIN_AUTH_KEYPAIR])
      .rpc();

    } catch(e) {
      console.log("e: ", e)
    }
    var merkleTreeAccountInfo = await provider.connection.getAccountInfo(
          MERKLE_TREE_USDC
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

  });



  it("Deposit and withdraw token", async () => {
    const userAccount = await newAccountWithLamports(provider.connection)
    let relayer = await newAccountWithLamports(provider.connection);
    let relayer_recipient = await newAccountWithTokens({
        connection: provider.connection,
        MINT,
        ADMIN_AUTH_KEYPAIR,
        userAccount: relayer,
        amount: 0
    });

    while(true) {
      let relayerFee = U64(Math.floor(Math.random() * 80_000) + 10_000);
      let nr_leaves = 1//Math.floor(Math.random() * 16);


      const userAccountToken = await newAccountWithTokens({
        connection: provider.connection,
        MINT,
        ADMIN_AUTH_KEYPAIR,
        userAccount,
        amount: 1_000_000_000_000_00
      })
      let escrowTokenAccountInfo1 = await token.getAccount(
        provider.connection,
        userAccountToken,
        token.TOKEN_PROGRAM_ID
      );
      var signer
      var pdas
      var leavesPdas = []
      var utxos = []

      //
      // *
      // * test deposit
      // *
      //

      let merkleTree = await light.buildMerkelTree(provider.connection, MERKLE_TREE_USDC.toBytes());

      let Keypair = new light.Keypair()
      for (var i = 0; i < nr_leaves; i++) {
        console.log(`${i}/ ${nr_leaves}`)
        var amount = Math.floor(Math.random() * 1_000_000_000) + 100_000

        let res = await deposit({
          Keypair,
          relayer,
          encryptionKeypair,
          MINT,
          amount: amount,
          connection: provider.connection,
          merkleTree,
          merkleTreePdaToken: MERKLE_TREE_PDA_TOKEN_USDC,
          userAccount,
          userAccountToken,
          verifierProgram,
          merkleTreeProgram,
          authority: AUTHORITY,
          preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX_USDC,
          merkle_tree_pubkey: MERKLE_TREE_USDC,
          provider,
          relayerFee,
          is_token: true,
          rent: RENT_ESCROW
        })
        leavesPdas.push({ isSigner: false, isWritable: true, pubkey: res[0]})
        utxos.push(res[1])
        signer = res[2]
        pdas = res[3]
      }

      await executeUpdateMerkleTreeTransactions({
        connection: provider.connection,
        signer: userAccount,
        merkleTreeProgram: merkleTreeProgram,
        leavesPdas,
        merkleTree,
        merkleTreeIndex: 1,
        merkle_tree_pubkey: MERKLE_TREE_USDC,
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
      const merkleTreeWithdrawal = await light.buildMerkelTree(provider.connection, MERKLE_TREE_USDC.toBytes());
      let deposit_utxo1 = utxos[0][0];
      let deposit_utxo2 = utxos[0][1];
      deposit_utxo1.index = merkleTreeWithdrawal._layers[0].indexOf(deposit_utxo1.getCommitment()._hex)
      deposit_utxo2.index = merkleTreeWithdrawal._layers[0].indexOf(deposit_utxo2.getCommitment()._hex)


      const recipientWithdrawal = await newAccountWithLamports(provider.connection)

      var recipientTokenAccount = await token.getOrCreateAssociatedTokenAccount(
         connection,
         relayer,
         MINT,
         recipientWithdrawal.publicKey
     );
      let inputUtxosWithdrawal = []
      if (deposit_utxo1.index == 1) {
        inputUtxosWithdrawal = [deposit_utxo1, new light.Utxo()] // 38241198
      } else {
        inputUtxosWithdrawal = [deposit_utxo2, new light.Utxo()] // 38241198
      }
      let outputAmount = 300_000 + Math.floor((inputUtxosWithdrawal[0].amount - 300_000) - (inputUtxosWithdrawal[0].amount  - 300_000) * Math.random())
      let outputUtxo = new light.Utxo(outputAmount)
      let outputUtxosWithdrawal = [outputUtxo, new light.Utxo() ]

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

      var dataWithdrawal = await light.getProof(
        inputUtxosWithdrawal,
        outputUtxosWithdrawal,
        merkleTreeWithdrawal,
        1, //merkleTreeIndex:
        MERKLE_TREE_USDC.toBytes(),
        externalAmountBigNumber,
        relayerFee,
        recipientTokenAccount.address.toBase58(),
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

      let resWithdrawalTransact = await transact({
        connection: provider.connection,
        ix_data: ix_dataWithdrawal,
        pdas: pdasWithdrawal,
        origin_token: MERKLE_TREE_PDA_TOKEN_USDC,
        MINT,
        signer: relayer,
        recipient: recipientTokenAccount.address,
        relayer_recipient,
        mode: "withdrawal",
        verifierProgram,
        merkleTreeProgram,
        authority: AUTHORITY,
        preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX_USDC,
        merkle_tree_pubkey: MERKLE_TREE_USDC,
        provider,
        relayerFee,
        is_token: true
      })
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
        merkle_tree_pubkey: MERKLE_TREE_USDC,
        merkleTreeIndex: 1,
        provider
      });

  }

  })



});
