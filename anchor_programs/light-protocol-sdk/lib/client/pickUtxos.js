"use strict";
// TODO change any to type
Object.defineProperty(exports, "__esModule", { value: true });
exports.pickUtxos = void 0;
const pickUtxos = (amount, utxos) => {
    let options = [];
    let set = new Set();
    for (var utxo of utxos) {
        // 2 perfect match
        let num = amount - Number(utxo.amount._hex);
        if (set.has(num)) {
            options.push(num, utxo);
            break;
        }
        set.add(utxo);
    }
    if (options.length < 1) {
        // 1 perfect match
        let match = utxos.filter((utxo) => Number(utxo.amount._hex) == Number(amount));
        if (match.length > 0) {
        }
        options.push(...match);
    }
    // 2 above
    let i, j;
    if (options.length < 1) {
        for (i = 0; i < utxos.length; i++) {
            for (j = 0; j < utxos.length; j++) {
                if (i == j)
                    continue;
                else if (Number(utxos[i].amount._hex) + Number(utxos[j].amount._hex) >=
                    Number(amount)) {
                    options.push(utxos[i], utxos[j]);
                    return options;
                }
            }
        }
    }
    if (options.length < 1) {
        // 1 above
        let match = utxos.filter((utxo) => Number(utxo.amount._hex) >= Number(amount));
        if (match.length > 0) {
        }
        options.push(...match);
    }
    return options;
};
exports.pickUtxos = pickUtxos;
