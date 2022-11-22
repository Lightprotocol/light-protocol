const nacl = require('tweetnacl');
const anchor = require("@project-serum/anchor")

export class Keypair {
  /**
   * Initialize a new keypair. Generates a random private key if not defined
   *
   * @param {string} privkey
   */
  privkey: string
  pubkey: any
  encryptionKey: any
  constructor(
    poseidon,
    privkey = anchor.utils.bytes.hex.encode(nacl.randomBytes(32))
  ) {
      this.privkey = privkey;
      this.pubkey = poseidon.F.toString(poseidon([this.privkey]));
      this.encryptionKey = (0, eth_sig_util_1.getEncryptionPublicKey)(privkey.slice(2));
      this.poseidon = poseidon;
  }

  // add these methods and just json stringify the object
  toString() {
    console.log("not implemented");
  }

  fromString() {
    console.log("not implemented");
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
