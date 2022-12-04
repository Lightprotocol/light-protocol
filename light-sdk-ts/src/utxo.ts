import { Keypair } from './keypair'
import { BigNumber, BytesLike, ethers } from 'ethers'
import { box} from 'tweetnacl'
const crypto = require('crypto');
const randomBN = (nbytes = 31) => new anchor.BN(crypto.randomBytes(nbytes));
exports.randomBN = randomBN;
const anchor = require("@project-serum/anchor")
import {toBufferLE, toBigIntLE} from 'bigint-buffer';
import { thawAccountInstructionData } from '@solana/spl-token';
import { FEE_ASSET, FIELD_SIZE, FIELD_SIZE_ETHERS, MINT, MINT_CIRCUIT } from './constants';
import {hashAndTruncateToCircuit} from './utils';
import { PublicKey, SystemProgram } from '@solana/web3.js';
// import { BN } from 'bn.js';
var ffjavascript = require('ffjavascript');
const { unstringifyBigInts, stringifyBigInts, leInt2Buff, leBuff2int } = ffjavascript.utils;
const N_ASSETS = 3;
import {BN } from '@project-serum/anchor'

// TODO: write test
export class Utxo {
  /** Initialize a new UTXO - unspent transaction output or input. Note, a full TX consists of 2 inputs and 2 outputs
   *
   * @param {new BN[]} amounts array UTXO amount
   * @param {new BN | new BN | number | string} blinding Blinding factor
   */

  amounts: BN[]
  assets: BN[]
  blinding: BN
  keypair: Keypair
  index: number | null
  appData: Array<any>
  verifierAddress: BN
  verifierAddressCircuit: BN
  instructionType: BigNumber
  poolType: BN
  _commitment: BN | null
  _nullifier: BN | null

  constructor({
    poseidon,
    assets = [0, 0, 0],
    amounts = [0, 0, 0],
    keypair, // shielded pool keypair that is derived from seedphrase. OutUtxo: supply pubkey
    // blinding = randomBN(),
    poolType = new BN('0'),
    verifierAddress = SystemProgram.programId,
    appData = [],
    index = null
  }) {
    if (assets.length != amounts.length) {
      throw `utxo constructor: asset.length  ${assets.length}!= amount.length ${amounts.length}`;
    }
    while (assets.length < N_ASSETS) {
      assets.push(new anchor.BN(0))
    }
    for (var i= 0; i < N_ASSETS; i++) {
      if (amounts[i] < 0) {
          throw `utxo constructor: amount cannot be negative, amounts[${i}] = ${amounts[i]}`
      }
    }

    while (amounts.length < N_ASSETS) {
      amounts.push(0)
    }
    if (!keypair) {
      keypair = new Keypair(poseidon)
    }
    // let appDataArray = new Array<any>;
    // for (let elem in appData) {
    //   console.log(Array.from(appData[elem].toString()));
      
    //   appDataArray.push(Array.from(appData[elem]));
    // }
    
    // console.log("appDataArray.flat() ",appDataArray.flat());
    
    if (appData.length > 0) {
      this.instructionType = BigNumber.from(ethers.utils.keccak256(appData).toString()).mod(FIELD_SIZE_ETHERS);
    } else {
      this.instructionType = new BN('0');
    }

    //TODO: add check that amounts are U64
    this.amounts = amounts.map((x) => new BN(x.toString()));
    this.blinding = new BN(randomBN());
    this.keypair = keypair;
    this.index = index;
    // TODO: add hashAndTruncateToCircuit for assets
    console.log("assets ", assets);
    
    this.assets = assets.map((x) => new BN(x.toString()));
    // this.assetsCircuit =  assets.map((x) => new BN(x.toString()))
    // this.hashAndTruncateToCircuit(verifierAddress.toBytes());

    this._commitment = null;
    this._nullifier = null;
    this.poseidon = poseidon;
    this.appData = appData;
    this.poolType = poolType;
    // TODO: add hashAndTruncateToCircuit for verifierAddress
    this.verifierAddress = verifierAddress;
    this.verifierAddressCircuit = hashAndTruncateToCircuit(verifierAddress.toBytes());
  }

