"use strict";
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.setUserUtxos = void 0;
function setUserUtxos(connection, recipientEncryptionKeypair, shieldedKeypair, ekpN, skpN) {
    return __awaiter(this, void 0, void 0, function* () {
        // Get all leaves
        var leafAccounts = yield connection.getProgramAccounts(new solana.PublicKey(process.env.NEXT_PUBLIC_PROGRAM_ID || ''), {
            filters: [
                {
                    dataSize: 106 + 222,
                },
            ], // 0..10index 10..74leaves 74..106mtpubkey 106..encryptedutxos
        });
        /// Sort all leaves
        var leavesToSort = [];
        var sortedLeafAccounts = [];
        leafAccounts.map((acc) => {
            leavesToSort.push({
                index: U64(acc.account.data.slice(2, 10)).toString(),
                leaves: acc.account.data.slice(10, 74),
                data: acc.account.data,
            });
        });
        leavesToSort.sort((a, b) => parseFloat(a.index) - parseFloat(b.index));
        sortedLeafAccounts = leavesToSort;
        var leaves = [];
        for (var i = 0; i < leafAccounts.length; i++) {
            leaves.push(toFixedHex(leavesToSort[i].leaves.slice(0, 32).reverse()));
            leaves.push(toFixedHex(leavesToSort[i].leaves.slice(32, 64).reverse()));
        }
        var userUtxos = [];
        sortedLeafAccounts.map((acc) => {
            let utxoPair = [];
            let decrypted = [];
            let nonces = [];
            let senderThrowAwayPubkeys = [];
            utxoPair[0] = acc.data.slice(106, 161); // 55
            nonces[0] = acc.data.slice(161, 185);
            senderThrowAwayPubkeys[0] = acc.data.slice(185, 217);
            utxoPair[1] = acc.data.slice(217, 272);
            nonces[1] = acc.data.slice(272, 296);
            senderThrowAwayPubkeys[1] = acc.data.slice(296, 328);
            // Try decrypt utxos
            utxoPair.map((encryptedUtxo, i) => {
                var [success, utxo] = Utxo.decrypt(encryptedUtxo, nonces[i], senderThrowAwayPubkeys[i], recipientEncryptionKeypair, shieldedKeypair, acc.index);
                if (success) {
                    decrypted.push(utxo);
                }
            });
            userUtxos.push(...decrypted);
            // Try decrypt utxos with new ekpN
            // decrypted = [];
            // utxoPair.map((encryptedUtxo, i) => {
            //   let [success, utxo] = Utxo.decrypt(
            //     encryptedUtxo,
            //     nonces[i],
            //     senderThrowAwayPubkeys[i],
            //     ekpN,
            //     skpN,
            //     acc.index,
            //   );
            //   if (success) {
            //     decrypted.push(utxo);
            //   }
            // });
            // userUtxos.push(...decrypted);
        });
        /// Remove utxos that dont hold value
        let fullUtxos = userUtxos.filter((utxo) => Number(utxo.amount._hex) > 0);
        /// set for deposit
        let nextIndex = fullUtxos.length;
        /// collect nullifier pubkeys
        var nullifier_accounts = yield connection.getProgramAccounts(program_pubKey, {
            filters: [{ dataSize: 2 }],
        });
        let nullifierPubkeys = [];
        nullifier_accounts.map((acc) => nullifierPubkeys.push(acc.pubkey.toBase58()));
        let unspentUtxos = [];
        // Filter unspent Utxos
        let promises = fullUtxos.map((utxo) => __awaiter(this, void 0, void 0, function* () {
            utxo.index = leaves.indexOf(toFixedHex(utxo.getCommitment()));
            let nullifier = yield solana.PublicKey.findProgramAddress(
            // [110,102] nonce like onchain
            [leInt2Buffer(utxo.getNullifier().toString()), [110, 102]], program_pubKey);
            if (nullifierPubkeys.indexOf(nullifier[0].toBase58()) < 0) {
                unspentUtxos.push(utxo);
            }
        }));
        yield Promise.all(promises);
        /// Calculate user's balance
        let userBalance = 0;
        for (let utxo of unspentUtxos) {
            userBalance += Number(utxo.amount._hex);
        }
        return { unspentUtxos, userBalance, nextIndex };
    });
}
exports.setUserUtxos = setUserUtxos;
