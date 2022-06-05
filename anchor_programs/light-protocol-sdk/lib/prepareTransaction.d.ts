import { BigNumber } from 'ethers';
import { Action } from './enums';
import Utxo from './utxos';
export declare const prepareTransaction: (inputUtxos: Utxo[] | undefined, outputUtxos: Utxo[] | undefined, relayFee: any, action: Action) => {
    inputUtxos: Utxo[];
    outputUtxos: Utxo[];
    externalAmountBigNumber: BigNumber;
};
