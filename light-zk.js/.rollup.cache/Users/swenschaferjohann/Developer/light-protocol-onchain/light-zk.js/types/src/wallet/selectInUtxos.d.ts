/// <reference types="bn.js" />
import { PublicKey } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import { Action, Utxo } from "../index";
export declare const getAmount: (u: Utxo, asset: PublicKey) => BN;
export declare const getUtxoSum: (utxos: Utxo[], asset: PublicKey) => BN;
export declare function selectInUtxos({ utxos, publicMint, publicAmountSpl, publicAmountSol, poseidon, relayerFee, inUtxos, outUtxos, action, numberMaxInUtxos, numberMaxOutUtxos, }: {
    publicMint?: PublicKey;
    publicAmountSpl?: BN;
    publicAmountSol?: BN;
    poseidon: any;
    relayerFee?: BN;
    utxos?: Utxo[];
    inUtxos?: Utxo[];
    outUtxos?: Utxo[];
    action: Action;
    numberMaxInUtxos: number;
    numberMaxOutUtxos: number;
}): Utxo[];
//# sourceMappingURL=selectInUtxos.d.ts.map