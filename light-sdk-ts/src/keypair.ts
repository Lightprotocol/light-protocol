const nacl = require('tweetnacl');
const anchor = require("@project-serum/anchor")
import {getEncryptionPublicKey} from "eth-sig-util"
import {BN, utils } from '@project-serum/anchor'

export class Keypair {
  /**
   * Initialize a new keypair. Generates a random private key if not defined
   *
   * @param {string} privkey
   */
  privkey: BN
  pubkey: BN
  encryptionKey: any
  poseidon: any

  constructor(
    poseidon,
    // TODO: change into bytes
    privkey = new BN(nacl.randomBytes(32))
  ) {
      // TODO: change key derivation and write tests
      // privkey should be Sha3([ed25519Sig(),"shielded"].concat())
      this.privkey = privkey;
      console.log(this.privkey);
      
      this.pubkey = new BN(poseidon.F.toString(poseidon([this.privkey])));
      // Should be getEncryptionPublicKey(Sha3([ed25519Sig(),"encryption"].concat()))
      this.encryptionKey = getEncryptionPublicKey(privkey.toString("hex", 32));
      this.poseidon = poseidon;
  }

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


  fromBytes(
    {pubkey, encPubkey, privkey}:
    {pubkey: Array<any>, encPubkey: Array<any>, privkey: Array<any>}
    ) {
    if(privkey != undefined) {
      this.privkey = anchor.utils.bytes.hex.encode(privkey);
      this.pubkey = new BN(poseidon.F.toString(this.poseidon([new BN(privkey, undefined, 'le')])));
      this.encryptionKey = getEncryptionPublicKey(new BN(privkey, undefined, 'le').toString("hex", 32));

    } else {
      this.pubkey = new BN(pubkey, undefined, 'le');
      this.encryptionKey = utils.bytes.base64.encode(encPubkey);
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
      return this.poseidon.F.toString(this.poseidon([this.privkey, commitment, merklePath]));
  }

}
