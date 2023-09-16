"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.validateUtxoAmounts = exports.createRecipientUtxos = exports.createOutUtxos = exports.getRecipientsAmount = exports.getUtxoArrayAmount = void 0;
const web3_js_1 = require("@solana/web3.js");
const anchor_1 = require("@coral-xyz/anchor");
const index_1 = require("../index");
// mint: PublicKey, expectedAmount: BN,
const getUtxoArrayAmount = (mint, inUtxos) => {
    let inAmount = index_1.BN_0;
    inUtxos.forEach((inUtxo) => {
        inUtxo.assets.forEach((asset, i) => {
            if (asset.toBase58() === mint.toBase58()) {
                inAmount = inAmount.add(inUtxo.amounts[i]);
            }
        });
    });
    return inAmount;
};
exports.getUtxoArrayAmount = getUtxoArrayAmount;
const getRecipientsAmount = (mint, recipients) => {
    if (mint.toBase58() === web3_js_1.SystemProgram.programId.toBase58()) {
        return recipients.reduce((sum, recipient) => sum.add(recipient.solAmount), index_1.BN_0);
    }
    else {
        return recipients.reduce((sum, recipient) => recipient.mint.toBase58() === mint.toBase58()
            ? sum.add(recipient.splAmount)
            : sum.add(index_1.BN_0), index_1.BN_0);
    }
};
exports.getRecipientsAmount = getRecipientsAmount;
// --------------------------------------------------------------------------
// Algorithm:
// check nr recipients is leq to nrOuts of verifier
// check that publicMint and recipientMints exist in inputUtxos
// checks sum inAmounts for every asset are less or equal to sum OutAmounts
// unshield
// publicSol -sumSolAmount
// publicSpl -sumSplAmount
// publicMint
// transfer
// check no publics
// shield
// sumInSol +sumSolAmount
// sumInSpl +sumSplAmount
// publicMint
// create via recipients requested utxos and subtract amounts from sums
// beforeEach utxo check that no amount is negative
// create change utxos with remaining spl balances and sol balance
// --------------------------------------------------------------------------
function createOutUtxos({ poseidon, inUtxos, outUtxos = [], publicMint, publicAmountSpl, publicAmountSol, relayerFee, changeUtxoAccount, action, appUtxo, numberMaxOutUtxos, assetLookupTable, verifierProgramLookupTable, separateSolUtxo = false, }) {
    var _a, _b;
    if (!poseidon)
        throw new index_1.CreateUtxoError(index_1.TransactionParametersErrorCode.NO_POSEIDON_HASHER_PROVIDED, "createOutUtxos", "Poseidon not initialized");
    if (relayerFee) {
        publicAmountSol = publicAmountSol
            ? publicAmountSol.add(relayerFee)
            : relayerFee;
    }
    const assetPubkeys = !inUtxos && action === index_1.Action.SHIELD
        ? [
            web3_js_1.SystemProgram.programId,
            publicMint ? publicMint : web3_js_1.SystemProgram.programId,
        ]
        : index_1.TransactionParameters.getAssetPubkeys(inUtxos).assetPubkeys;
    if (!assetPubkeys)
        throw new index_1.CreateUtxoError(index_1.TransactionErrorCode.ASSET_PUBKEYS_UNDEFINED, "constructor");
    // TODO: enable perfect manual amounts of amounts to recipients
    // check nr outUtxos is leq to nrOuts of verifier
    if (outUtxos.length > numberMaxOutUtxos - 1) {
        throw new index_1.CreateUtxoError(index_1.CreateUtxoErrorCode.INVALID_NUMER_OF_RECIPIENTS, "createOutUtxos", `Number of recipients greater than allowed: ${outUtxos.length} allowed ${1}`);
    }
    outUtxos.map((outUtxo) => {
        if (!assetPubkeys.find((x) => { var _a; return x.toBase58() === ((_a = outUtxo.assets[1]) === null || _a === void 0 ? void 0 : _a.toBase58()); })) {
            throw new index_1.CreateUtxoError(index_1.CreateUtxoErrorCode.INVALID_RECIPIENT_MINT, "createOutUtxos", `Mint ${outUtxo.assets[1]} does not exist in input utxos mints ${assetPubkeys}`);
        }
    });
    // add public mint if it does not exist in inUtxos
    if (publicMint &&
        !assetPubkeys.find((x) => x.toBase58() === publicMint.toBase58())) {
        assetPubkeys.push(publicMint);
    }
    let assets = validateUtxoAmounts({
        assetPubkeys,
        inUtxos,
        outUtxos,
        publicAmountSol,
        publicAmountSpl,
        action,
    });
    let publicSolAssetIndex = assets.findIndex((x) => x.asset.toBase58() === web3_js_1.SystemProgram.programId.toBase58());
    // remove duplicates
    const key = "asset";
    assets = [...new Map(assets.map((item) => [item[key], item])).values()];
    // subtract public amounts from sumIns
    if (action === index_1.Action.UNSHIELD) {
        if (!publicAmountSol && !publicAmountSpl)
            throw new index_1.CreateUtxoError(index_1.CreateUtxoErrorCode.NO_PUBLIC_AMOUNTS_PROVIDED, "createOutUtxos", "publicAmountSol not initialized for unshield");
        if (!publicAmountSpl)
            publicAmountSpl = index_1.BN_0;
        if (!publicAmountSol)
            throw new index_1.CreateUtxoError(index_1.CreateUtxoErrorCode.PUBLIC_SOL_AMOUNT_UNDEFINED, "constructor");
        if (publicAmountSpl && !publicMint)
            throw new index_1.CreateUtxoError(index_1.CreateUtxoErrorCode.NO_PUBLIC_MINT_PROVIDED, "createOutUtxos", "publicMint not initialized for unshield");
        let publicSplAssetIndex = assets.findIndex((x) => x.asset.toBase58() === (publicMint === null || publicMint === void 0 ? void 0 : publicMint.toBase58()));
        assets[publicSplAssetIndex].sumIn =
            assets[publicSplAssetIndex].sumIn.sub(publicAmountSpl);
        assets[publicSolAssetIndex].sumIn =
            assets[publicSolAssetIndex].sumIn.sub(publicAmountSol);
        // add public amounts to sumIns
    }
    else if (action === index_1.Action.SHIELD) {
        if (relayerFee)
            throw new index_1.CreateUtxoError(index_1.CreateUtxoErrorCode.RELAYER_FEE_DEFINED, "createOutUtxos", "Shield and relayer fee defined");
        if (!publicAmountSpl)
            publicAmountSpl = index_1.BN_0;
        if (!publicAmountSol)
            publicAmountSol = index_1.BN_0;
        let publicSplAssetIndex = assets.findIndex((x) => x.asset.toBase58() === (publicMint === null || publicMint === void 0 ? void 0 : publicMint.toBase58()));
        let publicSolAssetIndex = assets.findIndex((x) => x.asset.toBase58() === web3_js_1.SystemProgram.programId.toBase58());
        assets[publicSplAssetIndex].sumIn =
            assets[publicSplAssetIndex].sumIn.add(publicAmountSpl);
        assets[publicSolAssetIndex].sumIn =
            assets[publicSolAssetIndex].sumIn.add(publicAmountSol);
    }
    else if (action === index_1.Action.TRANSFER) {
        if (!publicAmountSol)
            throw new index_1.CreateUtxoError(index_1.CreateUtxoErrorCode.PUBLIC_SOL_AMOUNT_UNDEFINED, "constructor");
        let publicSolAssetIndex = assets.findIndex((x) => x.asset.toBase58() === web3_js_1.SystemProgram.programId.toBase58());
        assets[publicSolAssetIndex].sumIn =
            assets[publicSolAssetIndex].sumIn.sub(publicAmountSol);
    }
    var outputUtxos = [...outUtxos];
    // create recipient output utxos, one for each defined recipient
    for (var j in outUtxos) {
        if (outUtxos[j].assets[1] && !outUtxos[j].amounts[1]) {
            throw new index_1.CreateUtxoError(index_1.CreateUtxoErrorCode.SPL_AMOUNT_UNDEFINED, "createOutUtxos", `Mint defined while splAmount is undefinedfor recipient ${outUtxos[j]}`);
        }
        let solAmount = outUtxos[j].amounts[0] ? outUtxos[j].amounts[0] : index_1.BN_0;
        let splAmount = outUtxos[j].amounts[1] ? outUtxos[j].amounts[1] : index_1.BN_0;
        let splMint = outUtxos[j].assets[1]
            ? outUtxos[j].assets[1]
            : web3_js_1.SystemProgram.programId;
        let publicSplAssetIndex = assets.findIndex((x) => x.asset.toBase58() === (splMint === null || splMint === void 0 ? void 0 : splMint.toBase58()));
        assets[publicSplAssetIndex].sumIn = assets[publicSplAssetIndex].sumIn
            .sub(splAmount)
            .clone();
        assets[publicSolAssetIndex].sumIn = assets[publicSolAssetIndex].sumIn
            .sub(solAmount)
            .clone();
    }
    // create change utxo
    // Also handles case that we have more than one change utxo because we wanted to withdraw sol and used utxos with different spl tokens
    // it creates a change utxo for every asset that is non-zero then check that number of utxos is less or equal to verifier.config.outs
    let publicSplAssets = assets.filter((x) => x.sumIn.toString() !== "0" &&
        x.asset.toBase58() !== web3_js_1.SystemProgram.programId.toBase58());
    let nrOutUtxos = publicSplAssets.length ? publicSplAssets.length : 1;
    if (separateSolUtxo && publicSplAssets.length > 0) {
        // nrOutUtxos -= 1;
        /**
         * Problem:
         * - we want to keep the majority of sol holdings in a single sol utxo, but we want to keep a small amount of sol in every spl utxo as well
         * - for example when merging incoming spl utxos we might have no sol in any of these utxos to pay the relayer
         *   -> we need an existing sol utxo but we don't want to merge it into the spl utxos
         * - sol amount should leave a minimum amount in spl utxos if possible
         */
        const preliminarySolAmount = assets[publicSolAssetIndex].sumIn.sub(index_1.MINIMUM_LAMPORTS.mul(index_1.BN_2));
        const solAmount = preliminarySolAmount.isNeg()
            ? assets[publicSolAssetIndex].sumIn
            : preliminarySolAmount;
        assets[publicSolAssetIndex].sumIn =
            assets[publicSolAssetIndex].sumIn.sub(solAmount);
        let solChangeUtxo = new index_1.Utxo({
            poseidon,
            assets: [web3_js_1.SystemProgram.programId],
            amounts: [solAmount],
            account: changeUtxoAccount,
            appData: appUtxo === null || appUtxo === void 0 ? void 0 : appUtxo.appData,
            appDataHash: appUtxo === null || appUtxo === void 0 ? void 0 : appUtxo.appDataHash,
            includeAppData: appUtxo === null || appUtxo === void 0 ? void 0 : appUtxo.includeAppData,
            verifierAddress: appUtxo === null || appUtxo === void 0 ? void 0 : appUtxo.verifierAddress,
            assetLookupTable,
            verifierProgramLookupTable,
        });
        outputUtxos.push(solChangeUtxo);
    }
    for (var x = 0; x < nrOutUtxos; x++) {
        let solAmount = index_1.BN_0;
        if (x == 0) {
            solAmount = assets[publicSolAssetIndex].sumIn;
        }
        // catch case of sol deposit with undefined spl assets
        let splAmount = ((_a = publicSplAssets[x]) === null || _a === void 0 ? void 0 : _a.sumIn) ? publicSplAssets[x].sumIn : index_1.BN_0;
        let splAsset = ((_b = publicSplAssets[x]) === null || _b === void 0 ? void 0 : _b.asset)
            ? publicSplAssets[x].asset
            : web3_js_1.SystemProgram.programId;
        if (solAmount.isZero() && splAmount.isZero())
            continue;
        let changeUtxo = new index_1.Utxo({
            poseidon,
            assets: [web3_js_1.SystemProgram.programId, splAsset],
            amounts: [solAmount, splAmount],
            account: changeUtxoAccount,
            appData: appUtxo === null || appUtxo === void 0 ? void 0 : appUtxo.appData,
            appDataHash: appUtxo === null || appUtxo === void 0 ? void 0 : appUtxo.appDataHash,
            includeAppData: appUtxo === null || appUtxo === void 0 ? void 0 : appUtxo.includeAppData,
            verifierAddress: appUtxo === null || appUtxo === void 0 ? void 0 : appUtxo.verifierAddress,
            assetLookupTable,
            verifierProgramLookupTable,
        });
        outputUtxos.push(changeUtxo);
    }
    if (outputUtxos.length > numberMaxOutUtxos) {
        throw new index_1.CreateUtxoError(index_1.CreateUtxoErrorCode.INVALID_OUTPUT_UTXO_LENGTH, "createOutUtxos", `Probably too many input assets possibly in combination with an incompatible number of shielded recipients`);
    }
    return outputUtxos;
}
exports.createOutUtxos = createOutUtxos;
/**
 * @description Creates an array of UTXOs for each recipient based on their specified amounts and assets.
 *
 * @param recipients - Array of Recipient objects containing the recipient's account, SOL and SPL amounts, and mint.
 * @param poseidon - A Poseidon instance for hashing.
 *
 * @throws CreateUtxoError if a recipient has a mint defined but the SPL amount is undefined.
 * @returns An array of Utxos, one for each recipient.
 */
