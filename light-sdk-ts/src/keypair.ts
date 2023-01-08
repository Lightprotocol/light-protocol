const nacl = require('tweetnacl');
const anchor = require("@coral-xyz/anchor")
import {getEncryptionPublicKey} from "eth-sig-util"
import {BN, utils } from '@coral-xyz/anchor'
const { blake2b } = require('@noble/hashes/blake2b');
const b2params = {dkLen: 32 };
export class Keypair {
  /**
   * Initialize a new keypair. Generates a random private key if not defined
   *
   * @param {string} privkey
   */
  privkey: BN
  pubkey: BN
  encryptionKey: String
  poseidon: any

  constructor(
    poseidon: any,
    seed = new BN(nacl.randomBytes(32)).toString(),
    index?: BN
  ) {

      // creates a burner utxo by using the index for domain separation
      if (index) {
        let encSeed = seed + "encryption" +  index.toString();
        this.encryptionKey = getEncryptionPublicKey(
          blake2b
          .create(b2params)
          .update(encSeed)
          .digest()
        );

        let privkeySeed = seed + "burner" +  index.toString();
        this.privkey = new BN(blake2b
        .create(b2params)
        .update(privkeySeed)
        .digest())
      } else {
        
        let encSeed = seed + "encryption";
        this.encryptionKey = getEncryptionPublicKey(
          blake2b
          .create(b2params)
          .update(encSeed)
          .digest()
        );
  
        let privkeySeed = seed + "privkey";
        this.privkey = new BN(blake2b.create(b2params)
          .update(privkeySeed)
          .digest())
        
      }
      this.pubkey = new BN(poseidon.F.toString(poseidon([this.privkey])));
      // Should be getEncryptionPublicKey(Sha3([ed25519Sig(),"encryption"].concat()))
      // this.encryptionKey = getEncryptionPublicKey(privkey.toString("hex", 32));
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
    {pubkey, encPubkey, privkey, poseidon}:
    {pubkey: Array<any>, encPubkey: Array<any>, privkey: Array<any>, poseidon: any}
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
  sign(commitment: any, merklePath: any) {
      return this.poseidon.F.toString(this.poseidon([this.privkey, commitment, merklePath]));
  }

}
