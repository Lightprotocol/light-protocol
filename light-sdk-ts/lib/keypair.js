"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.Keypair = void 0;
const nacl = require('tweetnacl');
const anchor = require("@coral-xyz/anchor");
const anchor_1 = require("@coral-xyz/anchor");
const { blake2b } = require('@noble/hashes/blake2b');
const b2params = { dkLen: 32 };
class Keypair {
    constructor({ poseidon, seed = new anchor_1.BN(nacl.randomBytes(32)).toString("hex"), burner = false, privateKey, publicKey }) {
        if (seed.length < 32) {
            throw "seed too short length less than 32";
        }
        this.poseidon = poseidon;
        this.burnerSeed = new Uint8Array();
        // creates a burner utxo by using the index for domain separation
        if (burner) {
            // burnerSeed can be shared since hash cannot be inverted - only share this for app utxos
            // sharing the burnerSeed saves 32 bytes in onchain data if it is require to share both
            // the encryption and private key of a utxo
            this.burnerSeed = new anchor_1.BN(seed, "hex").toBuffer("be", 32);
            this.privkey = Keypair.generateShieldedKeyPrivateKey(seed);
            this.encryptionPublicKey = Keypair.getEncryptionKeyPair(seed).publicKey;
            this.encPrivateKey = Keypair.getEncryptionKeyPair(seed).secretKey;
            this.pubkey = Keypair.generateShieldedKeyPublicKey(this.privkey, this.poseidon);
        }
        else if (privateKey) {
            this.privkey = privateKey;
            this.encryptionPublicKey = new Uint8Array();
            this.pubkey = Keypair.generateShieldedKeyPublicKey(this.privkey, this.poseidon);
        }
        else if (publicKey) {
            this.pubkey = publicKey;
            this.privkey = new anchor_1.BN('0');
            this.encryptionPublicKey = new Uint8Array();
        }
        else {
            this.privkey = Keypair.generateShieldedKeyPrivateKey(seed);
            this.encryptionPublicKey = Keypair.getEncryptionKeyPair(seed).publicKey;
            this.encPrivateKey = Keypair.getEncryptionKeyPair(seed).secretKey;
            this.pubkey = Keypair.generateShieldedKeyPublicKey(this.privkey, this.poseidon);
        }
    }
    encryptionPublicKeyToBytes() {
        return new anchor_1.BN(this.encryptionPublicKey).toBuffer('be', 32);
    }
    // might  be obsolete
    // make general for cases in which only pubkey, encPubkey, privkey or pairs of these are defined
    fromBytes({ pubkey, encPubkey, privkey, poseidon, burnerSeed }) {
        if (privkey != undefined) {
            this.privkey = anchor.utils.bytes.hex.encode(privkey);
            this.pubkey = new anchor_1.BN(poseidon.F.toString(this.poseidon([new anchor_1.BN(privkey, undefined, 'le')])));
            this.encryptionPublicKey = Keypair.getEncryptionKeyPair(new anchor_1.BN(burnerSeed).toString("hex")).publicKey; //getEncryptionPublicKey(new BN(privkey, undefined, 'le').toString("hex", 32));
        }
        else {
            this.pubkey = new anchor_1.BN(pubkey, undefined, 'le');
            this.encryptionPublicKey = encPubkey;
        }
    }
    /**
       * Sign a message using keypair private key
       *
       * @param {string|number|BigNumber} commitment a hex string with commitment
       * @param {string|number|BigNumber} merklePath a hex string with merkle path
       * @returns {BigNumber} a hex string with signature
       */
    sign(commitment, merklePath) {
        return this.poseidon.F.toString(this.poseidon([this.privkey.toString(), commitment.toString(), merklePath]));
    }
    static createBurner(poseidon, seed, index) {
        const burnerSeed = blake2b
            .create(b2params)
            .update(seed + "burnerSeed" + index.toString("hex"))
            .digest();
        const burnerSeedString = new anchor_1.BN(burnerSeed).toString("hex");
        return new Keypair({ poseidon, seed: burnerSeedString, burner: true });
    }
    static fromBurnerSeed(poseidon, burnerSeed) {
        const burnerSeedString = new anchor_1.BN(burnerSeed).toString("hex");
        return new Keypair({ poseidon, seed: burnerSeedString, burner: true });
    }
    static fromPrivkey(poseidon, privateKey) {
        const privkey = new anchor_1.BN(privateKey);
        return new Keypair({ poseidon, privateKey: privkey });
    }
    static fromPubkey(poseidon, publicKey) {
        const pubKey = new anchor_1.BN(publicKey, undefined, "be");
        return new Keypair({ poseidon, publicKey: pubKey });
    }
    static getEncryptionKeyPair(seed) {
        const encSeed = seed + "encryption";
        const encryptionPrivateKey = blake2b
            .create(b2params)
            .update(encSeed)
            .digest();
        return nacl.box.keyPair.fromSecretKey(encryptionPrivateKey);
    }
    ;
    static generateShieldedKeyPrivateKey(seed) {
        const privkeySeed = seed + "shielded";
        const privateKey = new anchor_1.BN(blake2b.create(b2params)
            .update(privkeySeed)
            .digest());
        return privateKey;
    }
    ;
    static generateShieldedKeyPublicKey(privateKey, poseidon) {
        return new anchor_1.BN(poseidon.F.toString(poseidon([privateKey])));
    }
}
exports.Keypair = Keypair;
