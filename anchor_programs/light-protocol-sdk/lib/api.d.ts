import solana from '@solana/web3.js';
import { Action } from './enums';
export declare const withdraw: ({ publicKey, inputBytes, proofBytes, extDataBytes, recipient, action, amount, encryptionKeypair, uuid, timeout, }: {
    publicKey: solana.PublicKey | null;
    inputBytes: any[];
    proofBytes: any;
    extDataBytes: any;
    recipient: string;
    action: Action;
    amount: number;
    encryptionKeypair: nacl.BoxKeyPair;
    uuid: string;
    timeout: number;
}) => Promise<any>;
