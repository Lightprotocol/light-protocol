import { PublicKey } from "@solana/web3.js"



export class Relayer {
relayerPubkey: PublicKey // signs the transaction
encryptionPubkey: Uint8Array
relayerRecipient: PublicKey // receives the fees
lookUpTable: PublicKey

constructor(
    relayerPubkey: PublicKey,encryptionPubkey: Uint8Array,relayerRecipient: PublicKey,
    lookUpTable: PublicKey) {
    this.relayerPubkey = relayerPubkey;
    this.encryptionPubkey = encryptionPubkey;
    this.relayerRecipient = relayerRecipient
    this.lookUpTable = lookUpTable
}

};