function createRecipientUtxos({ recipients, poseidon, assetLookupTable, verifierProgramLookupTable, }) {
    var _a, _b, _c, _d;
    var outputUtxos = [];
    // create recipient output utxos, one for each defined recipient
    for (var j in recipients) {
        if (recipients[j].mint && !recipients[j].splAmount) {
            throw new index_1.CreateUtxoError(index_1.CreateUtxoErrorCode.SPL_AMOUNT_UNDEFINED, "createOutUtxos", `Mint defined while splAmount is undefined for recipient ${recipients[j]}`);
        }
        let solAmount = recipients[j].solAmount ? recipients[j].solAmount : index_1.BN_0;
        let splAmount = recipients[j].splAmount ? recipients[j].splAmount : index_1.BN_0;
        let splMint = recipients[j].mint
            ? recipients[j].mint
            : web3_js_1.SystemProgram.programId;
        let recipientUtxo = new index_1.Utxo({
            poseidon,
            assets: [web3_js_1.SystemProgram.programId, splMint],
            amounts: [solAmount, splAmount],
            account: recipients[j].account,
            appData: (_a = recipients[j].appUtxo) === null || _a === void 0 ? void 0 : _a.appData,
            includeAppData: (_b = recipients[j].appUtxo) === null || _b === void 0 ? void 0 : _b.includeAppData,
            appDataHash: (_c = recipients[j].appUtxo) === null || _c === void 0 ? void 0 : _c.appDataHash,
            verifierAddress: (_d = recipients[j].appUtxo) === null || _d === void 0 ? void 0 : _d.verifierAddress,
            assetLookupTable,
            verifierProgramLookupTable,
        });
        outputUtxos.push(recipientUtxo);
    }
    return outputUtxos;
}
exports.createRecipientUtxos = createRecipientUtxos;
/**
 * @description Validates if the sum of input UTXOs for each asset is less than or equal to the sum of output UTXOs.
 *
 * @param assetPubkeys - Array of PublicKeys representing the asset public keys to be checked.
 * @param inUtxos - Array of input UTXOs containing the asset amounts being spent.
 * @param outUtxos - Array of output UTXOs containing the asset amounts being received.
 *
 * @throws Error if the sum of input UTXOs for an asset is less than the sum of output UTXOs.
 */
