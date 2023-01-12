"use strict";
var __createBinding =
  (this && this.__createBinding) ||
  (Object.create
    ? function (o, m, k, k2) {
        if (k2 === undefined) k2 = k;
        var desc = Object.getOwnPropertyDescriptor(m, k);
        if (
          !desc ||
          ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)
        ) {
          desc = {
            enumerable: true,
            get: function () {
              return m[k];
            },
          };
        }
        Object.defineProperty(o, k2, desc);
      }
    : function (o, m, k, k2) {
        if (k2 === undefined) k2 = k;
        o[k2] = m[k];
      });
var __setModuleDefault =
  (this && this.__setModuleDefault) ||
  (Object.create
    ? function (o, v) {
        Object.defineProperty(o, "default", { enumerable: true, value: v });
      }
    : function (o, v) {
        o["default"] = v;
      });
var __importStar =
  (this && this.__importStar) ||
  function (mod) {
    if (mod && mod.__esModule) return mod;
    var result = {};
    if (mod != null)
      for (var k in mod)
        if (k !== "default" && Object.prototype.hasOwnProperty.call(mod, k))
          __createBinding(result, mod, k);
    __setModuleDefault(result, mod);
    return result;
  };
var __awaiter =
  (this && this.__awaiter) ||
  function (thisArg, _arguments, P, generator) {
    function adopt(value) {
      return value instanceof P
        ? value
        : new P(function (resolve) {
            resolve(value);
          });
    }
    return new (P || (P = Promise))(function (resolve, reject) {
      function fulfilled(value) {
        try {
          step(generator.next(value));
        } catch (e) {
          reject(e);
        }
      }
      function rejected(value) {
        try {
          step(generator["throw"](value));
        } catch (e) {
          reject(e);
        }
      }
      function step(result) {
        result.done
          ? resolve(result.value)
          : adopt(result.value).then(fulfilled, rejected);
      }
      step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
  };
Object.defineProperty(exports, "__esModule", { value: true });
exports.createTestAccounts =
  exports.createMintWrapper =
  exports.newAccountWithTokens =
  exports.newProgramOwnedAccount =
  exports.newAddressWithLamports =
  exports.newAccountWithLamports =
    void 0;
const solana = require("@solana/web3.js");
const anchor = __importStar(require("@project-serum/anchor"));
const web3_js_1 = require("@solana/web3.js");
const { SystemProgram } = require("@solana/web3.js");
// const token = require('@solana/spl-token')
var _ = require("lodash");
const spl_token_1 = require("@solana/spl-token");
const spl_token_2 = require("@solana/spl-token");
const constants_1 = require("../constants");
const chai_1 = require("chai");
let circomlibjs = require("circomlibjs");
// TODO: check whether we need all of these functions
const sleep = (ms) => {
  return new Promise((resolve) => setTimeout(resolve, ms));
};
const newAccountWithLamports = (
  connection,
  account = new anchor.web3.Account(),
  lamports = 1e10
) =>
  __awaiter(void 0, void 0, void 0, function* () {
    let x = yield connection.confirmTransaction(
      yield connection.requestAirdrop(account.publicKey, lamports),
      {
        commitment: "comfirmed",
        preflightCommitment: "comfirmed",
      }
    );
    console.log("newAccountWithLamports ", account.publicKey.toBase58());
    return account;
  });
exports.newAccountWithLamports = newAccountWithLamports;
const newAddressWithLamports = (
  connection,
  address = new anchor.web3.Account().publicKey,
  lamports = 1e11
) =>
  __awaiter(void 0, void 0, void 0, function* () {
    let retries = 30;
    yield connection.requestAirdrop(address, lamports);
    for (;;) {
      yield sleep(500);
      // eslint-disable-next-line eqeqeq
      if (lamports == (yield connection.getBalance(address))) {
        console.log(`Airdropped ${lamports} to ${address.toBase58()}`);
        return address;
      }
      if (--retries <= 0) {
        break;
      }
    }
    throw new Error(`Airdrop of ${lamports} failed`);
  });
exports.newAddressWithLamports = newAddressWithLamports;
const newProgramOwnedAccount = ({ connection, owner, lamports = 0 }) =>
  __awaiter(void 0, void 0, void 0, function* () {
    let account = new anchor.web3.Account();
    let payer = new anchor.web3.Account();
    let retry = 0;
    while (retry < 30) {
      try {
        yield connection.confirmTransaction(
          yield connection.requestAirdrop(payer.publicKey, 1e7),
          {
            commitment: "comfirmed",
            preflightCommitment: "comfirmed",
          }
        );
        const tx = new solana.Transaction().add(
          solana.SystemProgram.createAccount({
            fromPubkey: payer.publicKey,
            newAccountPubkey: account.publicKey,
            space: 0,
            lamports: yield connection.getMinimumBalanceForRentExemption(1),
            Id: owner.programId,
          })
        );
        tx.feePayer = payer.publicKey;
        tx.recentBlockhash = yield connection.getRecentBlockhash();
        // tx.sign([payer])
        // console.log("getMinimumBalanceForRentExemption: ", )
        let x = yield solana.sendAndConfirmTransaction(
          connection,
          tx,
          [payer, account],
          {
            commitment: "confirmed",
            preflightCommitment: "confirmed",
          }
        );
        return account;
      } catch (_a) {}
      retry++;
    }
    throw "Can't create program account with lamports";
  });
exports.newProgramOwnedAccount = newProgramOwnedAccount;
function newAccountWithTokens({
  connection,
  MINT,
  ADMIN_AUTH_KEYPAIR,
  userAccount,
  amount,
}) {
  return __awaiter(this, void 0, void 0, function* () {
    let tokenAccount;
    try {
      console.log("userAccount.publicKey: ", userAccount.publicKey.toBase58());
      console.log(MINT);
      console.log(ADMIN_AUTH_KEYPAIR.publicKey.toBase58());
      tokenAccount = yield (0, spl_token_1.createAccount)(
        connection,
        ADMIN_AUTH_KEYPAIR,
        MINT,
        userAccount.publicKey
      );
      console.log(tokenAccount);
    } catch (e) {
      console.log(e);
    }
    console.log("fere");
    try {
      yield (0,
      spl_token_1.mintTo)(connection, ADMIN_AUTH_KEYPAIR, MINT, tokenAccount, ADMIN_AUTH_KEYPAIR.publicKey, amount, []);
    } catch (e) {
      console.log(e);
    }
    return tokenAccount;
  });
}
exports.newAccountWithTokens = newAccountWithTokens;
function createMintWrapper({
  authorityKeypair,
  mintKeypair = new web3_js_1.Keypair(),
  nft = false,
  decimals = 2,
  connection,
}) {
  return __awaiter(this, void 0, void 0, function* () {
    if (nft == true) {
      decimals = 0;
    }
    // await provider.connection.confirmTransaction(await provider.connection.requestAirdrop(mintKeypair.publicKey, 1_000_000, {preflightCommitment: "confirmed", commitment: "confirmed"}));
    try {
      let space = spl_token_1.MINT_SIZE;
      let txCreateAccount = new solana.Transaction().add(
        SystemProgram.createAccount({
          fromPubkey: authorityKeypair.publicKey,
          lamports: connection.getMinimumBalanceForRentExemption(space),
          newAccountPubkey: mintKeypair.publicKey,
          programId: spl_token_1.TOKEN_PROGRAM_ID,
          space: space,
        })
      );
      let res = yield (0, web3_js_1.sendAndConfirmTransaction)(
        connection,
        txCreateAccount,
        [authorityKeypair, mintKeypair],
        constants_1.confirmConfig
      );
      (0, chai_1.assert)(
        (yield connection.getTransaction(res, {
          commitment: "confirmed",
        })) != null,
        "create mint account failed"
      );
      let mint = yield (0, spl_token_2.createMint)(
        connection,
        authorityKeypair,
        authorityKeypair.publicKey,
        null, // freez auth
        decimals, //2,
        mintKeypair
      );
      (0,
      chai_1.assert)((yield connection.getAccountInfo(mint)) != null, "create mint failed");
      console.log("mintKeypair.publicKey: ", mintKeypair.publicKey.toBase58());
      return mintKeypair.publicKey;
    } catch (e) {
      console.log(e);
    }
  });
}
exports.createMintWrapper = createMintWrapper;
function createTestAccounts(connection) {
  return __awaiter(this, void 0, void 0, function* () {
    // const connection = new Connection('http://127.0.0.1:8899', 'confirmed');
    let balance = yield connection.getBalance(
      constants_1.ADMIN_AUTH_KEY,
      "confirmed"
    );
    if (balance === 0) {
      let amount = 1000000000000;
      console.time("requestAirdrop");
      let res = yield connection.requestAirdrop(
        constants_1.ADMIN_AUTH_KEY,
        amount
      );
      console.timeEnd("requestAirdrop");
      console.time("confirmAirdrop");
      yield connection.confirmTransaction(res, "confirmed");
      console.timeEnd("confirmAirdrop");
      let Newbalance = yield connection.getBalance(constants_1.ADMIN_AUTH_KEY);
      console.log(res);
      console.log(`${Newbalance} == ${balance + amount}`);
      (0, chai_1.assert)(Newbalance == balance + amount, "airdrop failed");
      // await provider.connection.confirmTransaction(, confirmConfig)
      // subsidising transactions
      let txTransfer1 = new solana.Transaction().add(
        solana.SystemProgram.transfer({
          fromPubkey: constants_1.ADMIN_AUTH_KEYPAIR.publicKey,
          toPubkey: constants_1.AUTHORITY,
          lamports: 1000000000,
        })
      );
      yield (0, web3_js_1.sendAndConfirmTransaction)(
        connection,
        txTransfer1,
        [constants_1.ADMIN_AUTH_KEYPAIR],
        constants_1.confirmConfig
      );
    }
    if (
      (yield connection.getBalance(
        solana.Keypair.fromSecretKey(constants_1.MINT_PRIVATE_KEY).publicKey,
        "confirmed"
      )) == 0
    ) {
      yield createMintWrapper({
        authorityKeypair: constants_1.ADMIN_AUTH_KEYPAIR,
        mintKeypair: web3_js_1.Keypair.fromSecretKey(
          constants_1.MINT_PRIVATE_KEY
        ),
        connection,
      });
      console.log("created mint");
    }
    let balanceUserToken = null;
    try {
      balanceUserToken = yield (0, spl_token_1.getAccount)(
        connection,
        constants_1.userTokenAccount,
        "confirmed",
        spl_token_1.TOKEN_PROGRAM_ID
      );
    } catch (e) {}
    console.log("balanceUserToken ", balanceUserToken);
    try {
      if (balanceUserToken == null) {
        // create associated token account
        yield newAccountWithTokens({
          connection: connection,
          MINT: constants_1.MINT,
          ADMIN_AUTH_KEYPAIR: constants_1.ADMIN_AUTH_KEYPAIR,
          userAccount: constants_1.USER_TOKEN_ACCOUNT,
          amount: 1000000000000,
        });
      }
    } catch (error) {
      console.log(error);
    }
    try {
      if (balanceUserToken == null) {
        // create associated token account
        yield newAccountWithTokens({
          connection: connection,
          MINT: constants_1.MINT,
          ADMIN_AUTH_KEYPAIR: constants_1.ADMIN_AUTH_KEYPAIR,
          userAccount: constants_1.RECIPIENT_TOKEN_ACCOUNT,
          amount: 0,
        });
      }
    } catch (error) {}
    // let merkleTreeConfig = new MerkleTreeConfig({merkleTreePubkey: MERKLE_TREE_KEY,payer: ADMIN_AUTH_KEYPAIR, connection: provider.connection })
    // MERKLE_TREE_AUTHORITY_PDA = await merkleTreeConfig.getMerkleTreeAuthorityPda();
    let POSEIDON = yield circomlibjs.buildPoseidonOpt();
    let KEYPAIR = new web3_js_1.Keypair(POSEIDON, constants_1.KEYPAIR_PRIVKEY);
    let RELAYER_RECIPIENT = new anchor.web3.Account().publicKey;
    return {
      POSEIDON,
      KEYPAIR,
      RELAYER_RECIPIENT,
    };
  });
}
exports.createTestAccounts = createTestAccounts;
