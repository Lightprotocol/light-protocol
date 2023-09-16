"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.getSystem = exports.System = exports.setEnvironment = exports.initLookUpTable = exports.isProgramVerifier = exports.firstLetterToUpper = exports.firstLetterToLower = exports.createAccountObject = exports.sleep = exports.fetchQueuedLeavesAccountInfo = exports.fetchNullifierAccountInfo = exports.getUpdatedSpentUtxos = exports.convertAndComputeDecimals = exports.decimalConversion = exports.strToArr = exports.arrToStr = exports.fetchVerifierByIdLookUp = exports.fetchAssetByIdLookUp = exports.getAssetIndex = exports.getAssetLookUpId = exports.hashAndTruncateToCircuit = void 0;
const tslib_1 = require("tslib");
const anchor_1 = require("@coral-xyz/anchor");
const constants_1 = require("./constants");
const web3_js_1 = require("@solana/web3.js");
const merkleTree_1 = require("./merkleTree");
const constants_system_verifier_1 = require("./test-utils/constants_system_verifier");
const anchor = tslib_1.__importStar(require("@coral-xyz/anchor"));
const errors_1 = require("./errors");
const sha256_1 = require("@noble/hashes/sha256");
const decimal_js_1 = require("decimal.js");
const spl_account_compression_1 = require("@solana/spl-account-compression");
const spl_token_1 = require("@solana/spl-token");
const os = tslib_1.__importStar(require("os"));
const crypto = require("@noble/hashes/crypto");
function hashAndTruncateToCircuit(data) {
    return new anchor_1.BN(sha256_1.sha256.create().update(Buffer.from(data)).digest().slice(1, 32), undefined, "be");
}
exports.hashAndTruncateToCircuit = hashAndTruncateToCircuit;
// TODO: add pooltype
async function getAssetLookUpId({ connection, asset, }) {
    let poolType = new Array(32).fill(0);
    let mtConf = new merkleTree_1.MerkleTreeConfig({
        connection,
    });
    let pubkey = await mtConf.getSplPoolPda(asset, poolType);
    let registeredAssets = await mtConf.merkleTreeProgram.account.registeredAssetPool.fetch(pubkey.pda);
    return registeredAssets.index;
}
exports.getAssetLookUpId = getAssetLookUpId;
function getAssetIndex(assetPubkey, assetLookupTable) {
    return new anchor_1.BN(assetLookupTable.indexOf(assetPubkey.toBase58()));
}
exports.getAssetIndex = getAssetIndex;
function fetchAssetByIdLookUp(assetIndex, assetLookupTable) {
    return new web3_js_1.PublicKey(assetLookupTable[assetIndex.toNumber()]);
}
exports.fetchAssetByIdLookUp = fetchAssetByIdLookUp;
function fetchVerifierByIdLookUp(index, verifierProgramLookupTable) {
    return new web3_js_1.PublicKey(verifierProgramLookupTable[index.toNumber()]);
}
exports.fetchVerifierByIdLookUp = fetchVerifierByIdLookUp;
const arrToStr = (uint8arr) => "LPx" + Buffer.from(uint8arr.buffer).toString("hex");
exports.arrToStr = arrToStr;
const strToArr = (str) => new Uint8Array(Buffer.from(str.slice(3), "hex"));
exports.strToArr = strToArr;
function decimalConversion({ tokenCtx, skipDecimalConversions, publicAmountSpl, publicAmountSol, minimumLamports, minimumLamportsAmount, }) {
    if (!skipDecimalConversions) {
        publicAmountSpl = publicAmountSpl
            ? (0, exports.convertAndComputeDecimals)(publicAmountSpl, tokenCtx.decimals)
            : undefined;
        // If SOL amount is not provided, the default value is either minimum amount (if defined) or 0.
        publicAmountSol = publicAmountSol
            ? (0, exports.convertAndComputeDecimals)(publicAmountSol, new anchor_1.BN(1e9))
            : minimumLamports
                ? minimumLamportsAmount
                : constants_1.BN_0;
    }
    else {
        publicAmountSpl = publicAmountSpl ? new anchor_1.BN(publicAmountSpl) : undefined;
        publicAmountSol = publicAmountSol ? new anchor_1.BN(publicAmountSol) : constants_1.BN_0;
    }
    return { publicAmountSpl, publicAmountSol };
}
exports.decimalConversion = decimalConversion;
const convertAndComputeDecimals = (amount, decimals) => {
    if (typeof amount === "number" && amount < 0) {
        throw new Error("Negative amounts are not allowed.");
    }
    if (typeof amount === "string" && amount.startsWith("-")) {
        throw new Error("Negative amounts are not allowed.");
    }
    if (decimals.lt(constants_1.BN_1)) {
        throw new Error("Decimal numbers have to be at least 1 since we precompute 10**decimalValue.");
    }
    let amountStr = amount.toString();
    if (!new decimal_js_1.Decimal(amountStr).isInt()) {
        const convertedFloat = new decimal_js_1.Decimal(amountStr).times(new decimal_js_1.Decimal(decimals.toString()));
        if (!convertedFloat.isInt())
            throw new Error(`Decimal conversion of value ${amountStr} failed`);
        return new anchor_1.BN(convertedFloat.toString());
    }
    const bnAmount = new anchor_1.BN(amountStr);
    return bnAmount.mul(decimals);
};
exports.convertAndComputeDecimals = convertAndComputeDecimals;
const getUpdatedSpentUtxos = (tokenBalances) => {
    return Array.from(tokenBalances.values())
        .map((value) => Array.from(value.spentUtxos.values()))
        .flat();
};
exports.getUpdatedSpentUtxos = getUpdatedSpentUtxos;
const fetchNullifierAccountInfo = async (nullifier, connection) => {
    const nullifierPubkey = web3_js_1.PublicKey.findProgramAddressSync([
        new anchor.BN(nullifier.toString()).toArrayLike(Buffer, "be", 32),
        anchor.utils.bytes.utf8.encode("nf"),
    ], constants_1.merkleTreeProgramId)[0];
    var retries = 2;
    while (retries > 0) {
        const res = await connection.getAccountInfo(nullifierPubkey, "processed");
        if (res)
            return res;
        retries--;
    }
    return connection.getAccountInfo(nullifierPubkey, "processed");
};
exports.fetchNullifierAccountInfo = fetchNullifierAccountInfo;
// use
const fetchQueuedLeavesAccountInfo = async (leftLeaf, connection) => {
    const queuedLeavesPubkey = web3_js_1.PublicKey.findProgramAddressSync([leftLeaf, anchor.utils.bytes.utf8.encode("leaves")], constants_1.merkleTreeProgramId)[0];
    return connection.getAccountInfo(queuedLeavesPubkey, "confirmed");
};
exports.fetchQueuedLeavesAccountInfo = fetchQueuedLeavesAccountInfo;
const sleep = (ms) => {
    return new Promise((resolve) => setTimeout(resolve, ms));
};
exports.sleep = sleep;
/**
 * @description Creates an object of a type defined in accounts[accountName],
 * @description all properties need to be part of obj, if a property is missing an error is thrown.
 * @description The accounts array is part of an anchor idl.
 * @param obj Object properties are picked from.
 * @param accounts Idl accounts array from which accountName is selected.
 * @param accountName Defines which account in accounts to use as type for the output object.
 * @returns
 */
