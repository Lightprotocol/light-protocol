import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { VerifierProgram } from "../target/types/verifier_program";
const { SystemProgram } = require('@solana/web3.js');
import { findProgramAddressSync } from "@project-serum/anchor/dist/cjs/utils/pubkey";
import fs from 'fs';
const solana = require("@solana/web3.js");
const {U64, I64} = require('n64');
import nacl from "tweetnacl";
import { BigNumber } from 'ethers'
import { IDL, MerkleTreeProgram } from "../target/types/merkle_tree_program";
import _ from "lodash";
import { Keypair, PublicKey, SYSVAR_CLOCK_PUBKEY, SYSVAR_RENT_PUBKEY } from "@solana/web3.js";
import { ASSOCIATED_TOKEN_PROGRAM_ID, TOKEN_PROGRAM_ID } from "@solana/spl-token";
const TYPE_SEED = {defined: "&[u8]"};
const constants:any = {};
IDL.constants.map((item) => {
  if(_.isEqual(item.type, TYPE_SEED)) {
    constants[item.name] = item.value.replace("b\"", "").replace("\"", "");
  } else //if(_.isEqual(item.type, TYPE_PUBKEY) || _.isEqual(item.type, TYPE_INIT_DATA))
  {
    constants[item.name] = JSON.parse(item.value)
  }
});
export const DEFAULT_PROGRAMS = {
  systemProgram: SystemProgram.programId,
  tokenProgram: TOKEN_PROGRAM_ID,
  associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
  rent: SYSVAR_RENT_PUBKEY,
  clock: SYSVAR_CLOCK_PUBKEY,
};

const ADMIN_AUTH_PRIVATE_KEY = [
  17, 34, 231, 31, 83, 147, 93, 173, 61, 164, 25, 0, 204, 82, 234, 91, 202, 187, 228, 110, 146,
  97, 112, 131, 180, 164, 96, 220, 57, 207, 65, 107, 2, 99, 226, 251, 88, 66, 92, 33, 25, 216,
  211, 185, 112, 203, 212, 238, 105, 144, 72, 121, 176, 253, 106, 168, 115, 158, 154, 188, 62,
  255, 166, 81,
];
const ADMIN_AUTH_KEY = new PublicKey(new Uint8Array(constants.MERKLE_TREE_INIT_AUTHORITY));
const ADMIN_AUTH_KEYPAIR = Keypair.fromSecretKey(new Uint8Array(ADMIN_AUTH_PRIVATE_KEY));

const MERKLE_TREE_KP = Keypair.generate();
const MERKLE_TREE_KEY = MERKLE_TREE_KP.publicKey;
const MERKLE_TREE_SIZE = 16658;

const VERIFIER_KEY = Array(32).fill(0);

const NODE_LEFT = Array(32).fill(2);
const NODE_RIGHT = Array(32).fill(2);
const ROOT_HASH = Array(32).fill(0);

const INIT_STORAGE_DATA = []
      .concat(NODE_LEFT)
      .concat(NODE_RIGHT)
      .concat(ROOT_HASH)
      .concat([...ADMIN_AUTH_KEY.toBytes()])
      .concat([...MERKLE_TREE_KEY.toBytes().values()])
      .concat(VERIFIER_KEY)

const newAccountWithLamports = async (connection, lamports = 1e10) => {
  const account = new anchor.web3.Account()

  let retries = 30
  await connection.requestAirdrop(account.publicKey, lamports)
  for (;;) {
    await sleep(500)
    // eslint-disable-next-line eqeqeq
    if (lamports == (await connection.getBalance(account.publicKey))) {
      return account
    }
    if (--retries <= 0) {
      break
    }
  }
  throw new Error(`Airdrop of ${lamports} failed`)
}
const sleep = (ms) => {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

describe("verifier_program with merkle tree", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const provider = anchor.AnchorProvider.local();

  const program = anchor.workspace.VerifierProgram as Program<VerifierProgram>;
  const merkleTreeProgram = anchor.workspace.MerkleTreeProgram as Program<MerkleTreeProgram>;

  const connection = program.provider.connection;

  const [VERIFIER_TMP_STORAGE_KEY] = PublicKey.findProgramAddressSync(
    [Buffer.from(new Uint8Array(/*CONCAT_DATA.slice(73, 105)*/NODE_LEFT)), Buffer.from(constants.STORAGE_SEED)],
    merkleTreeProgram.programId
  );
  const [MERKLE_TREE_TMP_STORAGE_KEY] = PublicKey.findProgramAddressSync(
    [Buffer.from(new Uint8Array(/*CONCAT_DATA.slice(73, 105)*/NODE_LEFT)), Buffer.from(constants.STORAGE_SEED)],
    merkleTreeProgram.programId
  );

  before(async () => {
    const airdropTx = await connection.requestAirdrop(ADMIN_AUTH_KEY, 100_000_000_000_000);
    await connection.confirmTransaction(airdropTx);
  })

  it("Initialize Merkle Tree", async () => {

    const tx = await merkleTreeProgram.methods.initializeNewMerkleTree().accounts({
      authority: ADMIN_AUTH_KEY,
      merkleTree: MERKLE_TREE_KEY,
      ...DEFAULT_PROGRAMS
    })
    .preInstructions([
      SystemProgram.createAccount({
        fromPubkey: ADMIN_AUTH_KEY,
        newAccountPubkey: MERKLE_TREE_KEY,
        space: MERKLE_TREE_SIZE,
        lamports: await connection.getMinimumBalanceForRentExemption(MERKLE_TREE_SIZE),
        programId: merkleTreeProgram.programId,
      })
    ])
    .signers([ADMIN_AUTH_KEYPAIR, MERKLE_TREE_KP])
    .rpc();
  });

  it("Init Merkle Tree Storage via Verifier", async () => {
    const tx = await program.methods.createMerkleTreeTmpStorage(Buffer.from(INIT_STORAGE_DATA)).accounts({
      authority: ADMIN_AUTH_KEY,
      verifierTmp: VERIFIER_TMP_STORAGE_KEY,
      merkleTreeTmpStorage: MERKLE_TREE_TMP_STORAGE_KEY,
      merkleTreeProgram: merkleTreeProgram.programId,
      ...DEFAULT_PROGRAMS
    })
    .signers([ADMIN_AUTH_KEYPAIR])
    .rpc();
  });
})