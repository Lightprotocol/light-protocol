"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.confirmTransaction = exports.sendVersionedTransactions = exports.sendVersionedTransaction = void 0;
const web3_js_1 = require("@solana/web3.js");
const constants_1 = require("../constants");
const sendVersionedTransaction = async (ix, connection, lookUpTable, payer) => {
    var _a;
    const recentBlockhash = (await connection.getLatestBlockhash(constants_1.confirmConfig))
        .blockhash;
    const ixSigner = (_a = ix.keys
        .map((key) => {
        if (key.isSigner == true)
            return key.pubkey;
    })[0]) === null || _a === void 0 ? void 0 : _a.toBase58();
    if (payer.publicKey.toBase58() != ixSigner) {
        throw new Error(` payer pubkey is not equal instruction signer ${payer.publicKey.toBase58()} != ${ixSigner} (only one signer supported)`);
    }
    const txMsg = new web3_js_1.TransactionMessage({
        payerKey: payer.publicKey,
        instructions: [
            web3_js_1.ComputeBudgetProgram.setComputeUnitLimit({ units: 1400000 }),
            ix,
        ],
        recentBlockhash: recentBlockhash,
    });
    const lookupTableAccount = await connection.getAccountInfo(lookUpTable, "confirmed");
    const unpackedLookupTableAccount = web3_js_1.AddressLookupTableAccount.deserialize(lookupTableAccount.data);
    const compiledTx = txMsg.compileToV0Message([
        {
            state: unpackedLookupTableAccount,
            key: lookUpTable,
            isActive: () => {
                return true;
            },
        },
    ]);
    if (compiledTx.addressTableLookups[0]) {
        compiledTx.addressTableLookups[0].accountKey = lookUpTable;
    }
    var tx = new web3_js_1.VersionedTransaction(compiledTx);
    let retries = 3;
    while (retries > 0) {
        tx = await payer.signTransaction(tx);
        try {
            return await connection.sendTransaction(tx, constants_1.confirmConfig);
        }
        catch (e) {
            console.log(e);
            retries--;
            if (retries == 0 || e.logs !== undefined) {
                console.log(e);
                throw e;
            }
        }
    }
};
exports.sendVersionedTransaction = sendVersionedTransaction;
async function sendVersionedTransactions(instructions, connection, lookUpTable, payer) {
    try {
        let signatures = [];
        for (var instruction of instructions) {
            let signature = await (0, exports.sendVersionedTransaction)(instruction, connection, lookUpTable, payer);
            if (!signature)
                throw new Error("sendVersionedTransactions: signature is undefined");
            signatures.push(signature);
            await confirmTransaction(connection, signature);
        }
        return { signatures };
    }
    catch (error) {
        return { error };
    }
}
exports.sendVersionedTransactions = sendVersionedTransactions;
async function confirmTransaction(connection, signature, confirmation = "confirmed") {
    const latestBlockHash = await connection.getLatestBlockhash(confirmation);
    let strategy = {
        signature: signature.toString(),
        lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
        blockhash: latestBlockHash.blockhash,
    };
    return await connection.confirmTransaction(strategy, confirmation);
}
exports.confirmTransaction = confirmTransaction;
//# sourceMappingURL=sendVersionedTransaction.js.map