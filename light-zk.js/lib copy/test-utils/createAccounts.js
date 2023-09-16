"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.createTestAccounts = exports.createMintWrapper = exports.newAccountWithTokens = exports.newProgramOwnedAccount = exports.newAddressWithLamports = exports.newAccountWithLamports = void 0;
const tslib_1 = require("tslib");
const solana = require("@solana/web3.js");
const anchor = tslib_1.__importStar(require("@coral-xyz/anchor"));
const anchor_1 = require("@coral-xyz/anchor");
const web3_js_1 = require("@solana/web3.js");
const { SystemProgram } = require("@solana/web3.js");
// const token = require('@solana/spl-token')
// @ts-ignore
var _ = require("lodash");
const spl_token_1 = require("@solana/spl-token");
const account_1 = require("../account");
const spl_token_2 = require("@solana/spl-token");
const index_1 = require("../index");
const chai_1 = require("chai");
let circomlibjs = require("circomlibjs");
// TODO: check whether we need all of these functions
const newAccountWithLamports = async (connection, account = web3_js_1.Keypair.generate(), lamports = 1e10) => {
    const signature = await connection.requestAirdrop(account.publicKey, lamports);
    await (0, index_1.confirmTransaction)(connection, signature);
    console.log("newAccountWithLamports ", account.publicKey.toBase58());
    return account;
};
exports.newAccountWithLamports = newAccountWithLamports;
const newAddressWithLamports = async (connection, address = new anchor.web3.Account().publicKey, lamports = 1e11) => {
    let retries = 30;
    await connection.requestAirdrop(address, lamports);
    for (;;) {
        await (0, index_1.sleep)(500);
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
exports.newAddressWithLamports = newAddressWithLamports;
const newProgramOwnedAccount = async ({ connection, owner, }) => {
    let account = new anchor.web3.Account();
    let payer = new anchor.web3.Account();
    let retry = 0;
    while (retry < 30) {
        try {
            let signature = await connection.requestAirdrop(payer.publicKey, 1e7);
            await (0, index_1.confirmTransaction)(connection, signature);
            const tx = new solana.Transaction().add(solana.SystemProgram.createAccount({
                fromPubkey: payer.publicKey,
                newAccountPubkey: account.publicKey,
                space: 0,
                lamports: await connection.getMinimumBalanceForRentExemption(1),
                Id: owner.programId,
            }));
            tx.feePayer = payer.publicKey;
            tx.recentBlockhash = await connection.getLatestBlockhash();
            await solana.sendAndConfirmTransaction(connection, tx, [payer, account], {
                commitment: "confirmed",
                preflightCommitment: "confirmed",
            });
            return account;
        }
        catch { }
        retry++;
    }
    throw "Can't create program account with lamports";
};
exports.newProgramOwnedAccount = newProgramOwnedAccount;
// FIXME: doesn't need a keypair for userAccount...
async function newAccountWithTokens({ connection, MINT, ADMIN_AUTH_KEYPAIR, userAccount, amount, }) {
    let tokenAccount = await (0, spl_token_1.createAccount)(connection, ADMIN_AUTH_KEYPAIR, MINT, userAccount.publicKey);
    try {
        await (0, spl_token_1.mintTo)(connection, ADMIN_AUTH_KEYPAIR, MINT, tokenAccount, ADMIN_AUTH_KEYPAIR.publicKey, amount.toNumber(), []);
        //FIXME: remove this
    }
    catch (e) {
        console.log("mintTo error", e);
        await (0, spl_token_1.mintTo)(connection, ADMIN_AUTH_KEYPAIR, MINT, tokenAccount, ADMIN_AUTH_KEYPAIR.publicKey, 
        //@ts-ignore
        amount, []);
    }
    return tokenAccount;
}
exports.newAccountWithTokens = newAccountWithTokens;
async function createMintWrapper({ authorityKeypair, mintKeypair = new web3_js_1.Keypair(), nft = false, decimals = 2, connection, }) {
    if (nft == true) {
        decimals = 0;
    }
    try {
        let space = spl_token_1.MINT_SIZE;
        let txCreateAccount = new solana.Transaction().add(SystemProgram.createAccount({
            fromPubkey: authorityKeypair.publicKey,
            lamports: connection.getMinimumBalanceForRentExemption(space),
            newAccountPubkey: mintKeypair.publicKey,
            programId: spl_token_1.TOKEN_PROGRAM_ID,
            space: space,
        }));
        let res = await (0, web3_js_1.sendAndConfirmTransaction)(connection, txCreateAccount, [authorityKeypair, mintKeypair], index_1.confirmConfig);
        (0, chai_1.assert)((await connection.getTransaction(res, {
            commitment: "confirmed",
        })) != null, "create mint account failed");
        let mint = await (0, spl_token_2.createMint)(connection, authorityKeypair, authorityKeypair.publicKey, null, // freez auth
        decimals, //2,
        mintKeypair);
        (0, chai_1.assert)((await connection.getAccountInfo(mint)) != null, "create mint failed");
        return mintKeypair.publicKey;
    }
    catch (e) {
        console.log(e);
    }
}
exports.createMintWrapper = createMintWrapper;
async function createTestAccounts(connection, userTokenAccount) {
    // const connection = new Connection('http://127.0.0.1:8899', 'confirmed');
    let balance = await connection.getBalance(index_1.ADMIN_AUTH_KEY, "confirmed");
    if (balance === 0) {
        let amount = 1000000000000;
        let signature = await connection.requestAirdrop(index_1.ADMIN_AUTH_KEY, amount);
        await (0, index_1.confirmTransaction)(connection, signature);
        let Newbalance = await connection.getBalance(index_1.ADMIN_AUTH_KEY);
        (0, chai_1.assert)(Newbalance == balance + amount, "airdrop failed");
        let signature2 = await connection.requestAirdrop(index_1.AUTHORITY_ONE, amount);
        await (0, index_1.confirmTransaction)(connection, signature2);
        // subsidising transactions
        let txTransfer1 = new solana.Transaction().add(solana.SystemProgram.transfer({
            fromPubkey: index_1.ADMIN_AUTH_KEYPAIR.publicKey,
            toPubkey: index_1.AUTHORITY,
            lamports: 3000000000,
        }));
        await (0, web3_js_1.sendAndConfirmTransaction)(connection, txTransfer1, [index_1.ADMIN_AUTH_KEYPAIR], index_1.confirmConfig);
    }
    if ((await connection.getBalance(web3_js_1.Keypair.fromSecretKey(index_1.MINT_PRIVATE_KEY).publicKey, "confirmed")) == 0) {
        await createMintWrapper({
            authorityKeypair: index_1.ADMIN_AUTH_KEYPAIR,
            mintKeypair: web3_js_1.Keypair.fromSecretKey(index_1.MINT_PRIVATE_KEY),
            connection,
        });
        console.log("created mint ", web3_js_1.Keypair.fromSecretKey(index_1.MINT_PRIVATE_KEY).publicKey.toBase58());
    }
    let balanceUserToken = null;
    let userSplAccount = null;
    try {
        let tokenCtx = index_1.TOKEN_REGISTRY.get("USDC");
        if (userTokenAccount) {
            userSplAccount = userTokenAccount;
        }
        else {
            userSplAccount = (0, spl_token_1.getAssociatedTokenAddressSync)(tokenCtx.mint, index_1.ADMIN_AUTH_KEYPAIR.publicKey);
        }
        console.log("test setup: admin spl acc", userSplAccount.toBase58(), userTokenAccount === null || userTokenAccount === void 0 ? void 0 : userTokenAccount.toBase58());
        balanceUserToken = await (0, spl_token_1.getAccount)(connection, userSplAccount, //userTokenAccount,
        "confirmed", spl_token_1.TOKEN_PROGRAM_ID);
    }
    catch (e) { }
    try {
        if (balanceUserToken == null) {
            // create associated token account
            await newAccountWithTokens({
                connection: connection,
                MINT: index_1.MINT,
                ADMIN_AUTH_KEYPAIR: index_1.ADMIN_AUTH_KEYPAIR,
                userAccount: userTokenAccount ? index_1.USER_TOKEN_ACCOUNT : index_1.ADMIN_AUTH_KEYPAIR,
                amount: new anchor_1.BN(1000000000000),
            });
        }
    }
    catch (error) {
        console.log(error);
    }
    console.log("userSplAccount ", userSplAccount === null || userSplAccount === void 0 ? void 0 : userSplAccount.toBase58());
    try {
        if (balanceUserToken == null) {
            // create associated token account
            await newAccountWithTokens({
                connection: connection,
                MINT: index_1.MINT,
                ADMIN_AUTH_KEYPAIR: index_1.ADMIN_AUTH_KEYPAIR,
                userAccount: index_1.RECIPIENT_TOKEN_ACCOUNT,
                amount: index_1.BN_0,
            });
        }
    }
    catch (error) { }
    let POSEIDON = await circomlibjs.buildPoseidonOpt();
    let KEYPAIR = new account_1.Account({
        poseidon: POSEIDON,
        seed: index_1.KEYPAIR_PRIVKEY.toString(),
    });
    let RELAYER_RECIPIENT = new anchor.web3.Account().publicKey;
    return {
        POSEIDON,
        KEYPAIR,
        RELAYER_RECIPIENT,
    };
}
exports.createTestAccounts = createTestAccounts;
//# sourceMappingURL=createAccounts.js.map