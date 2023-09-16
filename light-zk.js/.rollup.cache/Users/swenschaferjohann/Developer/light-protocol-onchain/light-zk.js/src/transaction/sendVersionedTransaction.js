import { AddressLookupTableAccount, ComputeBudgetProgram, TransactionMessage, VersionedTransaction, } from "@solana/web3.js";
import { confirmConfig } from "../constants";
export const sendVersionedTransaction = async (ix, connection, lookUpTable, payer) => {
    var _a;
    const recentBlockhash = (await connection.getLatestBlockhash(confirmConfig))
        .blockhash;
    const ixSigner = (_a = ix.keys
        .map((key) => {
        if (key.isSigner == true)
            return key.pubkey;
    })[0]) === null || _a === void 0 ? void 0 : _a.toBase58();
    if (payer.publicKey.toBase58() != ixSigner) {
        throw new Error(` payer pubkey is not equal instruction signer ${payer.publicKey.toBase58()} != ${ixSigner} (only one signer supported)`);
    }
    const txMsg = new TransactionMessage({
        payerKey: payer.publicKey,
        instructions: [
            ComputeBudgetProgram.setComputeUnitLimit({ units: 1400000 }),
            ix,
        ],
        recentBlockhash: recentBlockhash,
    });
    const lookupTableAccount = await connection.getAccountInfo(lookUpTable, "confirmed");
    const unpackedLookupTableAccount = AddressLookupTableAccount.deserialize(lookupTableAccount.data);
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
    var tx = new VersionedTransaction(compiledTx);
    let retries = 3;
    while (retries > 0) {
        tx = await payer.signTransaction(tx);
        try {
            return await connection.sendTransaction(tx, confirmConfig);
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
export async function sendVersionedTransactions(instructions, connection, lookUpTable, payer) {
    try {
        let signatures = [];
        for (var instruction of instructions) {
            let signature = await sendVersionedTransaction(instruction, connection, lookUpTable, payer);
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
export async function confirmTransaction(connection, signature, confirmation = "confirmed") {
    const latestBlockHash = await connection.getLatestBlockhash(confirmation);
    let strategy = {
        signature: signature.toString(),
        lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
        blockhash: latestBlockHash.blockhash,
    };
    return await connection.confirmTransaction(strategy, confirmation);
}
//# sourceMappingURL=sendVersionedTransaction.js.map