function validateUtxoAmounts({ assetPubkeys, inUtxos, outUtxos, publicAmountSol, publicAmountSpl, action, }) {
    const publicAmountMultiplier = action === index_1.Action.SHIELD ? index_1.BN_1 : new anchor_1.BN(-1);
    const _publicAmountSol = publicAmountSol
        ? publicAmountSol.mul(publicAmountMultiplier)
        : index_1.BN_0;
    const _publicAmountSpl = publicAmountSpl
        ? publicAmountSpl.mul(publicAmountMultiplier)
        : index_1.BN_0;
    let assets = [];
    for (const [index, assetPubkey] of assetPubkeys.entries()) {
        var sumIn = inUtxos ? (0, exports.getUtxoArrayAmount)(assetPubkey, inUtxos) : index_1.BN_0;
        var sumOut = action === index_1.Action.TRANSFER && outUtxos.length === 0
            ? sumIn
            : (0, exports.getUtxoArrayAmount)(assetPubkey, outUtxos);
        var sumInAdd = assetPubkey.toBase58() === web3_js_1.SystemProgram.programId.toBase58()
            ? sumIn.add(_publicAmountSol)
            : index < 2
                ? sumIn.add(_publicAmountSpl)
                : sumIn;
        var sumOutAdd = assetPubkey.toBase58() === web3_js_1.SystemProgram.programId.toBase58()
            ? sumOut.add(_publicAmountSol)
            : index < 2
                ? sumOut.add(_publicAmountSpl)
                : sumOut;
        sumInAdd = action === index_1.Action.SHIELD ? sumInAdd : sumIn;
        sumOutAdd = action === index_1.Action.SHIELD ? sumOut : sumOutAdd;
        assets.push({
            asset: assetPubkey,
            sumIn,
            sumOut,
        });
        if (sumInAdd.lt(index_1.BN_0))
            throw new index_1.CreateUtxoError(index_1.CreateUtxoErrorCode.RECIPIENTS_SUM_AMOUNT_MISSMATCH, "validateUtxoAmounts", `utxos don't cover the required amount for asset ${assetPubkey.toBase58()} sumIn ${sumIn}  public amount: ${assetPubkey.toBase58() === web3_js_1.SystemProgram.programId.toBase58()
                ? publicAmountSol
                : publicAmountSpl} action: ${action}`);
        if (!sumInAdd.gte(sumOutAdd)) {
            throw new index_1.CreateUtxoError(index_1.CreateUtxoErrorCode.RECIPIENTS_SUM_AMOUNT_MISSMATCH, "validateUtxoAmounts", `for asset ${assetPubkey.toBase58()} sumOut ${sumOut} greather than sumIN ${sumIn}`);
        }
    }
    return assets;
}
exports.validateUtxoAmounts = validateUtxoAmounts;
//# sourceMappingURL=createOutUtxos.js.map