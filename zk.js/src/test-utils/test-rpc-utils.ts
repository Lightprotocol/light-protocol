import {bs58} from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import {UTXO_PREFIX_LENGTH} from "../constants";

export const getIdsFromEncryptedUtxos = (
    encryptedUtxos: Buffer,
    numberOfLeaves: number,
): string[] => {
    const utxoLength = 124; //encryptedUtxos.length / numberOfLeaves;
    // divide encrypted utxos by multiples of 2
    // and extract the first two bytes of each
    const ids: string[] = [];
    for (let i = 0; i < encryptedUtxos.length; i += utxoLength) {
        ids.push(bs58.encode(encryptedUtxos.slice(i, i + UTXO_PREFIX_LENGTH)));
    }
    return ids;
};
