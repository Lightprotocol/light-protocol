const solana = require("@solana/web3.js");
import * as anchor from "@project-serum/anchor";
import { Connection, Keypair } from "@solana/web3.js";
const { SystemProgram } = require('@solana/web3.js');
const token = require('@solana/spl-token')
var _ = require('lodash');
import {getAccount, TOKEN_PROGRAM_ID} from "@solana/spl-token"

import {
  ADMIN_AUTH_KEYPAIR,
  AUTHORITY,
  MINT_PRIVATE_KEY,
  MINT,
  KEYPAIR_PRIVKEY,
  USER_TOKEN_ACCOUNT,
  RECIPIENT_TOKEN_ACCOUNT,
  ADMIN_AUTH_KEY,
  userTokenAccount
} from "../constants"
let circomlibjs = require("circomlibjs");

// TODO: check whether we need all of these functions
const sleep = (ms) => {
  return new Promise((resolve) => setTimeout(resolve, ms))
}
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
          Id: owner.programId,
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



export async function newAccountWithTokens ({
  connection,
  MINT,
  ADMIN_AUTH_KEYPAIR,
  userAccount,
  amount
}): Promise<any> {

  let tokenAccount
  try {
    console.log("userAccount.publicKey: ", userAccount.publicKey.toBase58());

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

export async function createMint({authorityKeypair, mintKeypair = new Keypair(),nft = false, decimals = 2, provider}) {
  if (nft == true) {
    decimals = 0;
  }
  // await provider.connection.confirmTransaction(await provider.connection.requestAirdrop(mintKeypair.publicKey, 1_000_000, {preflightCommitment: "confirmed", commitment: "confirmed"}));

  try {
    let space = token.MINT_SIZE

    let txCreateAccount = new solana.Transaction().add(
      SystemProgram.createAccount({
        fromPubkey: authorityKeypair.publicKey,
        lamports: provider.connection.getMinimumBalanceForRentExemption(space),
        newAccountPubkey: mintKeypair.publicKey,
        programId: token.TOKEN_PROGRAM_ID,
        space: space

      })
    )

    let res = await solana.sendAndConfirmTransaction(provider.connection, txCreateAccount, [authorityKeypair, mintKeypair], {commitment: "finalized", preflightCommitment: 'finalized',});

    let mint = await token.createMint(
      provider.connection,
      authorityKeypair,
      authorityKeypair.publicKey,
      null, // freez auth
      decimals, //2,
      mintKeypair
    );
    // assert(MINT.toBase58() == mint.toBase58());
    console.log("mintKeypair.publicKey: ", mintKeypair.publicKey.toBase58());
    return mintKeypair.publicKey;
  } catch(e) {
    console.log(e)
  }

}


export async function createTestAccounts(provider: anchor.Provider) {
  const connection = provider.connection;

  let balance = await connection.getBalance(ADMIN_AUTH_KEY, "confirmed");
    if (balance === 0) {
      await provider.connection.confirmTransaction(await provider.connection.requestAirdrop(ADMIN_AUTH_KEY, 1_000_000_000_000), "finalized")
          // subsidising transactions
      let txTransfer1 = new solana.Transaction().add(solana.SystemProgram.transfer({fromPubkey:ADMIN_AUTH_KEYPAIR.publicKey, toPubkey: AUTHORITY, lamports: 1_000_000_000}));
      await provider.sendAndConfirm(txTransfer1, [ADMIN_AUTH_KEYPAIR]);
    }
    
    if (await connection.getBalance(solana.Keypair.fromSecretKey(MINT_PRIVATE_KEY).publicKey, "finalized") == 0) {
      await createMint({
        authorityKeypair: ADMIN_AUTH_KEYPAIR,
        mintKeypair: solana.Keypair.fromSecretKey(MINT_PRIVATE_KEY),
        provider
      })
      console.log("created mint");
    }
    
    let balanceUserToken = null
    

    try {
      balanceUserToken = await getAccount(
        connection,
        userTokenAccount,
        "confirmed",
        TOKEN_PROGRAM_ID
      );
    } catch(e) {
      
    }
    console.log( "balanceUserToken ", balanceUserToken);
    
    try {
      if (balanceUserToken == null) {
        // create associated token account
         (await newAccountWithTokens({
          connection: provider.connection,
          MINT,
          ADMIN_AUTH_KEYPAIR,
          userAccount: USER_TOKEN_ACCOUNT,
          amount: 100_000_000_0000
        }))
      }
    } catch (error) {
      console.log(error);
      
    }

    try {
      if (balanceUserToken == null) {
        // create associated token account
        (await newAccountWithTokens({
          connection: provider.connection,
          MINT,
          ADMIN_AUTH_KEYPAIR,
          userAccount: RECIPIENT_TOKEN_ACCOUNT,
          amount: 0
        }))
      }
    } catch (error) {
    }
    
    // let merkleTreeConfig = new MerkleTreeConfig({merkleTreePubkey: MERKLE_TREE_KEY,payer: ADMIN_AUTH_KEYPAIR, connection: provider.connection })
    // MERKLE_TREE_AUTHORITY_PDA = await merkleTreeConfig.getMerkleTreeAuthorityPda();
    let POSEIDON = await circomlibjs.buildPoseidonOpt();

    let KEYPAIR = new Keypair(POSEIDON, KEYPAIR_PRIVKEY)
    let RELAYER_RECIPIENT = new anchor.web3.Account().publicKey;
    return {
      POSEIDON,
      KEYPAIR,
      RELAYER_RECIPIENT
    }
}