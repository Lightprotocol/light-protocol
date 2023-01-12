"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.Keypair = void 0;
const nacl = require("tweetnacl");
const anchor = require("@project-serum/anchor");
const eth_sig_util_1 = require("eth-sig-util");
const anchor_1 = require("@project-serum/anchor");
const sodium = require("libsodium");
class Keypair {
  constructor(
    poseidon,
    seed = new anchor_1.BN(nacl.randomBytes(32)).toString(),
    index
  ) {
    // TODO: change key derivation and write tests
    // privkey should be Sha3([ed25519Sig(),"shielded"].concat())
    if (index) {
      let encSeed = seed + "encryption";
      this.encryptionKey = (0, eth_sig_util_1.getEncryptionPublicKey)(
        sodium.crypto_generichash(32, sodium.from_string(encSeed))
      );
      let privkeySeed = seed + "burner" + index.toString();
      this.privkey = sodium.crypto_generichash(
        32,
        sodium.from_string(privkeySeed)
      );
    } else {
      let encSeed = seed + "encryption";
      this.encryptionKey = (0, eth_sig_util_1.getEncryptionPublicKey)(
        sodium.crypto_generichash(32, sodium.from_string(encSeed))
      );
      let privkeySeed = seed + "privkey";
      this.privkey = sodium.crypto_generichash(
        32,
        sodium.from_string(privkeySeed)
      );
    }
    this.pubkey = new anchor_1.BN(
      poseidon.F.toString(poseidon([this.privkey]))
    );
    // Should be getEncryptionPublicKey(Sha3([ed25519Sig(),"encryption"].concat()))
    // this.encryptionKey = getEncryptionPublicKey(privkey.toString("hex", 32));
    this.poseidon = poseidon;
  }
  // seed is currently a signature the user signs
  // fromSeed(seed: String, poseidon: any) {
  //   let encSeed = seed + "encryption";
  //   this.encryptionKey = getEncryptionPublicKey(
  //     sodium.crypto_generichash(32, sodium.from_string(encSeed))
  //   );
  //   let privkeySeed = seed + "privkey";
  //   this.privkey = sodium.crypto_generichash(32, sodium.from_string(privkeySeed))
  // }
  // new burner
  // newBurner(seed: String, poseidon: any, index: BN) {
  //   let encSeed = seed + "encryption";
  //   this.encryptionKey = getEncryptionPublicKey(
  //     sodium.crypto_generichash(32, sodium.from_string(encSeed))
  //   );
  //   let privkeySeed = seed + "burner" +  index.toString();
  //   this.privkey = sodium.crypto_generichash(32, sodium.from_string(privkeySeed))
  //   this.poseidon = poseidon;
  // }
  // add these methods and just json stringify the object
  pubKeyToBytes() {
    console.log("not implemented");
  }
  privKeyToBytes() {
    console.log("not implemented");
  }
  encryptionKeyToBytes() {
    console.log("not implemented");
  }
  fromBytes({ pubkey, encPubkey, privkey }) {
    if (privkey != undefined) {
      this.privkey = anchor.utils.bytes.hex.encode(privkey);
      this.pubkey = new anchor_1.BN(
        poseidon.F.toString(
          this.poseidon([new anchor_1.BN(privkey, undefined, "le")])
        )
      );
      this.encryptionKey = (0, eth_sig_util_1.getEncryptionPublicKey)(
        new anchor_1.BN(privkey, undefined, "le").toString("hex", 32)
      );
    } else {
      this.pubkey = new anchor_1.BN(pubkey, undefined, "le");
      this.encryptionKey = anchor_1.utils.bytes.base64.encode(encPubkey);
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
    return this.poseidon.F.toString(
      this.poseidon([this.privkey, commitment, merklePath])
    );
  }
}
exports.Keypair = Keypair;
