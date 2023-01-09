"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.Keypair = void 0;
const nacl = require('tweetnacl');
const anchor = require("@coral-xyz/anchor");
const anchor_1 = require("@coral-xyz/anchor");
const { blake2b } = require('@noble/hashes/blake2b');
const b2params = { dkLen: 32 };
class Keypair {
    constructor(poseidon, seed = new anchor_1.BN(nacl.randomBytes(32)).toString("hex"), index) {
        if (seed.length < 32) {
            throw "seed too short length less than 32";
        }
        this.poseidon = poseidon;
        this.burnerSeed = new Uint8Array();
        // creates a burner utxo by using the index for domain separation
        if (index) {
            // seed can be shared since hash cannot be inverted
            // sharing the burnerSeed saves 32 bytes in onchain data
            this.burnerSeed = blake2b
                .create(b2params)
                .update(seed + "burnerSeed" + index.toString())
                .digest();
            const burnerSeedString = new anchor_1.BN(this.burnerSeed).toString("hex");
            this.encryptionPublicKey = getEncryptionKeyPair(burnerSeedString).publicKey;
            this.privkey = generateShieldedKeyPrivateKey(burnerSeedString);
        }
        else {
            this.encryptionPublicKey = getEncryptionKeyPair(seed).publicKey;
            this.privkey = generateShieldedKeyPrivateKey(seed);
        }
        this.pubkey = generateShieldedKeyPublicKey(this.privkey, this.poseidon);
    }
    encryptionPublicKeyToBytes() {
        return new anchor_1.BN(this.encryptionPublicKey).toBuffer('be', 32);
    }
    // make general for cases in which only pubkey, encPubkey, privkey or pairs of these are defined
    fromBytes({ pubkey, encPubkey, privkey, poseidon, burnerSeed }) {
        if (privkey != undefined) {
            this.privkey = anchor.utils.bytes.hex.encode(privkey);
            this.pubkey = new anchor_1.BN(poseidon.F.toString(this.poseidon([new anchor_1.BN(privkey, undefined, 'le')])));
            this.encryptionPublicKey = getEncryptionKeyPair(new anchor_1.BN(burnerSeed).toString("hex")).publicKey; //getEncryptionPublicKey(new BN(privkey, undefined, 'le').toString("hex", 32));
        }
        else {
            this.pubkey = new anchor_1.BN(pubkey, undefined, 'le');
            this.encryptionPublicKey = encPubkey;
        }
    }
    fromBurnerSeed(burnerSeed, poseidon) {
        this.poseidon = poseidon;
        this.burnerSeed = burnerSeed;
        const burnerSeedString = new anchor_1.BN(burnerSeed).toString("hex");
        this.encryptionPublicKey = getEncryptionKeyPair(burnerSeedString).publicKey;
        this.privkey = generateShieldedKeyPrivateKey(burnerSeedString);
        this.pubkey = generateShieldedKeyPublicKey(this.privkey, this.poseidon);
        return this;
    }
    // fromPubkey(pubkey): Keypair {
    // }
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
function getEncryptionKeyPair(seed) {
    const encSeed = seed + "encryption";
    const encryptionPrivateKey = blake2b
        .create(b2params)
        .update(encSeed)
        .digest();
    return nacl.box.keyPair.fromSecretKey(encryptionPrivateKey);
}
;
function generateShieldedKeyPrivateKey(seed) {
    const privkeySeed = seed + "shielded";
    const privateKey = new anchor_1.BN(blake2b.create(b2params)
        .update(privkeySeed)
        .digest());
    return privateKey;
}
;
function generateShieldedKeyPublicKey(privateKey, poseidon) {
    return new anchor_1.BN(poseidon.F.toString(poseidon([privateKey.toBuffer('be', 32)])));
}
