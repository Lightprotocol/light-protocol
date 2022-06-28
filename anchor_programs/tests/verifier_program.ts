import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { VerifierProgram } from "../target/types/verifier_program";
import { AttackerProgram } from "../target/types/attacker_program";

const { SystemProgram } = require('@solana/web3.js');
import { findProgramAddressSync } from "@project-serum/anchor/dist/cjs/utils/pubkey";
import fs from 'fs';
const solana = require("@solana/web3.js");
const {U64, I64} = require('n64');
import nacl from "tweetnacl";
import { BigNumber, providers } from 'ethers'
const { poseidonHash } = require('./utils/poseidonHash')
const {
  amount,
  encryptionKeypair,
  externalAmountBigNumber,
  publicKey,
  // recipient,
  // relayer,
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

const PREPARED_INPUTS_TX_COUNT = 42
const MILLER_LOOP_TX_COUNT = 42
const FINAL_EXPONENTIATION_TX_COUNT = 19
const MERKLE_TREE_UPDATE_TX_COUNT = 38
const MERKLE_TREE_SIGNER_AUTHORITY = new solana.PublicKey([59, 42, 227, 2, 155, 13, 249, 77, 6, 97, 72, 159, 190, 119, 46, 110, 226, 42, 153, 232, 210, 107, 116, 255, 63, 213, 216, 18, 94, 128, 155, 225])
//
// const Utxo = require("./utils/utxo");
// const prepareTransaction = require("./utils/prepareTransaction");
const MerkleTree = require("./utils/merkleTree");

// const light = require('@darjusch/light-protocol-sdk');
const light = require('../light-protocol-sdk');


const newAddressWithLamports = async (connection,address = new anchor.web3.Account().publicKey, lamports = 1e11) => {

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
const newProgramOwnedAccount = async ({connection, owner, lamports = 0}) => {
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
const newAccountWithLamports = async (connection,account = new anchor.web3.Account(),lamports = 1e13) => {
  await connection.confirmTransaction(await connection.requestAirdrop(account.publicKey, lamports))
  return account;
}
const sleep = (ms) => {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

function assert_eq(
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



const read_and_parse_instruction_data_bytes = ()  => {
  let file = fs.readFileSync('tests/deposit.txt','utf8');
  // let file = await fs.readFile("deposit.txt", function(err, fd) {
  //  if (err) {
  //     return console.error(err);
  //  }
   console.log("File opened successfully!");
   var data = JSON.parse(file.toString());
   var partsOfStr = data.bytes[0].split(',');
   let bytes = []
   partsOfStr.map((byte, index)=> {
     if (index > 8) {
       bytes[index] = Number(byte);

     }
   })
   bytes = bytes.slice(9,)

   let ix_data = {
     rootHash:          bytes.slice(0,32),
     amount:             bytes.slice(32,64),
     txIntegrityHash:  bytes.slice(64,96),
     nullifier0:         bytes.slice(96,128),
     nullifier1:         bytes.slice(128,160),
     leafRight:         bytes.slice(160,192),
     leafLeft:          bytes.slice(192,224),
     proofAbc:        bytes.slice(224,480),
     recipient:          bytes.slice(480,512),
     extAmount:         bytes.slice(512,520),
     relayer:            bytes.slice(520, 552),
     fee:                bytes.slice(552, 560),
     merkleTreePdaPubkey:bytes.slice(560, 592),
     merkleTreeIndex:  bytes.slice(592,593),
     encryptedUtxos:    bytes.slice(593,593+222),
   }
   return {ix_data, bytes};
}

function parse_instruction_data_bytes(data) {
   let ix_data = {
     rootHash:          data.data.publicInputsBytes.slice(0,32),
     amount:             data.data.publicInputsBytes.slice(32,64),
     txIntegrityHash:  data.data.publicInputsBytes.slice(64,96),
     nullifier0:         data.data.publicInputsBytes.slice(96,128),
     nullifier1:         data.data.publicInputsBytes.slice(128,160),
     leafRight:         data.data.publicInputsBytes.slice(160,192),
     leafLeft:          data.data.publicInputsBytes.slice(192,224),
     proofAbc:        data.data.proofBytes,
     recipient:          data.data.extDataBytes.slice(0,32),
     extAmount:         data.data.extAmount,
     relayer:            data.data.extDataBytes.slice(40,72),
     fee:                data.data.extDataBytes.slice(72,80),
     merkleTreePdaPubkey:data.data.extDataBytes.slice(80,112),
     merkleTreeIndex:     data.data.extDataBytes.slice(112,113),
     encryptedUtxos:    data.data.extDataBytes.slice(113,335),
   }
   return ix_data;
}

async function readAndParseAccountDataMerkleTreeTmpState({
  connection,
  pda
}) {
  var userAccountInfo = await connection.getAccountInfo(
        pda
      )

    let object = {
        is_initialized: userAccountInfo.data[0],
        account_type: userAccountInfo.data[1],
        current_instruction_index: U64.readLE(userAccountInfo.data.slice(2,10),0).toString(),
        found_root: userAccountInfo.data[10],                     //0
        merkle_tree_pda_pubkey: Array.prototype.slice.call(userAccountInfo.data.slice(11,43)),       //2
        relayer: Array.prototype.slice.call(userAccountInfo.data.slice(43,75)),     //3
        root_hash: Array.prototype.slice.call(userAccountInfo.data.slice(75,107)),

        state: Array.prototype.slice.call(userAccountInfo.data.slice(107,203)),
        current_round: U64.readLE(userAccountInfo.data.slice(235,243),0).toString(),
        current_round_index: U64.readLE(userAccountInfo.data.slice(243,251),0).toString(),
        current_index: U64.readLE(userAccountInfo.data.slice(251,259),0).toString(),
        current_level: U64.readLE(userAccountInfo.data.slice(259,267),0).toString(),
        current_level_hash: Array.prototype.slice.call(userAccountInfo.data.slice(235,267)),

        node_left: Array.prototype.slice.call(userAccountInfo.data.slice(267,299)),
        node_right: Array.prototype.slice.call(userAccountInfo.data.slice(299,331)),
        leaf_left: Array.prototype.slice.call(userAccountInfo.data.slice(331,363)),
        leaf_right: Array.prototype.slice.call(userAccountInfo.data.slice(363,395)),

    }
    return object;
}

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

const AUTHORITY_SEED = anchor.utils.bytes.utf8.encode("AUTHORITY_SEED")
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
/*
it("Initialize Nullifier Test", async () => {
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
      console.log("e: ", e)
    }

    try {
      const tx = await merkleTreeProgram.methods.initializeNullifier(nullifier0).accounts({
        authority: ADMIN_AUTH_KEY,
        nullifierPda: nullifier0PdaPubkey,
        programMerkleTree:  merkleTreeProgram.programId,
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
      console.log("should fail")
    }


    try {
      const tx = await verifierProgram.methods.testNullifierInsert(nullifier0).accounts({
        authority: authority,
        signingAddress: ADMIN_AUTH_KEY,
        nullifier0Pda: nullifier0PdaPubkey,
        programMerkleTree:  merkleTreeProgram.programId,
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
      console.log("should not fail")
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
  */
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

  /*it("Register Verifier Program", async () => {
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
      leafLeft: ix_data.leafLeft
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
          userAccount
        })
        leavesPdas.push({ isSigner: false, isWritable: true, pubkey: res[0]})
        utxos.push(res[1])
      }

      await executeUpdateMerkleTreeTransactions({
        connection: provider.connection,
        signer:userAccount,
        program: merkleTreeProgram,
        leavesPdas,
        merkleTree
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
        leafLeft: ix_dataWithdrawal.leafLeft
      })
      let resWithdrawalTransact = await transact({
        connection: provider.connection,
        ix_data: ix_dataWithdrawal,
        pdas: pdasWithdrawal,
        origin: MERKLE_TREE_PDA_TOKEN,
        signer: relayer,
        recipient: recipientWithdrawal.publicKey,
        relayer_recipient,
        verifierProgram,
        mode: "withdrawal"
      })
      leavesPdasWithdrawal.push({
        isSigner: false,
        isWritable: true,
        pubkey: resWithdrawalTransact
      })
      await executeUpdateMerkleTreeTransactions({
        connection: provider.connection,
        signer:relayer,
        program: merkleTreeProgram,
        leavesPdas: leavesPdasWithdrawal,
        merkleTree: merkleTreeWithdrawal
      });

})

  async function deposit({
    Keypair,
    encryptionKeypair,
    amount, // 1_000_000_00
    connection,
    merkleTree,
    merkleTreePdaToken,
    userAccount,
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
    console.log("generated proof")
    let ix_data = parse_instruction_data_bytes(data);

    let pdas = getPdaAddresses({
      tx_integrity_hash: ix_data.txIntegrityHash,
      nullifier0: ix_data.nullifier0,
      nullifier1: ix_data.nullifier1,
      leafLeft: ix_data.leafLeft
    })

    let leavesPda = await transact({
      connection: connection,
      ix_data,
      pdas,
      origin: userAccount,
      signer: burnerUserAccount,
      recipient: MERKLE_TREE_PDA_TOKEN,
      verifierProgram,
      batch_insert: true,
      mode: "deposit"
    })

    return [leavesPda, outputUtxos];
  }
  async function transact({
    connection,
    ix_data,
    pdas,
    origin,
    signer,
    recipient,
    verifierProgram,
    relayer_recipient,
    batch_insert,
    mode
  }) {
    // tx fee in lamports
    let tx_fee = 5000 * PREPARED_INPUTS_TX_COUNT + MILLER_LOOP_TX_COUNT + FINAL_EXPONENTIATION_TX_COUNT + 2* MERKLE_TREE_UPDATE_TX_COUNT;

    var userAccountPriorLastTx;
    if (mode === "deposit") {
      userAccountPriorLastTx = await connection.getAccountInfo(
            origin.publicKey
          )
    } else {
      userAccountPriorLastTx = await connection.getAccountInfo(
            origin
          )
    }

    let senderAccountBalancePriorLastTx = userAccountPriorLastTx.lamports;

    var recipientAccountPriorLastTx = await connection.getAccountInfo(
          recipient
        )

    let recipientBalancePriorLastTx = recipientAccountPriorLastTx != null ? recipientAccountPriorLastTx.lamports : 0;

    if (mode === "deposit") {
      console.log("creating escrow")
      // create escrow account
      const tx = await verifierProgram.methods.createEscrowState(
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
          connection:provider.connection,
          pdas,
          ix_data,
          user_pubkey: origin.publicKey,
          relayer_pubkey: signer.publicKey,
          tx_fee: new anchor.BN(tx_fee)//check doesn t work
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
                  merkleTree: MERKLE_TREE_KEY,
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
      program: verifierProgram
    })
    checkFinalExponentiationSuccess({
      pda: pdas.verifierStatePubkey,
      connection: provider.connection,
      ix_data
    })

    console.log("Compute Instructions Executed");


    if (mode == "deposit") {
      console.log(mode)
      var userAccountInfo = await provider.connection.getAccountInfo(
            pdas.feeEscrowStatePubkey
          )
      const accountAfterUpdate = verifierProgram.account.verifierState._coder.accounts.decode('FeeEscrowState', userAccountInfo.data);
      console.log(accountAfterUpdate)
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
                    merkleTreePdaToken: MERKLE_TREE_PDA_TOKEN,
                    userAccount: origin.publicKey,
                    merkleTree: MERKLE_TREE_KEY,
                    feeEscrowState: pdas.feeEscrowStatePubkey,
                    merkleTreeProgram:  merkleTreeProgram.programId,
                    preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
                    authority: AUTHORITY
                  }
                ).preInstructions([
                  SystemProgram.transfer({
                    fromPubkey: signer.publicKey,
                    toPubkey: AUTHORITY,
                    lamports: (await provider.connection.getMinimumBalanceForRentExemption(8)) * 2 + 3173760, //(await provider.connection.getMinimumBalanceForRentExemption(256)),
                  })
                ]).signers([signer]).rpc()

        } catch(e) {
          console.log(e)
          process.exit()
        }
        await checkLastTxSuccess({
          connection,
          pdas,
          sender:origin.publicKey,
          senderAccountBalancePriorLastTx,
          recipient: recipient,
          recipientBalancePriorLastTx,
          ix_data,
          mode
        })
    } else if (mode== "withdrawal") {
      console.log(mode)
      try {
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
                    escrowPda: pdas.escrowPdaPubkey,
                    merkleTreePdaToken: MERKLE_TREE_PDA_TOKEN,
                    merkleTree: MERKLE_TREE_KEY,
                    merkleTreeProgram:  merkleTreeProgram.programId,
                    recipient:  recipient,
                    relayerRecipient: relayer_recipient.publicKey,
                    preInsertedLeavesIndex: PRE_INSERTED_LEAVES_INDEX,
                    authority: AUTHORITY
                  }
                ).preInstructions([
                  SystemProgram.transfer({
                    fromPubkey: signer.publicKey,
                    toPubkey: AUTHORITY,
                    lamports: (await provider.connection.getMinimumBalanceForRentExemption(8)) * 2 + 3173760,//(await provider.connection.getMinimumBalanceForRentExemption(256)),
                  })
                ]).signers([signer]).rpc()

      } catch (e) {
        console.log(e)
      }
      await checkLastTxSuccess({
        connection,
        pdas,
        sender:MERKLE_TREE_PDA_TOKEN,
        senderAccountBalancePriorLastTx,
        recipient: recipient,
        recipientBalancePriorLastTx,
        ix_data,
        mode
      })

    } else {
      throw Error("mode not supplied");
    }

    return pdas.leavesPdaPubkey;
  }


  async function executeXComputeTransactions({number_of_transactions,signer,pdas, program}) {
    let arr = []
    console.log(`sending ${number_of_transactions} transactions`)
    console.log(`verifierState ${pdas.verifierStatePubkey}`)
    console.log(`merkleTreeUpdateState ${pdas.merkleTreeUpdateState}`)

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

  async function executeUpdateMerkleTreeTransactions({
    signer,
    program,
    leavesPdas,
    merkleTree
  }) {

  var merkleTreeAccountPrior = await provider.connection.getAccountInfo(
    MERKLE_TREE_KEY
  )
  let merkleTreeUpdateState = solana.PublicKey.findProgramAddressSync(
      [Buffer.from(new Uint8Array(signer.publicKey.toBytes())), anchor.utils.bytes.utf8.encode("storage")],
      merkleTreeProgram.programId)[0];
  console.log("merkleTreeUpdateState: ", merkleTreeUpdateState.toBase58())

  try {

    const tx1 = await program.methods.initializeMerkleTreeUpdateState(
        new anchor.BN(0) // merkle tree index
        ).accounts(
            {
              authority: signer.publicKey,
              merkleTreeUpdateState: merkleTreeUpdateState,
              systemProgram: SystemProgram.programId,
              rent: DEFAULT_PROGRAMS.rent,
              merkleTree: MERKLE_TREE_KEY
            }
          ).remainingAccounts(
            leavesPdas
          ).signers([signer]).rpc()
  }catch (e) {
    console.log("e: ", e);
    console.log("process.exit()")
    process.exit()
  }
  await checkMerkleTreeUpdateStateCreated({
    connection: provider.connection,
    program: program,
    merkleTreeUpdateState,
    MerkleTree: MERKLE_TREE_KEY,
    relayer: signer.publicKey,
    leavesPdas,
    current_instruction_index: 1
  })

    let arr = []
    // console.log(`sending ${29 + 5 * leavesPdas.length} transactions`)

    let i = 0;

    // the number of tx needs to increase with greater batchsize
    // 29 + 2 * leavesPdas.length is a first approximation
    for(let ix_id = 0; ix_id < 252; ix_id ++) {

      const transaction = new solana.Transaction();
      transaction.add(
        await program.methods.updateMerkleTree(new anchor.BN(i))
        .accounts({
          authority: signer.publicKey,
          merkleTreeUpdateState: merkleTreeUpdateState,
          merkleTree: MERKLE_TREE_KEY
        }).instruction()
      )
      i+=1;
      transaction.add(
        await program.methods.updateMerkleTree(new anchor.BN(i)).accounts({
          authority: signer.publicKey,
          merkleTreeUpdateState: merkleTreeUpdateState,
          merkleTree: MERKLE_TREE_KEY
        }).instruction()
      )
      i+=1;

      arr.push({tx:transaction, signers: [signer]})
    }
    console.log(`created ${arr.length} Merkle tree update tx`);


      await Promise.all(arr.map(async (tx, index) => {
        try {
          await provider.sendAndConfirm(tx.tx, tx.signers);
        } catch (e) {
          console.log("e: ", e)
        }
      }));

    await checkMerkleTreeUpdateStateCreated({
      connection: provider.connection,
      program: program,
      merkleTreeUpdateState,
      MerkleTree: MERKLE_TREE_KEY,
      relayer: signer.publicKey,
      leavesPdas,
      current_instruction_index: 56
    })
    // final tx to insert root
    let success = false;
    try {
      console.log("final tx to insert root")
        await program.methods.insertRootMerkleTree(
          new anchor.BN(254))
        .accounts({
          authority: signer.publicKey,
          merkleTreeUpdateState: merkleTreeUpdateState,
          merkleTree: MERKLE_TREE_KEY
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
          await program.methods.insertRootMerkleTree(
            new anchor.BN(254))
          .accounts({
            authority: signer.publicKey,
            merkleTreeUpdateState: merkleTreeUpdateState,
            merkleTree: MERKLE_TREE_KEY
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
          await program.methods.updateMerkleTree(new anchor.BN(i))
          .accounts({
            authority: signer.publicKey,
            // verifierStateAuthority:pdas.verifierStatePubkey,
            merkleTreeUpdateState: merkleTreeUpdateState,
            merkleTree: MERKLE_TREE_KEY
          }).instruction()
        )
        i+=1;
        transaction.add(
          await program.methods.updateMerkleTree(new anchor.BN(i)).accounts({
            authority: signer.publicKey,
            // verifierStateAuthority:pdas.verifierStatePubkey,
            merkleTreeUpdateState: merkleTreeUpdateState,
            merkleTree: MERKLE_TREE_KEY
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
      connection: provider.connection,
      merkleTreeUpdateState: merkleTreeUpdateState,
      merkleTreeAccountPrior,
      numberOfLeaves: leavesPdas.length * 2,
      leavesPdas,
      merkleTree: merkleTree
    })
  }

  function getPdaAddresses({tx_integrity_hash, nullifier0, nullifier1, leafLeft}) {
    return {
      signerAuthorityPubkey: solana.PublicKey.findProgramAddressSync(
          [merkleTreeProgram.programId.toBytes()],
          verifierProgram.programId)[0],
      verifierStatePubkey: solana.PublicKey.findProgramAddressSync(
          [Buffer.from(new Uint8Array(tx_integrity_hash)), anchor.utils.bytes.utf8.encode("storage")],
          verifierProgram.programId)[0],
      feeEscrowStatePubkey: solana.PublicKey.findProgramAddressSync(
          [Buffer.from(new Uint8Array(tx_integrity_hash)), anchor.utils.bytes.utf8.encode("fee_escrow")],
          verifierProgram.programId)[0],
      merkleTreeUpdateState: solana.PublicKey.findProgramAddressSync(
          [Buffer.from(new Uint8Array(leafLeft)), anchor.utils.bytes.utf8.encode("storage")],
          merkleTreeProgram.programId)[0],
      leavesPdaPubkey: solana.PublicKey.findProgramAddressSync(
          [Buffer.from(new Uint8Array(nullifier0)), anchor.utils.bytes.utf8.encode("leaves")],
          merkleTreeProgram.programId)[0],
      nullifier0PdaPubkey: solana.PublicKey.findProgramAddressSync(
          [Buffer.from(new Uint8Array(nullifier0)), anchor.utils.bytes.utf8.encode("nf")],
          merkleTreeProgram.programId)[0],
      nullifier1PdaPubkey: solana.PublicKey.findProgramAddressSync(
          [Buffer.from(new Uint8Array(nullifier1)), anchor.utils.bytes.utf8.encode("nf")],
          merkleTreeProgram.programId)[0],
      escrowPdaPubkey: solana.PublicKey.findProgramAddressSync(
          [Buffer.from(solana.PublicKey.findProgramAddressSync(
              [Buffer.from(new Uint8Array(tx_integrity_hash)), anchor.utils.bytes.utf8.encode("storage")],
              verifierProgram.programId)[0].toBytes()), anchor.utils.bytes.utf8.encode("escrow")],
          verifierProgram.programId)[0],
    }
  }

  async function checkEscrowAccountCreated({connection, pdas, user_pubkey,relayer_pubkey, ix_data, tx_fee}) {
    var userAccountInfo = await provider.connection.getAccountInfo(
      pdas.feeEscrowStatePubkey
    )
    const accountAfterUpdate = verifierProgram.account.verifierState._coder.accounts.decode('FeeEscrowState', userAccountInfo.data);
    assert(userAccountInfo.lamports, U64.readLE(ix_data.extAmount, 0).toString(), "incorrect amount transferred");
    assert(accountAfterUpdate.txFee.toString() == tx_fee.toString(), "tx_fee insert wrong");
    assert(accountAfterUpdate.relayerFee.toString() == U64.readLE(ix_data.fee, 0).toString(), "relayer_fee insert wrong");
    assert(accountAfterUpdate.relayerPubkey.toBase58() == relayer_pubkey.toBase58(), "relayer_pubkey insert wrong");
    assert(accountAfterUpdate.verifierStatePubkey.toBase58() == pdas.verifierStatePubkey.toBase58(), "verifierStatePubkey insert wrong");
    assert(accountAfterUpdate.userPubkey.toBase58() == user_pubkey.toBase58(), "user_pubkey insert wrong");
    assert(accountAfterUpdate.userPubkey.toBase58() == user_pubkey.toBase58(), "user_pubkey insert wrong");
    assert(Number(accountAfterUpdate.creationSlot) <= await connection.getSlot(), "Slot set wrong");
    assert(Number(accountAfterUpdate.creationSlot) > (await connection.getSlot()) - 5, "Slot set outside of 5 block tolerance");

    var verifierStateInfo = await provider.connection.getAccountInfo(
      pdas.verifierStatePubkey
    )
    const verifierStateInfoUpdate = verifierProgram.account.verifierState._coder.accounts.decode('VerifierState', verifierStateInfo.data);
    assert(verifierStateInfoUpdate.signingAddress.toBase58() == relayer_pubkey.toBase58(), "relayer_pubkey insert wrong");

    console.log("checkEscrowAccountCreated success")
  }

  async function checkVerifierStateAccountCreated({connection, pda, ix_data, relayer_pubkey}) {
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

  async function checkFinalExponentiationSuccess({connection, pda, ix_data}) {
    var userAccountInfo = await provider.connection.getAccountInfo(pda)

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

  async function checkLastTxSuccess({
    connection,
    pdas,
    sender,
    senderAccountBalancePriorLastTx,
    recipient,
    recipientBalancePriorLastTx,
    ix_data,
    mode
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
    checkRentExemption({
      account: nullifier0Account,
      connection: provider.connection
    })

    var nullifier1Account = await connection.getAccountInfo(
      pdas.nullifier0PdaPubkey
    )

    checkRentExemption({
      account: nullifier1Account,
      connection: provider.connection
    })

    var leavesAccount = await provider.connection.getAccountInfo(
      pdas.leavesPdaPubkey
    )

    let leavesAccountData = unpackLeavesAccount(leavesAccount.data)
    checkRentExemption({
      account: leavesAccount,
      connection: provider.connection
    })

    assert_eq(leavesAccountData.leafLeft, ix_data.leafLeft, "left leaf not inserted correctly")
    assert_eq(leavesAccountData.leafRight, ix_data.leafRight, "right leaf not inserted correctly")
    assert_eq(leavesAccountData.encryptedUtxos, ix_data.encryptedUtxos, "encryptedUtxos not inserted correctly")
    assert(leavesAccountData.leafType == 7);

    var preInsertedLeavesIndexAccount = await provider.connection.getAccountInfo(
      PRE_INSERTED_LEAVES_INDEX
    )

    const preInsertedLeavesIndexAccountAfterUpdate = merkleTreeProgram.account.preInsertedLeavesIndex._coder.accounts.decode('PreInsertedLeavesIndex', preInsertedLeavesIndexAccount.data);

    assert(Number(preInsertedLeavesIndexAccountAfterUpdate.nextIndex) == Number(leavesAccountData.leafIndex) + 2)

    var senderAccount = await provider.connection.getAccountInfo(sender)
    var recipientAccount = await provider.connection.getAccountInfo(recipient)

    if (mode == "deposit") {

      console.log(`Balance now ${senderAccount.lamports} balance beginning ${senderAccountBalancePriorLastTx}`)
      assert(senderAccount.lamports == (I64(senderAccountBalancePriorLastTx) - I64.readLE(ix_data.extAmount, 0)).toString(), "amount not transferred correctly");

      console.log(`Balance now ${recipientAccount.lamports} balance beginning ${recipientBalancePriorLastTx}`)
      console.log(`Balance now ${recipientAccount.lamports} balance beginning ${(I64(recipientBalancePriorLastTx) + I64.readLE(ix_data.extAmount, 0)).toString()}`)
      assert(recipientAccount.lamports == (I64(recipientBalancePriorLastTx).add(I64.readLE(ix_data.extAmount, 0))).toString(), "amount not transferred correctly");

    } else if (mode == "withdrawal") {
      var senderAccount = await provider.connection.getAccountInfo(sender)
      console.log("senderAccount: ", senderAccount)
      console.log("senderAccountBalancePriorLastTx: ", senderAccountBalancePriorLastTx)

      console.log(`Balance now ${senderAccount.lamports} balance beginning ${I64(senderAccountBalancePriorLastTx).add(I64.readLE(ix_data.extAmount, 0)).sub(I64(relayerFee))} ${I64.readLE(ix_data.extAmount, 0)} ${I64(relayerFee)}`)
      assert(senderAccount.lamports == ((I64(senderAccountBalancePriorLastTx).add(I64.readLE(ix_data.extAmount, 0))).sub(I64(relayerFee))).toString(), "amount not transferred correctly");

      var recipientAccount = await provider.connection.getAccountInfo(recipient)
      console.log(`Balance now ${recipientAccount.lamports} balance beginning ${I64(recipientBalancePriorLastTx).toString()}`)

      // console.log(`Balance now ${recipientAccount.lamports} balance beginning ${((I64(recipientBalancePriorLastTx).sub(I64.readLE(ix_data.extAmount, 0))).add(I64(relayerFee))).toString()}`)
      assert(recipientAccount.lamports == ((I64(recipientBalancePriorLastTx).sub(I64.readLE(ix_data.extAmount, 0))).add(I64(relayerFee))).toString(), "amount not transferred correctly");

    } else {
      throw Error("mode not supplied");
    }
  }

  async function checkMerkleTreeUpdateStateCreated({
      connection,
      program,
      merkleTreeUpdateState,
      relayer,
      MerkleTree,
      leavesPdas,
      current_instruction_index
    }) {
    var merkleTreeTmpAccountInfo = await provider.connection.getAccountInfo(
      merkleTreeUpdateState
    )

    assert(merkleTreeTmpAccountInfo.owner.toBase58() === merkleTreeProgram.programId.toBase58(), "merkle tree pda owner wrong after initializing")
    const merkleTreeUpdateStateData = merkleTreeProgram.account.preInsertedLeavesIndex._coder.accounts.decode('MerkleTreeUpdateState', merkleTreeTmpAccountInfo.data);

    var MerkleTreeAccountInfo = await provider.connection.getAccountInfo(
      MerkleTree
    )
    assert(merkleTreeUpdateStateData.relayer.toBase58() == relayer.toBase58(), "The incorrect signer has been saved")
    assert(merkleTreeUpdateStateData.merkleTreePdaPubkey.toBase58()== MerkleTree.toBase58(), "the incorrect merkle tree pubkey was saved")
    assert(merkleTreeUpdateStateData.numberOfLeaves== leavesPdas.length, "The incorrect number of leaves was saved")
    console.log(`Current instruction index is ${merkleTreeUpdateStateData.currentInstructionIndex} should be ${current_instruction_index}`)
    assert(merkleTreeUpdateStateData.currentInstructionIndex== current_instruction_index, "The instruction index is wrong")
    assert(new solana.PublicKey(MerkleTreeAccountInfo.data.slice(16658-40,16658-8)).toBase58()== merkleTreeUpdateState.toBase58());
    assert(U64.readLE(MerkleTreeAccountInfo.data.slice(16658-8,16658), 0) >= (await connection.getSlot()) - 5, "Lock has not been taken at this or in the 5 prior slots");
  }

  async function checkMerkleTreeBatchUpdateSuccess({
    connection,
    merkleTreeUpdateState,
    merkleTreeAccountPrior,
    numberOfLeaves,
    leavesPdas,
    merkleTree,
  }) {

    var merkleTreeTmpStateAccount = await connection.getAccountInfo(
          merkleTreeUpdateState
        )

    assert(merkleTreeTmpStateAccount === null, "Shielded transaction failed merkleTreeTmpStateAccount is not closed")

    var merkleTreeAccount = await connection.getAccountInfo(MERKLE_TREE_KEY)
    // Merkle tree is locked by merkleTreeUpdateState
    assert(new solana.PublicKey(merkleTreeAccount.data.slice(16658-40,16658-8)).toBase58()== new solana.PublicKey(new Uint8Array(32).fill(0)).toBase58());
    assert(U64.readLE(merkleTreeAccount.data.slice(16658-8,16658), 0) == 0, "Lock has not been taken within prior  20 slots");

    let merkle_tree_prior_leaves_index = U64.readLE(merkleTreeAccountPrior.data.slice(594, 594 + 8),0);
    console.log("merkle_tree_prior_leaves_index: ", merkle_tree_prior_leaves_index)
    let merkle_tree_prior_current_root_index = U64.readLE(merkleTreeAccountPrior.data.slice(594 - 8, 594),0).toNumber()

    let current_root_index = U64.readLE(merkleTreeAccount.data.slice(594 - 8, 594),0).toNumber()
    console.log("merkle_tree_prior_current_root_index: ", merkle_tree_prior_current_root_index)
    console.log("current_root_index: ", current_root_index)
    assert(merkle_tree_prior_current_root_index + 1 == current_root_index)
    let current_root_start_range = 610 + current_root_index * 32;
    let current_root_end_range = 610 + (current_root_index + 1) * 32;
    console.log(`root: ${BigNumber.from(merkleTreeAccount.data.slice(current_root_start_range, current_root_end_range).reverse()).toHexString()}`)

    console.log(`prior +${numberOfLeaves} ${merkle_tree_prior_leaves_index.add(U64(numberOfLeaves)).toString()}, now ${U64.readLE(merkleTreeAccount.data.slice(594, 594 + 8), 0).toString()}
    `)
    // index has increased by numberOfLeaves
    console.log(`index has increased by numberOfLeaves: ${merkle_tree_prior_leaves_index.add(U64(numberOfLeaves)).toString()}, ${U64.readLE(merkleTreeAccount.data.slice(594, 594 + 8), 0).toString()}`)
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
    assert(merkleTree.root().toHexString() == (await light.buildMerkelTree(provider.connection)).root().toHexString());

  }



  function unpackLeavesAccount(leavesAccountData) {
    return{
      leafType: leavesAccountData[1],
      leafIndex:    U64.readLE(leavesAccountData.slice(2,10),0),
      leafLeft:     Array.prototype.slice.call(leavesAccountData.slice(10, 42)),
      leafRight:    Array.prototype.slice.call(leavesAccountData.slice(42, 74)),
      encryptedUtxos: Array.prototype.slice.call(leavesAccountData.slice(106,328)),
      merkleTree:   Array.prototype.slice.call(leavesAccountData.slice(74, 106)),
    }
  }
  async function checkRentExemption({
    connection,
    account
  }) {
    let requiredBalance = connection.getMinimumBalanceForRentExemption(account.data.length);
    if (account.lamports  < requiredBalance) {
      throw Error(`Account of size ${account.data.length} not rentexempt balance ${account.lamports} should be${requiredBalance}`)
    }

  }
});
