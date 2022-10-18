"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.unpackEncryptedMessage = exports.packEncryptedMessage = exports.Keypair = void 0;
const eth_sig_util_1 = require("eth-sig-util");
// const ethers_1 = require("ethers");
const nacl = require('tweetnacl');
const anchor = require("@project-serum/anchor")

const poseidonHash_1 = require("./poseidonHash");
const toFixedHex_1 = require("./toFixedHex");
function packEncryptedMessage(encryptedMessage) {
    const nonceBuf = Buffer.from(encryptedMessage.nonce, 'base64');
    const ephemPublicKeyBuf = Buffer.from(encryptedMessage.ephemPublicKey, 'base64');
    const ciphertextBuf = Buffer.from(encryptedMessage.ciphertext, 'base64');
    const messageBuff = Buffer.concat([
        Buffer.alloc(24 - nonceBuf.length),
        nonceBuf,
        Buffer.alloc(32 - ephemPublicKeyBuf.length),
        ephemPublicKeyBuf,
        ciphertextBuf,
    ]);
    return '0x' + messageBuff.toString('hex');
}
exports.packEncryptedMessage = packEncryptedMessage;
function unpackEncryptedMessage(encryptedMessage) {
    if (encryptedMessage.slice(0, 2) === '0x') {
        encryptedMessage = encryptedMessage.slice(2);
    }
    const messageBuff = Buffer.from(encryptedMessage, 'hex');
    const nonceBuf = messageBuff.slice(0, 24);
    const ephemPublicKeyBuf = messageBuff.slice(24, 56);
    const ciphertextBuf = messageBuff.slice(56);
    return {
        version: 'x25519-xsalsa20-poly1305',
        nonce: nonceBuf.toString('base64'),
        ephemPublicKey: ephemPublicKeyBuf.toString('base64'),
        ciphertext: ciphertextBuf.toString('base64'),
    };
}
exports.unpackEncryptedMessage = unpackEncryptedMessage;
class Keypair {
    constructor(poseidon, privkey = anchor.utils.bytes.hex.encode(nacl.randomBytes(32))) {
        this.privkey = privkey;

        this.pubkey = poseidon.F.toString(poseidon([this.privkey]));
        this.encryptionKey = (0, eth_sig_util_1.getEncryptionPublicKey)(privkey.slice(2));
        this.poseidon = poseidon;
    }
    toString() {
        return ((0, toFixedHex_1.toFixedHex)(this.pubkey) +
            Buffer.from(this.encryptionKey, 'base64').toString('hex'));
    }
    /**
     * Key address for this keypair, alias to {@link toString}
     *
     * @returns {string}
     */
    address() {
        return this.toString();
    }

    /**
     * Initialize new keypair from address outPubkeystring
     *
     * @param str
     * @returns {Keypair}
     */
    static fromString(str) {
        if (str.length === 130) {
            str = str.slice(2);
        }
        if (str.length !== 128) {
            throw new Error('Invalid key length');
        }
        return Object.assign(new Keypair(), {
            privkey: null,
            pubkey: new anchor.BN('0x' + str.slice(0, 64)),
            encryptionKey: Buffer.from(str.slice(64, 128), 'hex').toString('base64'),
        });
    }
    /**
     * Sign a message using keypair private key
     *
     * @param {string|number|BigNumber} commitment a hex string with commitment
     * @param {string|number|BigNumber} merklePath a hex string with merkle path
     * @returns {BigNumber} a hex string with signature
     */
    sign(commitment, merklePath) {
        return this.poseidon.F.toString(this.poseidon([this.privkey, commitment, merklePath]));
    }
    
}
exports.Keypair = Keypair;