function createAccountObject(obj, accounts, accountName) {
    const account = accounts.find((account) => account.name === accountName);
    if (!account) {
        throw new errors_1.UtilsError(errors_1.UtilsErrorCode.ACCOUNT_NAME_UNDEFINED_IN_IDL, "pickFieldsFromObject", `${accountName} does not exist in idl`);
    }
    const fieldNames = account.type.fields.map((field) => field.name);
    let accountObject = {};
    fieldNames.forEach((fieldName) => {
        accountObject[fieldName] = obj[fieldName];
        if (!accountObject[fieldName])
            throw new errors_1.UtilsError(errors_1.UtilsErrorCode.PROPERTY_UNDEFINED, "pickFieldsFromObject", `Property ${fieldName.toString()} undefined`);
    });
    return accountObject;
}
exports.createAccountObject = createAccountObject;
function firstLetterToLower(input) {
    if (!input)
        return input;
    return input.charAt(0).toLowerCase() + input.slice(1);
}
exports.firstLetterToLower = firstLetterToLower;
function firstLetterToUpper(input) {
    if (!input)
        return input;
    return input.charAt(0).toUpperCase() + input.slice(1);
}
exports.firstLetterToUpper = firstLetterToUpper;
/**
 * This function checks if an account in the provided idk object exists with a name
 * ending with 'PublicInputs' and contains a field named 'publicAppVerifier'.
 *
 * @param {Idl} idl - The IDL object to check.
 * @returns {boolean} - Returns true if such an account exists, false otherwise.
 */
