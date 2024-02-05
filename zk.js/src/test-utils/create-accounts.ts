import * as anchor from "@coral-xyz/anchor";
import { BN } from "@coral-xyz/anchor";
import {
  Connection,
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  sendAndConfirmTransaction,
  SystemProgram,
  Transaction
} from "@solana/web3.js";
// @ts-ignore
const _ = require("lodash");
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
import { WasmFactory } from "@lightprotocol/account.rs";
import { Program } from "@coral-xyz/anchor";

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
  lamports: number;
}) => {
  const account = new anchor.web3.Account();
  const payer = new anchor.web3.Account();
  let retry = 0;
  while (retry < 30) {
    try {
      const signature = await connection.requestAirdrop(payer.publicKey, 1e7);
      await confirmTransaction(connection, signature);

      const tx = new Transaction().add(
        SystemProgram.createAccount({
          fromPubkey: payer.publicKey,
          newAccountPubkey: account.publicKey,
          space: 0,
          lamports: await connection.getMinimumBalanceForRentExemption(1),
          programId: owner.programId,
        }),
      );

      tx.feePayer = payer.publicKey;      
      await sendAndConfirmTransaction(connection, tx, [payer, account], {
        commitment: "confirmed",
        preflightCommitment: "confirmed",
      });
      return account;
    } catch (_) {
      /* empty */
    }

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
  const tokenAccount = await createAccount(
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
  nft?: boolean;
  decimals?: number;
  connection: Connection;
}): Promise<PublicKey> {
  if (nft) {
    decimals = 0;
  }

  const space = MINT_SIZE;

  const requiredLamports = await connection.getMinimumBalanceForRentExemption(space);
  const txCreateAccount = new Transaction().add(
    SystemProgram.createAccount({
      fromPubkey: authorityKeypair.publicKey,
      lamports: requiredLamports,
      newAccountPubkey: mintKeypair.publicKey,
      programId: TOKEN_PROGRAM_ID,
      space: space,
    }),
  );

  const res = await sendAndConfirmTransaction(
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

  const mint = await createMint(
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
}

export async function createTestAccounts(
  connection: Connection,
  userTokenAccount?: PublicKey,
) {
  // const connection = new Connection('http://127.0.0.1:8899', 'confirmed');

  const balance = await connection.getBalance(ADMIN_AUTH_KEY, "confirmed");
  const amount = 500 * LAMPORTS_PER_SOL;
  if (balance < amount) {
    const signature = await connection.requestAirdrop(ADMIN_AUTH_KEY, amount);
    await confirmTransaction(connection, signature);

    const newBalance = await connection.getBalance(ADMIN_AUTH_KEY);

    if (newBalance !== balance + amount) {
      throw new Error("airdrop failed");
    }

    const signature2 = await connection.requestAirdrop(AUTHORITY_ONE, amount);
    await confirmTransaction(connection, signature2);
    // subsidising transactions
    const txTransfer1 = new Transaction().add(
      SystemProgram.transfer({
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
    const tokenCtx = TOKEN_REGISTRY.get("USDC");
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
  } catch (_) {
    /* empty */
  }

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
  } catch (_) {
    /* empty */
  }

  const WASM = await WasmFactory.getInstance();
  const ACCOUNT = Account.createFromSeed(WASM, KEYPAIR_PRIVKEY.toString());
  const RPC_RECIPIENT = new anchor.web3.Account().publicKey;
  return {
    WASM,
    ACCOUNT,
    RPC_RECIPIENT,
  };
}