  // TODO: rethink this is headache because we cannot invert the derivation. Thus we need to look up the pubkey.
  // TODO: implement lookup logic.
  // Hashes and truncates ed25519 pubkey to fit field size of bn254
  // hashAndTruncateToCircuit(data: Uint8Array) {
  //   return new BN(leInt2Buff(BigNumber.from(ethers.utils.keccak256(data).toString()), 32).slice(0,31), undefined, 'le')
  // }

 
  toBytes() {
    //TODO: get assetIndex(this.asset[1])
    let assetIndex = new BN("0");

    // case no appData
    if (this.instructionType.toString() == '0') {
      console.log("isntruction type is 0");
      
      return new Uint8Array([
        ...this.blinding.toBuffer(),
        ...this.amounts[0].toArray('le', 8),
        ...this.amounts[1].toArray('le', 8),
        ...new BN(assetIndex).toArray('le', 8)
    ]);
    }

    console.log("this.instructionType" ,this.instructionType);
    
    return new Uint8Array([
      ...this.blinding.toArray('le', 31),
      ...this.amounts[0].toArray('le', 8),
      ...this.amounts[1].toArray('le', 8),
      ...assetIndex.toArray('le', 8),
      ...leInt2Buff(unstringifyBigInts(this.instructionType.toString()), 32),
      ...this.poolType.toArray('le', 8),
      ...this.verifierAddressCircuit.toArray('le', 31),
      ...new Array(1),
      ...this.appData
    ]);
  }
  // take a decrypted byteArray as input
  // TODO: make robust and versatile for any combination of filled in fields or not
  fromBytes(bytes: Uint8Array, keypair = null) {
    console.log("here");
    
    //TODO: fetch asset from index
    // let assetIndex = toBigIntLE(bytes.slice(47,55));
    this.assets = [FEE_ASSET, MINT_CIRCUIT] // assets

    // TODO: reevaluate
    // this.keypair = keypair;
    
    console.log(this.assets);
    
    console.log("here1");
    this.amounts =  [new BN(bytes.slice(31,39), undefined, 'le'), new BN(bytes.slice(39,47), undefined, 'le')] // amounts
    console.log("here2");
    this.instructionType =  BigNumber.from(leBuff2int(bytes.slice(55,87)).toString()) // instruction Type
    console.log("here3");
    this.poolType =  new BN(bytes.slice(87,95), undefined, 'le'), // pool Type
    console.log("here4");
    this.blinding =  new BN(bytes.slice(0,31), undefined, 'le'), // blinding
    console.log("here5 ", this.blinding.toString());

    this.verifierAddressCircuit =  new BN(bytes.slice(95,126), undefined, 'le'), // verifierAddress
    console.log("here6");
    
    if(bytes[127]) {
      if (bytes.length < 129) {
        throw "no app data provided";
      }

      this.appData =  Array.from(bytes.slice(127,bytes.length))
    } else {
      this.appData = new Array<any>();
    }
    
    // return new Utxo(
    //   poseidon,
    //   [FEE_ASSET, MINT], // assets
    //   [toBigIntLE(bytes.slice(31,39)), toBigIntLE(bytes.slice(39,47))], // amounts
    //   toBigIntLE(bytes.slice(55,87)), // instruction Type
    //   toBigIntLE(bytes.slice(87,95)), // pool Type
    //   toBigIntLE(bytes.slice(0,31)), // blinding
    //   toBigIntLE(bytes.slice(95,127)), // verifierAddress
    //   JSON.parse(bytes.slice(127,).toString())
    // );
    return this
  }

  /**
   * Returns commitment for this UTXO
   *signature:
   * @returns {BigNumber}
   */
  getCommitment() {
      if (!this._commitment) {
        let amountHash = this.poseidon.F.toString(this.poseidon(this.amounts));

        let assetHash = this.poseidon.F.toString(this.poseidon(this.assets));
        this._commitment = this.poseidon.F.toString(this.poseidon([
            amountHash,
            this.keypair.pubkey,
            this.blinding,
            assetHash,
            this.instructionType,
            this.poolType
        ]));


      }
      return this._commitment;
  }

  /**
   * Returns nullifier for this UTXO
   *
   * @returns {BigNumber}
   */
  getNullifier() {
      if (!this._nullifier) {
          if (this.amount > 0 &&
              (this.index === undefined ||
                  this.index === null ||
                  this.keypair.privkey === undefined ||
                  this.keypair.privkey === null)) {
              throw new Error('Can not compute nullifier without utxo index or private key');
          }

          const signature = this.keypair.privkey
              ? this.keypair.sign(this.getCommitment(), this.index || 0)
              : 0;

          this._nullifier = this.poseidon.F.toString(this.poseidon([
              this.getCommitment(),
              this.index || 0,
              signature,
          ]))
      }
      return this._nullifier;
  }

  /**
   * Encrypt UTXO to recipient pubkey
   *
   * @returns {string}
   */
  // TODO: hardcode senderThrowAwayKeypair
  // TODO: add filled length
  // TODO: add actual length which is filled up with 0s
  encrypt(nonce, encryptionKeypair, senderThrowAwayKeypair) {
      console.log("at least asset missing in encrypted bytes");

      const bytes_message = this.toBytes()

      const ciphertext = box(bytes_message, nonce, encryptionKeypair.PublicKey, senderThrowAwayKeypair.secretKey);

      return ciphertext;
  }

  // TODO: add parse asset from assetIndex
  static decrypt(encryptedUtxo, nonce, senderThrowAwayPubkey, recipientEncryptionKeypair, shieldedKeypair, assets = [], POSEIDON, index) {

      const cleartext = box.open(encryptedUtxo, nonce, senderThrowAwayPubkey, recipientEncryptionKeypair.secretKey);
      if (!cleartext) {
          return [false, null];
      }
      const buf = Buffer.from(cleartext);
      // TODO: use fromBytes()
      const utxoAmount1 = new anchor.BN(Array.from(buf.slice(31, 39)).reverse());
      const utxoAmount2 = new anchor.BN(Array.from(buf.slice(39, 47)).reverse());

      const utxoBlinding = new anchor.BN( buf.slice(0, 31));

      // TODO: find a better way to make this fail since this can be a footgun
      return [
          true,
          new Utxo(POSEIDON, assets, [utxoAmount1, utxoAmount2], shieldedKeypair,"0", utxoBlinding, index)
      ];
  }

}

exports.Utxo = Utxo;