function isProgramVerifier(idl) {
    if (!idl.accounts)
        throw new Error("Idl does not contain accounts");
    return idl.accounts.some((account) => account.name.endsWith("PublicInputs") &&
        account.type.fields.some((field) => field.name === "publicAppVerifier"));
}
exports.isProgramVerifier = isProgramVerifier;
async function initLookUpTable(payer, provider, extraAccounts) {
    const payerPubkey = payer.publicKey;
    const recentSlot = (await provider.connection.getSlot("confirmed")) - 10;
    var [lookUpTable] = web3_js_1.PublicKey.findProgramAddressSync([
        payerPubkey.toBuffer(),
        new anchor.BN(recentSlot).toArrayLike(Buffer, "le", 8),
    ], web3_js_1.AddressLookupTableProgram.programId);
    const createInstruction = web3_js_1.AddressLookupTableProgram.createLookupTable({
        authority: payerPubkey,
        payer: payerPubkey,
        recentSlot,
    })[0];
    let escrows = web3_js_1.PublicKey.findProgramAddressSync([anchor.utils.bytes.utf8.encode("escrow")], constants_1.verifierProgramZeroProgramId)[0];
    var transaction = new web3_js_1.Transaction().add(createInstruction);
    const addressesToAdd = [
        web3_js_1.SystemProgram.programId,
        constants_1.merkleTreeProgramId,
        constants_1.DEFAULT_PROGRAMS.rent,
        spl_account_compression_1.SPL_NOOP_PROGRAM_ID,
        constants_1.MERKLE_TREE_AUTHORITY_PDA,
        merkleTree_1.MerkleTreeConfig.getEventMerkleTreePda(),
        merkleTree_1.MerkleTreeConfig.getTransactionMerkleTreePda(),
        merkleTree_1.MerkleTreeConfig.getTransactionMerkleTreePda(new anchor.BN(1)),
        merkleTree_1.MerkleTreeConfig.getTransactionMerkleTreePda(new anchor.BN(2)),
        constants_1.PRE_INSERTED_LEAVES_INDEX,
        constants_1.AUTHORITY,
        spl_token_1.TOKEN_PROGRAM_ID,
        escrows,
        constants_1.TOKEN_AUTHORITY,
        constants_1.REGISTERED_POOL_PDA_SOL,
        constants_1.REGISTERED_POOL_PDA_SPL_TOKEN,
        constants_1.verifierProgramTwoProgramId,
        constants_1.REGISTERED_VERIFIER_ONE_PDA,
        constants_1.REGISTERED_VERIFIER_PDA,
        constants_1.REGISTERED_VERIFIER_TWO_PDA,
        constants_system_verifier_1.MINT,
    ];
    if (extraAccounts) {
        for (var i in extraAccounts) {
            addressesToAdd.push(extraAccounts[i]);
        }
    }
    const extendInstruction = web3_js_1.AddressLookupTableProgram.extendLookupTable({
        lookupTable: lookUpTable,
        authority: payerPubkey,
        payer: payerPubkey,
        addresses: addressesToAdd,
    });
    transaction.add(extendInstruction);
    let recentBlockhash = await provider.connection.getLatestBlockhash("confirmed");
    transaction.feePayer = payerPubkey;
    transaction.recentBlockhash = recentBlockhash.blockhash;
    try {
        await payer.sendAndConfirmTransaction(transaction);
    }
    catch (e) {
        console.log("e : ", e);
        console.log("payerPubkey : ", payerPubkey.toBase58());
        console.log("transaction : ", JSON.stringify(transaction));
        throw new Error(`Creating lookup table failed payer: ${payerPubkey}`);
    }
    let lookupTableAccount = await provider.connection.getAccountInfo(lookUpTable, "confirmed");
    if (lookupTableAccount == null)
        throw new Error(`Creating lookup table failed payer: ${payerPubkey}`);
    return lookUpTable;
}
exports.initLookUpTable = initLookUpTable;
// setting environment correctly for ethereum-crypto
function setEnvironment() {
    if (typeof process !== "undefined" &&
        process.versions != null &&
        process.versions.node != null) {
        crypto.node = require("crypto");
    }
    else {
        crypto.web = window.crypto;
    }
}
exports.setEnvironment = setEnvironment;
var System;
(function (System) {
    System[System["MacOsAmd64"] = 0] = "MacOsAmd64";
    System[System["MacOsArm64"] = 1] = "MacOsArm64";
    System[System["LinuxAmd64"] = 2] = "LinuxAmd64";
    System[System["LinuxArm64"] = 3] = "LinuxArm64";
})(System = exports.System || (exports.System = {}));
function getSystem() {
    const arch = os.arch();
    const platform = os.platform();
    switch (platform) {
        case "darwin":
            switch (arch) {
                case "x64":
                    return System.MacOsAmd64;
                case "arm":
                // fallthrough
                case "arm64":
                    return System.MacOsArm64;
                default:
                    throw new Error(`Architecture ${arch} is not supported.`);
            }
        case "linux":
            switch (arch) {
                case "x64":
                    return System.LinuxAmd64;
                case "arm":
                // fallthrough
                case "arm64":
                    return System.LinuxArm64;
                default:
                    throw new Error(`Architecture ${arch} is not supported.`);
            }
    }
    throw new Error(`Platform ${platform} is not supported.`);
}
exports.getSystem = getSystem;
//# sourceMappingURL=utils.js.map