const solana = require("@solana/web3.js");
import * as anchor from "@coral-xyz/anchor";
import { BN } from "@coral-xyz/anchor";
import {
  Connection,
  Keypair,
  PublicKey,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
const { SystemProgram } = require("@solana/web3.js");
// const token = require('@solana/spl-token')
// @ts-ignore
var _ = require("lodash");
import {
  createAccount,
  getAccount,
  getAssociatedTokenAddressSync,
  mintTo,
  MINT_SIZE,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { Account } from "../account";

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
  // userTokenAccount,
  confirmConfig,
  AUTHORITY_ONE,
  TOKEN_REGISTRY,
  sleep,
  confirmTransaction,
  BN_0,
} from "../index";
import { Program } from "@coral-xyz/anchor";
let circomlibjs = require("circomlibjs");

// TODO: check whether we need all of these functions

export const newAccountWithLamports = async (
  connection: Connection,
  account = Keypair.generate(),
  lamports = 1e10,
) => {
  const signature = await connection.requestAirdrop(
    account.publicKey,
    lamports,
  );
  await confirmTransaction(connection, signature);
  console.log("newAccountWithLamports ", account.publicKey.toBase58());
  return account;
};

export const newAddressWithLamports = async (
  connection: Connection,
  address = new anchor.web3.Account().publicKey,
  lamports = 1e11,
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
      let signature = await connection.requestAirdrop(payer.publicKey, 1e7);
      await confirmTransaction(connection, signature);

      const tx = new solana.Transaction().add(
        solana.SystemProgram.createAccount({
          fromPubkey: payer.publicKey,
          newAccountPubkey: account.publicKey,
          space: 0,
          lamports: await connection.getMinimumBalanceForRentExemption(1),
          Id: owner.programId,
        }),
      );

      tx.feePayer = payer.publicKey;
      tx.recentBlockhash = await connection.getLatestBlockhash();
      await solana.sendAndConfirmTransaction(connection, tx, [payer, account], {
        commitment: "confirmed",
        preflightCommitment: "confirmed",
      });
      return account;
    } catch {}

    retry++;
  }
  throw "Can't create program account with lamports";
};

// FIXME: doesn't need a keypair for userAccount...
export async function newAccountWithTokens({
  connection,
  MINT,
  ADMIN_AUTH_KEYPAIR,
  userAccount,
  amount,
}: {
  connection: Connection;
  MINT: PublicKey;
  ADMIN_AUTH_KEYPAIR: Keypair;
  userAccount: Keypair;
  amount: BN;
}): Promise<any> {
  let tokenAccount = await createAccount(
    connection,
    ADMIN_AUTH_KEYPAIR,
    MINT,
    userAccount.publicKey,
  );

  try {
    await mintTo(
      connection,
      ADMIN_AUTH_KEYPAIR,
      MINT,
      tokenAccount,
      ADMIN_AUTH_KEYPAIR.publicKey,
      amount.toNumber(),
      [],
    );
    //FIXME: remove this
  } catch (e) {
    console.log("mintTo error", e);
    await mintTo(
      connection,
      ADMIN_AUTH_KEYPAIR,
      MINT,
      tokenAccount,
      ADMIN_AUTH_KEYPAIR.publicKey,
      //@ts-ignore
      amount,
      [],
    );
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
      }),
    );

    let res = await sendAndConfirmTransaction(
      connection,
      txCreateAccount,
      [authorityKeypair, mintKeypair],
      confirmConfig,
    );
    const transactionResult = await connection.getTransaction(res, {
      commitment: "confirmed",
    });
    if (transactionResult === null) {
      throw new Error("create mint account failed");
    }

    let mint = await createMint(
      connection,
      authorityKeypair,
      authorityKeypair.publicKey,
      null, // freez auth
      decimals, //2,
      mintKeypair,
    );
    const accountInfo = await connection.getAccountInfo(mint);
    if (accountInfo === null) {
      throw new Error("create mint failed");
    }

    return mintKeypair.publicKey;
  } catch (e) {
    console.log(e);
  }
}

export async function createTestAccounts(
  connection: Connection,
  userTokenAccount?: PublicKey,
) {
  // const connection = new Connection('http://127.0.0.1:8899', 'confirmed');

  let balance = await connection.getBalance(ADMIN_AUTH_KEY, "confirmed");
  if (balance === 0) {
    let amount = 1_000_000_000_000;

    let signature = await connection.requestAirdrop(ADMIN_AUTH_KEY, amount);
    await confirmTransaction(connection, signature);

    let Newbalance = await connection.getBalance(ADMIN_AUTH_KEY);

    if (Newbalance !== balance + amount) {
      throw new Error("airdrop failed");
    }

    let signature2 = await connection.requestAirdrop(AUTHORITY_ONE, amount);
    await confirmTransaction(connection, signature2);
    // subsidising transactions
    let txTransfer1 = new solana.Transaction().add(
      solana.SystemProgram.transfer({
        fromPubkey: ADMIN_AUTH_KEYPAIR.publicKey,
        toPubkey: AUTHORITY,
        lamports: 3_000_000_000,
      }),
    );

    await sendAndConfirmTransaction(
      connection,
      txTransfer1,
      [ADMIN_AUTH_KEYPAIR],
      confirmConfig,
    );
  }

  if (
    (await connection.getBalance(
      Keypair.fromSecretKey(MINT_PRIVATE_KEY).publicKey,
      "confirmed",
    )) == 0
  ) {
    await createMintWrapper({
      authorityKeypair: ADMIN_AUTH_KEYPAIR,
      mintKeypair: Keypair.fromSecretKey(MINT_PRIVATE_KEY),
      connection,
    });
    console.log(
      "created mint ",
      Keypair.fromSecretKey(MINT_PRIVATE_KEY).publicKey.toBase58(),
    );
  }

  let balanceUserToken: null | any = null;
  let userSplAccount: PublicKey | null = null;
  try {
    let tokenCtx = TOKEN_REGISTRY.get("USDC");
    if (userTokenAccount) {
      userSplAccount = userTokenAccount;
    } else {
      userSplAccount = getAssociatedTokenAddressSync(
        tokenCtx!.mint,
        ADMIN_AUTH_KEYPAIR.publicKey,
      );
    }
    console.log(
      "test setup: admin spl acc",
      userSplAccount.toBase58(),
      userTokenAccount?.toBase58(),
    );

    balanceUserToken = await getAccount(
      connection,
      userSplAccount, //userTokenAccount,
      "confirmed",
      TOKEN_PROGRAM_ID,
    );
  } catch (e) {}

  try {
    if (balanceUserToken == null) {
      // create associated token account
      await newAccountWithTokens({
        connection: connection,
        MINT,
        ADMIN_AUTH_KEYPAIR,
        userAccount: userTokenAccount ? USER_TOKEN_ACCOUNT : ADMIN_AUTH_KEYPAIR, //USER_TOKEN_ACCOUNT, // this is to support legacy tests
        amount: new BN(100_000_000_0000),
      });
    }
  } catch (error) {
    console.log(error);
  }
  console.log("userSplAccount ", userSplAccount?.toBase58());

  try {
    if (balanceUserToken == null) {
      // create associated token account
      await newAccountWithTokens({
        connection: connection,
        MINT,
        ADMIN_AUTH_KEYPAIR,
        userAccount: RECIPIENT_TOKEN_ACCOUNT,
        amount: BN_0,
      });
    }
  } catch (error) {}

  let POSEIDON = await circomlibjs.buildPoseidonOpt();
  let KEYPAIR = new Account({
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
