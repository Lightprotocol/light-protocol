import { Keypair } from './utils/keypair';
import solana from '@solana/web3.js';
import Utxo from './utxos';
export declare class LightSDK {
    unshield(recipient: string, amount: number, handle: string): Promise<any>;
    /**
     * Prepare and send unshielding
     *
     * @param publicKey publicKey of [placeholder]
     * @param recipient recipient of the unshielding
     * @param amount amount to be unshielded
     * @param token token used for unshielding
     * @param encryptionPubkey encryptionPubkey used for encryption
     * @param inputUtxos utxos to pay with
     * @param relayerFee fee for the relayer
     * @param connection RPC connection
     * @param shieldedKeypair shieldedKeypair
     * @param timeout timeout timestamp in ms
     */
    prepareAndSendUnshield(publicKey: solana.PublicKey | null, recipient: string, amount: number, token: string, encryptionKeypair: nacl.BoxKeyPair, inputUtxos: Utxo[], relayerFee: any, connection: solana.Connection, shieldedKeypair: Keypair, uuid: string, timeout: number): Promise<{
        recipient: string;
        amount: number;
    }>;
}
