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
  getPdaAddresses,
  unpackLeavesAccount,
} from "./utils/unpack_accounts"
import {
  deposit,
  transact,
  executeXComputeTransactions,
  executeUpdateMerkleTreeTransactions,
  newAccountWithLamports,
  newProgramOwnedAccount,
  newAddressWithLamports
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
  relayerFee,
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
      it("Initialize Merkle Tree", async () => {
        await newAccountWithLamports(
          provider.connection,
          ADMIN_AUTH_KEYPAIR
        )
        await provider.connection.requestAirdrop(ADMIN_AUTH_KEY, 1_000_000_000_000)
        var ADMIN_AUTH_KEYPAIRAccountInfo = await provider.connection.getAccountInfo(
              ADMIN_AUTH_KEYPAIR.publicKey
            )

        try {
          const tx = await merkleTreeProgram.methods.initializeNewMerkleTree().accounts({
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

      });

/*it("Initialize Nullifier Test", async () => {
    await newAccountWithLamports(
      provider.connection,
      ADMIN_AUTH_KEYPAIR
    )
    console.log("ADMIN_AUTH_KEYPAIR: ", ADMIN_AUTH_KEYPAIR)
    await provider.connection.requestAirdrop(ADMIN_AUTH_KEY, 1_000_000_000_000)
    var ADMIN_AUTH_KEYPAIRAccountInfo = await provider.connection.getAccountInfo(
          ADMIN_AUTH_KEYPAIR.publicKey
        )
        console.log("ADMIN_AUTH_KEYPAIRAccountInfo: ", ADMIN_AUTH_KEYPAIRAccountInfo)

    console.log("MERKLE_TREE_KP: ", verifierProgram.methods);
    let nullifier0 = new Uint8Array(32).fill(2);
    let nullifier0PdaPubkey = solana.PublicKey.findProgramAddressSync(
        [Buffer.from(nullifier0), anchor.utils.bytes.utf8.encode("nf")],
        merkleTreeProgram.programId)[0];
    let authority = solana.PublicKey.findProgramAddressSync(
        [merkleTreeProgram.programId.toBuffer()],
        verifierProgram.programId)[0];
        console.log("merkleTreeProgram.programId.toBuffer() ", Array.prototype.slice.call(merkleTreeProgram.programId.toBuffer()))
        // , anchor.utils.bytes.utf8.encode("authority")
        console.log("authority: ", Array.prototype.slice.call(authority.toBytes()))
        console.log("authority: ", authority.toBase58())


    // try calling from other program
    try {
      const tx = await attackerProgram.methods.testNullifierInsert(nullifier0).accounts({
        authority: authority,
        signingAddress: ADMIN_AUTH_KEY,
        nullifier0Pda: nullifier0PdaPubkey,
        programMerkleTree:  merkleTreeProgram.programId,
        merkleTree: MERKLE_TREE_KEY,
        preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
        // pub nullifier0_pda: UncheckedAccount<'info>,
        //   pub program_merkle_tree: Program<'info, MerkleTreeProgram>,
        //   /// CHECK:` should be a pda
        //   #[account(mut)]
        //   pub authority: AccountInfo<'info>,
        //   #[account(mut)]
        //   pub signing_address: Signer<'info>,
        //   pub system_program: Program<'info, System>,
        //   pub rent: Sysvar<'info, Rent>
      }).preInstructions([
        SystemProgram.transfer({
          fromPubkey: ADMIN_AUTH_KEY,
          toPubkey: authority,
          lamports: await provider.connection.getMinimumBalanceForRentExemption(8),
        })
      ])
      .signers([ADMIN_AUTH_KEYPAIR])
      .rpc();

    } catch(e) {
      console.log("e: ", e)
    }

    try {
      const tx = await attackerProgram.methods.testCheckMerkleRootExists().accounts({
        authority: authority,
        signingAddress: ADMIN_AUTH_KEY,
        nullifier0Pda: nullifier0PdaPubkey,
        programMerkleTree:  merkleTreeProgram.programId,
        merkleTree: MERKLE_TREE_KEY,
        preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
        // pub nullifier0_pda: UncheckedAccount<'info>,
        //   pub program_merkle_tree: Program<'info, MerkleTreeProgram>,
        //   /// CHECK:` should be a pda
        //   #[account(mut)]
        //   pub authority: AccountInfo<'info>,
        //   #[account(mut)]
        //   pub signing_address: Signer<'info>,
        //   pub system_program: Program<'info, System>,
        //   pub rent: Sysvar<'info, Rent>
      })
      // .preInstructions([
      //   SystemProgram.transfer({
      //     fromPubkey: ADMIN_AUTH_KEY,
      //     toPubkey: authority,
      //     lamports: await provider.connection.getMinimumBalanceForRentExemption(8),
      //   })
      // ])
      .signers([ADMIN_AUTH_KEYPAIR])
      .rpc();

    } catch(e) {
      console.log("e: ", e)
      console.log("testCheckMerkleRootExists should fail")
    }


    try {
      const tx = await attackerProgram.methods.testInsertTwoLeaves().accounts({
        authority: authority,
        signingAddress: ADMIN_AUTH_KEY,
        nullifier0Pda: nullifier0PdaPubkey,
        programMerkleTree:  merkleTreeProgram.programId,
        merkleTree: MERKLE_TREE_KEY,
        preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
        ...DEFAULT_PROGRAMS
        // pub nullifier0_pda: UncheckedAccount<'info>,
        //   pub program_merkle_tree: Program<'info, MerkleTreeProgram>,
        //   /// CHECK:` should be a pda
        //   #[account(mut)]
        //   pub authority: AccountInfo<'info>,
        //   #[account(mut)]
        //   pub signing_address: Signer<'info>,
        //   pub system_program: Program<'info, System>,
        //   pub rent: Sysvar<'info, Rent>
      }).preInstructions([
        SystemProgram.transfer({
          fromPubkey: ADMIN_AUTH_KEY,
          toPubkey: authority,
          lamports: await provider.connection.getMinimumBalanceForRentExemption(8),
        })
      ])
      .signers([ADMIN_AUTH_KEYPAIR])
      .rpc();

    } catch(e) {
      console.log(" testInsertTwoLeaves should  fail")
      console.log("e: ", e)
    }

    try {
      const tx = await attackerProgram.methods.testWithdrawSol().accounts({
        authority: authority,
        signingAddress: ADMIN_AUTH_KEY,
        nullifier0Pda: nullifier0PdaPubkey,
        programMerkleTree:  merkleTreeProgram.programId,
        merkleTree: MERKLE_TREE_KEY,
        preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
        ...DEFAULT_PROGRAMS
        // pub nullifier0_pda: UncheckedAccount<'info>,
        //   pub program_merkle_tree: Program<'info, MerkleTreeProgram>,
        //   /// CHECK:` should be a pda
        //   #[account(mut)]
        //   pub authority: AccountInfo<'info>,
        //   #[account(mut)]
        //   pub signing_address: Signer<'info>,
        //   pub system_program: Program<'info, System>,
        //   pub rent: Sysvar<'info, Rent>
      }).preInstructions([
        SystemProgram.transfer({
          fromPubkey: ADMIN_AUTH_KEY,
          toPubkey: authority,
          lamports: await provider.connection.getMinimumBalanceForRentExemption(8),
        })
      ])
      .signers([ADMIN_AUTH_KEYPAIR])
      .rpc();

    } catch(e) {
      console.log("testWithdrawSol should fail")
      console.log("e: ", e)
    }

    console.log("MERKLE_TREE_KEY: ", MERKLE_TREE_KEY.toBase58())
    var merkleTreeAccountInfo = await provider.connection.getAccountInfo(
          nullifier0PdaPubkey
        )
    console.log("merkleTreeAccountInfo " ,merkleTreeAccountInfo)

    // // assert_eq(constants.INIT_BYTES_MERKLE_TREE_18,
    // //   merkleTreeAccountInfo.data.slice(0,constants.INIT_BYTES_MERKLE_TREE_18.length)
    // // )
    // if (merkleTreeAccountInfo.data.length !== MERKLE_TREE_SIZE) {
    //   throw "merkle tree pda size wrong after initializing";
    //
    // }
    // if (merkleTreeAccountInfo.owner.toBase58() !== merkleTreeProgram.programId.toBase58()) {
    //   throw "merkle tree pda owner wrong after initializing";
    // }
    // var merkleTreeIndexAccountInfo = await provider.connection.getAccountInfo(
    //       PRE_INSERTED_LEAVES_INDEX
    //     )
    // console.log("merkleTreeIndexAccountInfo. ", merkleTreeIndexAccountInfo)
    // if (merkleTreeIndexAccountInfo === null) {
    //   throw "merkleTreeIndexAccountInfo not initialized";
    // }
    console.log("process exit")
    process.exit()
  });


  it("Register Verifier Program", async () => {
    console.log("Register Verifier Program here");
    const tx = await merkleTreeProgram.methods.registerNewId().accounts({
      authority: ADMIN_AUTH_KEY,
      registry: REGISTERED_VERIFIER_KEY,
      newId: verifierProgram.programId,
      ...DEFAULT_PROGRAMS
    })
    .signers([ADMIN_AUTH_KEYPAIR])
    .rpc();
    console.log("register new id tx: ", tx)
    const registry = await merkleTreeProgram.account.registry.fetch(REGISTERED_VERIFIER_KEY);
    console.log("registry: ", registry)
    assert(registry.id.equals(verifierProgram.programId) , 'Verifier Program Id mismatch');

  });

  it("Failed to Create AuthorityConfig for not upgradable authority", async () => {
    const programInfo = await provider.connection.getAccountInfo(merkleTreeProgram.programId);
    const programDataAddress = PROGRAM_LAYOUT.decode(programInfo.data).programDataAddress;
    const authKeypair = solana.Keypair.generate();
    await expect(
      merkleTreeProgram.methods.createAuthorityConfig().accounts({
        authority: authKeypair.publicKey,
        merkleTreeProgram: merkleTreeProgram.programId,
        authorityConfig: AUTHORITY_CONFIG_KEY,
        merkleTreeProgramData: programDataAddress,
        ...DEFAULT_PROGRAMS
      })
      .signers([authKeypair])
      .rpc()
    ).to.be.rejectedWith("0", "A raw constraint was violated");
  });
  it("Create AuthorityConfig", async () => {
    const programInfo = await provider.connection.getAccountInfo(merkleTreeProgram.programId);
    console.log(programInfo.data);
    const programDataAddress = PROGRAM_LAYOUT.decode(programInfo.data).programDataAddress;

    const tx = await merkleTreeProgram.methods.createAuthorityConfig().accounts({
      authority: (merkleTreeProgram.provider as any).wallet.pubkey,
      merkleTreeProgram: merkleTreeProgram.programId,
      authorityConfig: AUTHORITY_CONFIG_KEY,
      merkleTreeProgramData: programDataAddress,
      ...DEFAULT_PROGRAMS
    })
    .rpc();
  });
  it("Failed to update AuthorityConfig for not current authority", async () => {
    const authKeypair = solana.Keypair.generate();
    await expect(
      merkleTreeProgram.methods.updateAuthorityConfig(ADMIN_AUTH_KEY).accounts({
        authority: authKeypair.publicKey,
        authorityConfig: AUTHORITY_CONFIG_KEY,
        ...DEFAULT_PROGRAMS
      })
      .signers([authKeypair])
      .rpc()
    ).to.be.rejectedWith("0", "A raw constraint was violated");
});
  it("Update Authority Config", async () => {
    const tx = await merkleTreeProgram.methods.updateAuthorityConfig(ADMIN_AUTH_KEY).accounts({
      authority: (merkleTreeProgram.provider as any).wallet.pubkey,
      authorityConfig: AUTHORITY_CONFIG_KEY,
      ...DEFAULT_PROGRAMS
    })
    .rpc();
  });*/
  /*
  it("Groth16 verification hardcoded inputs should succeed", async () => {
    let userAccount =new anchor.web3.Account()
    await newAccountWithLamports(provider.connection, userAccount ) // new anchor.web3.Account()
    let init_account = await newAccountWithLamports(provider.connection ) // new anchor.web3.Account()
    let merkleTreePdaToken = await newProgramOwnedAccount({
      connection: provider.connection,
      owner: merkleTreeProgram
    })
    let {ix_data, bytes} = read_and_parse_instruction_data_bytes();
    let merkleTree = await light.buildMerkelTree(provider.connection);

    let pdas = getPdaAddresses({
      tx_integrity_hash: ix_data.txIntegrityHash,
      nullifier0: ix_data.nullifier0,
      nullifier1: ix_data.nullifier1,
      leafLeft: ix_data.leafLeft,
      merkleTreeProgram,
      verifierProgram
    })
    console.log("pdas ", pdas)
    await newAddressWithLamports(provider.connection, pdas.verifierStatePubkey) // new anchor.web3.Account()
    let res  = await transact({
      connection: provider.connection,
      ix_data,
      pdas,
      origin: userAccount,
      signer: init_account,
      recipient: merkleTreePdaToken,
      verifierProgram,
      mode: "deposit"
    })
    let leavesPdas = [{ isSigner: false, isWritable: true, pubkey: res }]
    console.log(leavesPdas)

    await executeUpdateMerkleTreeTransactions({
      connection: provider.connection,
      signer:userAccount,
      program: merkleTreeProgram,
      leavesPdas,
      merkleTree
    });
  });*/

  it("Open and close escrow", async () => {
    const origin = await newAccountWithLamports(provider.connection) // new anchor.web3.Account()
    const relayer = await newAccountWithLamports(provider.connection) // new anchor.web3.Account()
    let {ix_data, bytes} = read_and_parse_instruction_data_bytes();
    let tx_fee = 5000 * 50;
    let escrow_amount = U64.readLE(ix_data.extAmount, 0).toNumber() + tx_fee + U64.readLE(ix_data.fee, 0).toNumber()
    let pdas = getPdaAddresses({
      tx_integrity_hash: ix_data.txIntegrityHash,
      nullifier0: ix_data.nullifier0,
      nullifier1: ix_data.nullifier1,
      leafLeft: ix_data.leafLeft,
      merkleTreeProgram,
      verifierProgram
    })
    var relayerInfoStart = await connection.getAccountInfo(
      relayer.publicKey
    )
    var userInfoStart = await connection.getAccountInfo(
      origin.publicKey
    )
    try{
      const tx = await verifierProgram.methods.createEscrow(
            ix_data.txIntegrityHash,
            new anchor.BN(tx_fee), // does not need to be checked since this tx is signed by the user
            ix_data.fee,
            new anchor.BN(I64.readLE(ix_data.extAmount,0).toString())
      ).accounts(
          {
            signingAddress: relayer.publicKey,
            verifierState: pdas.verifierStatePubkey,
            systemProgram: SystemProgram.programId,
            feeEscrowState: pdas.feeEscrowStatePubkey,
            user:           origin.publicKey,
          }
        ).signers([relayer, origin]).rpc();

    } catch (e) {
      console.log("e", e)
    }



      await checkEscrowAccountCreated({
        connection: provider.connection,
        pdas,
        ix_data,
        user_pubkey: origin.publicKey,
        relayer_pubkey: relayer.publicKey,
        tx_fee: new anchor.BN(tx_fee),//check doesn t work
        verifierProgram
      });

      var relayerInfoMid = await connection.getAccountInfo(
        relayer.publicKey
      )
      var userInfoMid = await connection.getAccountInfo(
        origin.publicKey
      )
      var feeEscrowStatePubkeyInfoMid = await connection.getAccountInfo(
        pdas.feeEscrowStatePubkey
      )
      try {
        const tx1 = await verifierProgram.methods.closeEscrow(
        ).accounts(
          {
            signingAddress: relayer.publicKey,
            verifierState: pdas.verifierStatePubkey,
            systemProgram: SystemProgram.programId,
            feeEscrowState: pdas.feeEscrowStatePubkey,
            user:           origin.publicKey,
            relayer:        relayer.publicKey,
          }
        ).signers([relayer]).rpc();
    } catch (e) {
      console.log("e", e)
    }
      var feeEscrowStatePubkeyInfo = await connection.getAccountInfo(
        pdas.feeEscrowStatePubkey
      )
      var relayerInfoEnd = await connection.getAccountInfo(
        relayer.publicKey
      )
      var userInfoEnd = await connection.getAccountInfo(
        origin.publicKey
      )
      assert(feeEscrowStatePubkeyInfo == null, "Escrow account is not closed");
      console.log("feeEscrowStatePubkeyInfo")
      console.log("relayerInfo", relayerInfoEnd)
      console.log("userInfo", userInfoEnd)
      console.log(`relayerInfoStart ${relayerInfoStart.lamports} ${relayerInfoMid.lamports} ${Number(relayerInfoEnd.lamports)}`)
      console.log(`relayerInfoStart ${relayerInfoStart.lamports} ${relayerInfoMid.lamports - relayerInfoStart.lamports} ${Number(relayerInfoEnd.lamports) - relayerInfoStart.lamports}`)
      assert(relayerInfoStart.lamports == relayerInfoEnd.lamports)
      console.log(`userInfoStart  ${userInfoStart.lamports} ${userInfoMid.lamports} ${userInfoEnd.lamports}`)
      console.log("ix_data.extAmount: ", U64.readLE(ix_data.extAmount, 0).toString())
      console.log(`userInfoStart  ${userInfoStart.lamports} ${userInfoMid.lamports + U64.readLE(ix_data.extAmount, 0).toNumber()} ${Number(userInfoEnd.lamports) - userInfoStart.lamports}`)
      assert(userInfoStart.lamports == userInfoEnd.lamports)
      console.log("feeEscrowStatePubkeyInfoMid: ", feeEscrowStatePubkeyInfoMid.lamports)
      let rent = await provider.connection.getMinimumBalanceForRentExemption(128);
      console.log("rent: ", rent)
      console.log("escrow_amount: ", escrow_amount)
      console.log(`feeEscrowStatePubkeyInfoMid.lamports : ${feeEscrowStatePubkeyInfoMid.lamports} ${escrow_amount + rent} `)
      assert(feeEscrowStatePubkeyInfoMid.lamports == escrow_amount + rent)
      assert(userInfoStart.lamports == userInfoMid.lamports + escrow_amount)


  })
  /*
  it("Dynamic Shielded transaction", async () => {
      const userAccount = await newAccountWithLamports(provider.connection) // new anchor.web3.Account()
      const recipientWithdrawal = await newAccountWithLamports(provider.connection) // new anchor.web3.Account()

      var leavesPdas = []
      var utxos = []

      //
      // *
      // * test deposit
      // *
      //

      let merkleTree = await light.buildMerkelTree(provider.connection);

      let Keypair = new light.Keypair()

      for (var i= 0; i < 2; i++) {
        let res = await deposit({
          Keypair,
          encryptionKeypair,
          amount: 1_000_000_00,// amount
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
          relayerFee
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
      const merkleTreeWithdrawal = await light.buildMerkelTree(provider.connection);
      let deposit_utxo1 = utxos[0][0];
      let deposit_utxo2 = utxos[0][1];
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
        externalAmountBigNumber,
        relayerFee,
        recipientWithdrawal.publicKey.toBase58(),
        relayer.publicKey.toBase58(),
        'WITHDRAWAL',
        encryptionKeypair
      )

      let ix_dataWithdrawal = parse_instruction_data_bytes(dataWithdrawal);
      let pdasWithdrawal = getPdaAddresses({
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

    })
  */
});
