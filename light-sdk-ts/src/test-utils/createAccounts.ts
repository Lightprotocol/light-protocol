const solana = require("@solana/web3.js");
import * as anchor from "@coral-xyz/anchor";
import {
  Connection,
  Keypair,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
const { SystemProgram } = require("@solana/web3.js");
// const token = require('@solana/spl-token')
var _ = require("lodash");
import {
  createAccount,
  getAccount,
  mintTo,
  MINT_SIZE,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { Keypair as ShieldedKeypair } from "../keypair";

import { createMint } from "@solana/spl-token";
import {
  ADMIN_AUTH_KEYPAIR,
  AUTHORITY,
  MINT_PRIVATE_KEY,
  MINT,
  KEYPAIR_PRIVKEY,
  USER_TOKEN_ACCOUNT,
  RECIPIENT_TOKEN_ACCOUNT,
  ADMIN_AUTH_KEY,
  userTokenAccount,
  confirmConfig,
} from "../index";
import { assert } from "chai";
import { Program } from "@coral-xyz/anchor";
let circomlibjs = require("circomlibjs");

// TODO: check whether we need all of these functions
const sleep = (ms) => {
  return new Promise((resolve) => setTimeout(resolve, ms));
};
export const newAccountWithLamports = async (
  connection: Connection,
  account = solana.Keypair.generate(),
  lamports = 1e10
) => {
  let x = await connection.confirmTransaction(
    await connection.requestAirdrop(account.publicKey, lamports),
    "confirmed"
  );
  console.log("newAccountWithLamports ", account.publicKey.toBase58());
  return account;
};

export const newAddressWithLamports = async (
  connection: Connection,
  address = new anchor.web3.Account().publicKey,
  lamports = 1e11
) => {
  let retries = 30;
  await connection.requestAirdrop(address, lamports);
  for (;;) {
    await sleep(500);
    // eslint-disable-next-line eqeqeq
    if (lamports == (await connection.getBalance(address))) {
      console.log(`Airdropped ${lamports} to ${address.toBase58()}`);
      return address;
    }
    if (--retries <= 0) {
      break;
    }
  }
  throw new Error(`Airdrop of ${lamports} failed`);
};

export const newProgramOwnedAccount = async ({
  connection,
  owner,
  lamports = 0,
}: {
  connection: Connection;
  owner: Program;
  lamports: Number;
}) => {
  let account = new anchor.web3.Account();
  let payer = new anchor.web3.Account();
  let retry = 0;
  while (retry < 30) {
    try {
      await connection.confirmTransaction(
        await connection.requestAirdrop(payer.publicKey, 1e7),
        "confirmed"
      );

      const tx = new solana.Transaction().add(
        solana.SystemProgram.createAccount({
          fromPubkey: payer.publicKey,
          newAccountPubkey: account.publicKey,
          space: 0,
          lamports: await connection.getMinimumBalanceForRentExemption(1),
          Id: owner.programId,
        })
      );

      tx.feePayer = payer.publicKey;
      tx.recentBlockhash = await connection.getRecentBlockhash();
      let x = await solana.sendAndConfirmTransaction(
        connection,
        tx,
        [payer, account],
        {
          commitment: "confirmed",
          preflightCommitment: "confirmed",
        }
      );
      return account;
    } catch {}

    retry++;
  }
  throw "Can't create program account with lamports";
};

export async function newAccountWithTokens({
  connection,
  MINT,
  ADMIN_AUTH_KEYPAIR,
  userAccount,
  amount,
}): Promise<any> {
  let tokenAccount;
  try {
    tokenAccount = await createAccount(
      connection,
      ADMIN_AUTH_KEYPAIR,
      MINT,
      userAccount.publicKey
      // userAccount
    );

    console.log(tokenAccount);
  } catch (e) {
    console.log(e);
  }

  try {
    await mintTo(
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

export async function createMintWrapper({
  authorityKeypair,
  mintKeypair = new Keypair(),
  nft = false,
  decimals = 2,
  connection,
}: {
  authorityKeypair: Keypair;
  mintKeypair?: Keypair;
  nft?: Boolean;
  decimals?: number;
  connection: Connection;
}) {
  if (nft == true) {
    decimals = 0;
  }

  try {
    let space = MINT_SIZE;

    let txCreateAccount = new solana.Transaction().add(
      SystemProgram.createAccount({
        fromPubkey: authorityKeypair.publicKey,
        lamports: connection.getMinimumBalanceForRentExemption(space),
        newAccountPubkey: mintKeypair.publicKey,
        programId: TOKEN_PROGRAM_ID,
        space: space,
      })
    );

    let res = await sendAndConfirmTransaction(
      connection,
      txCreateAccount,
      [authorityKeypair, mintKeypair],
      confirmConfig
    );
    assert(
      (await connection.getTransaction(res, {
        commitment: "confirmed",
      })) != null,
      "create mint account failed"
    );
    let mint = await createMint(
      connection,
      authorityKeypair,
      authorityKeypair.publicKey,
      null, // freez auth
      decimals, //2,
      mintKeypair
    );
    assert(
      (await connection.getAccountInfo(mint)) != null,
      "create mint failed"
    );
    return mintKeypair.publicKey;
  } catch (e) {
    console.log(e);
  }
}

export async function createTestAccounts(connection: Connection) {
  // const connection = new Connection('http://127.0.0.1:8899', 'confirmed');

  let balance = await connection.getBalance(ADMIN_AUTH_KEY, "confirmed");
  if (balance === 0) {
    let amount = 1_000_000_000_000;
    console.time("requestAirdrop");

    let res = await connection.requestAirdrop(ADMIN_AUTH_KEY, amount);
    console.timeEnd("requestAirdrop");
    console.time("confirmAirdrop");

    await connection.confirmTransaction(res, "confirmed");
    console.timeEnd("confirmAirdrop");

    let Newbalance = await connection.getBalance(ADMIN_AUTH_KEY);

    assert(Newbalance == balance + amount, "airdrop failed");
    // await provider.connection.confirmTransaction(, confirmConfig)
    // subsidising transactions
    let txTransfer1 = new solana.Transaction().add(
      solana.SystemProgram.transfer({
        fromPubkey: ADMIN_AUTH_KEYPAIR.publicKey,
        toPubkey: AUTHORITY,
        lamports: 1_000_000_000,
      })
    );
    await sendAndConfirmTransaction(
      connection,
      txTransfer1,
      [ADMIN_AUTH_KEYPAIR],
      confirmConfig
    );
  }

  if (
    (await connection.getBalance(
      solana.Keypair.fromSecretKey(MINT_PRIVATE_KEY).publicKey,
      "confirmed"
    )) == 0
  ) {
    await createMintWrapper({
      authorityKeypair: ADMIN_AUTH_KEYPAIR,
      mintKeypair: Keypair.fromSecretKey(MINT_PRIVATE_KEY),
      connection,
    });
    console.log("created mint");
  }

  let balanceUserToken = null;

  try {
    balanceUserToken = await getAccount(
      connection,
      userTokenAccount,
      "confirmed",
      TOKEN_PROGRAM_ID
    );
  } catch (e) {}

  try {
    if (balanceUserToken == null) {
      // create associated token account
      await newAccountWithTokens({
        connection: connection,
        MINT,
        ADMIN_AUTH_KEYPAIR,
        userAccount: USER_TOKEN_ACCOUNT,
        amount: 100_000_000_0000,
      });
    }
  } catch (error) {
    console.log(error);
  }

  try {
    if (balanceUserToken == null) {
      // create associated token account
      await newAccountWithTokens({
        connection: connection,
        MINT,
        ADMIN_AUTH_KEYPAIR,
        userAccount: RECIPIENT_TOKEN_ACCOUNT,
        amount: 0,
      });
    }
  } catch (error) {}

  let POSEIDON = await circomlibjs.buildPoseidonOpt();
  let KEYPAIR = new ShieldedKeypair({
    poseidon: POSEIDON,
    seed: KEYPAIR_PRIVKEY.toString(),
  });
  let RELAYER_RECIPIENT = new anchor.web3.Account().publicKey;
  return {
    POSEIDON,
    KEYPAIR,
    RELAYER_RECIPIENT,
  };
}
