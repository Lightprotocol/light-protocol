"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || function (mod) {
    if (mod && mod.__esModule) return mod;
    var result = {};
    if (mod != null) for (var k in mod) if (k !== "default" && Object.prototype.hasOwnProperty.call(mod, k)) __createBinding(result, mod, k);
    __setModuleDefault(result, mod);
    return result;
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.initLookUpTableTest = exports.initLookUpTableFromFile = void 0;
const anchor_1 = require("@coral-xyz/anchor");
const web3_js_1 = require("@solana/web3.js");
const anchor = __importStar(require("@coral-xyz/anchor"));
const spl_token_1 = require("@solana/spl-token");
const chai_1 = require("chai");
const fs_1 = require("fs");
const index_1 = require("../index");
const index_2 = require("../idls/index");
const spl_account_compression_1 = require("@solana/spl-account-compression");
// TODO: create cli function to create a lookup table for apps
// Probably only works for testing
async function initLookUpTableFromFile(provider, path = `lookUpTable.txt`, extraAccounts) {
    const recentSlot = (await provider.connection.getSlot("confirmed")) - 10;
    const payerPubkey = index_1.ADMIN_AUTH_KEYPAIR.publicKey;
    var [lookUpTable] = web3_js_1.PublicKey.findProgramAddressSync([
        payerPubkey.toBuffer(),
        new anchor.BN(recentSlot).toArrayLike(Buffer, "le", 8),
    ], web3_js_1.AddressLookupTableProgram.programId);
    try {
        let lookUpTableRead = new web3_js_1.PublicKey((0, fs_1.readFileSync)(path, "utf8"));
        let lookUpTableInfoInit = await provider.connection.getAccountInfo(lookUpTableRead);
        if (lookUpTableInfoInit) {
            lookUpTable = lookUpTableRead;
        }
    }
    catch (e) { }
    let LOOK_UP_TABLE = await initLookUpTableTest(provider, lookUpTable, recentSlot, extraAccounts);
    (0, fs_1.writeFile)(path, LOOK_UP_TABLE.toString(), function (err) {
        if (err) {
            return console.error(err);
        }
    });
    return LOOK_UP_TABLE; //new Promise((resolveOuter) => {LOOK_UP_TABLE});
}
exports.initLookUpTableFromFile = initLookUpTableFromFile;
async function initLookUpTableTest(provider, lookupTableAddress, recentSlot, extraAccounts = []) {
    var lookUpTableInfoInit = null;
    if (lookupTableAddress != undefined) {
        lookUpTableInfoInit = await provider.connection.getAccountInfo(lookupTableAddress);
    }
    if (lookUpTableInfoInit == null) {
        console.log("recentSlot: ", recentSlot);
        const payerPubkey = index_1.ADMIN_AUTH_KEYPAIR.publicKey;
        const createInstruction = web3_js_1.AddressLookupTableProgram.createLookupTable({
            authority: payerPubkey,
            payer: payerPubkey,
            recentSlot,
        })[0];
        const verifierProgramZero = new anchor_1.Program(index_2.IDL_VERIFIER_PROGRAM_ZERO, index_1.verifierProgramZeroProgramId);
        let escrows = (await web3_js_1.PublicKey.findProgramAddress([anchor.utils.bytes.utf8.encode("escrow")], verifierProgramZero.programId))[0];
        let ix0 = web3_js_1.SystemProgram.transfer({
            fromPubkey: index_1.ADMIN_AUTH_KEYPAIR.publicKey,
            toPubkey: index_1.AUTHORITY,
            lamports: 10000000000,
        });
        var transaction = new web3_js_1.Transaction().add(createInstruction);
        const addressesToAdd = [
            web3_js_1.SystemProgram.programId,
            index_1.merkleTreeProgramId,
            index_1.DEFAULT_PROGRAMS.rent,
            spl_account_compression_1.SPL_NOOP_PROGRAM_ID,
            index_1.MerkleTreeConfig.getEventMerkleTreePda(),
            index_1.MerkleTreeConfig.getTransactionMerkleTreePda(),
            index_1.PRE_INSERTED_LEAVES_INDEX,
            index_1.AUTHORITY,
            spl_token_1.TOKEN_PROGRAM_ID,
            escrows,
        ];
        const additonalAccounts = [
            index_1.TOKEN_AUTHORITY,
            index_1.REGISTERED_POOL_PDA_SOL,
            index_1.REGISTERED_POOL_PDA_SPL_TOKEN,
            index_1.verifierProgramTwoProgramId,
            index_1.REGISTERED_VERIFIER_ONE_PDA,
            index_1.REGISTERED_VERIFIER_PDA,
            index_1.REGISTERED_VERIFIER_TWO_PDA,
            index_1.MINT,
        ];
        extraAccounts = extraAccounts.concat(additonalAccounts);
        if (extraAccounts) {
            for (var i in extraAccounts) {
                addressesToAdd.push(extraAccounts[i]);
            }
        }
        const extendInstruction = web3_js_1.AddressLookupTableProgram.extendLookupTable({
            lookupTable: lookupTableAddress,
            authority: payerPubkey,
            payer: payerPubkey,
            addresses: addressesToAdd,
        });
        transaction.add(extendInstruction);
        transaction.add(ix0);
        let recentBlockhash = await provider.connection.getLatestBlockhash("confirmed");
        transaction.feePayer = payerPubkey;
        transaction.recentBlockhash = recentBlockhash.blockhash;
        try {
            await (0, web3_js_1.sendAndConfirmTransaction)(provider.connection, transaction, [index_1.ADMIN_AUTH_KEYPAIR], index_1.confirmConfig);
        }
        catch (e) {
            console.log("e : ", e);
        }
        let lookupTableAccount = await provider.connection.getAccountInfo(lookupTableAddress, "confirmed");
        (0, chai_1.assert)(lookupTableAccount != null);
    }
    return lookupTableAddress;
}
exports.initLookUpTableTest = initLookUpTableTest;
//# sourceMappingURL=initLookUpTable.js.map