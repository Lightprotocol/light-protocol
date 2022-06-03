import Utxo from './utxos';
import { Keypair } from './utils/keypair';
export declare const createOutputUtxo: (inputUtxos: any[], amount: number, shieldedKeypair: Keypair, relayerFee: any) => Utxo;
