import { Keypair } from './keypair'
import { BigNumber, BytesLike, ethers } from 'ethers'
import nacl, { box} from 'tweetnacl'
const crypto = require('crypto');
const randomBN = (nbytes = 31) => new anchor.BN(crypto.randomBytes(nbytes));
exports.randomBN = randomBN;
const anchor = require("@coral-xyz/anchor")
import {toBufferLE, toBigIntLE} from 'bigint-buffer';
// import { FEE_ASSET, FIELD_SIZE, FIELD_SIZE_ETHERS, MINT, MINT_CIRCUIT } from './constants';
import {fetchAssetByIdLookUp, hashAndTruncateToCircuit} from './utils';
import { PublicKey, SystemProgram } from '@solana/web3.js';
// import { BN } from 'bn.js';
var ffjavascript = require('ffjavascript');
const { unstringifyBigInts, stringifyBigInts, leInt2Buff, leBuff2int } = ffjavascript.utils;

import {BN } from '@coral-xyz/anchor'
import { assert } from 'chai';
import { CONSTANT_SECRET_AUTHKEY, FEE_ASSET } from './constants';
export const newNonce = () => nacl.randomBytes(nacl.box.nonceLength);
// TODO: move to constants
export const N_ASSETS = 2;

// TODO: write test
export class Utxo {
  /** Initialize a new UTXO - unspent transaction output or input. Note, a full TX consists of 2 inputs and 2 outputs
   *
   * @param {new BN[]} amounts array UTXO amount
   * @param {new BN | new BN | number | string} blinding Blinding factor
   */

  amounts: BN[]
  assets: PublicKey[]
  assetsCircuit:BN[]
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
  poseidon: any

  constructor({
    poseidon,
    // TODO: reduce to one (the first will always be 0 and the third is not necessary)
    assets = [SystemProgram.programId],
    amounts = [new BN('0')],
    keypair, // shielded pool keypair that is derived from seedphrase. OutUtxo: supply pubkey
    blinding = new BN(randomBN()),
    poolType = new BN('0'),
    verifierAddress = SystemProgram.programId,
    appData = [],
    index = null
  }: {
    poseidon: any,
    assets: PublicKey[],
    amounts: BN[],
    keypair: Keypair, // shielded pool keypair that is derived from seedphrase. OutUtxo: supply pubkey
    blinding: BN,
    poolType: BN,
    verifierAddress: PublicKey,
    appData?: Array<any>,
    index?: any
  }) {
    if (assets.length != amounts.length) {
      throw `utxo constructor: asset.length  ${assets.length}!= amount.length ${amounts.length}`;
    }
    if (assets.length > N_ASSETS) {
      throw `assets.lengt ${assets.length} > N_ASSETS ${N_ASSETS}`;
    }

    while (assets.length < N_ASSETS) {
      assets.push(SystemProgram.programId)
    }

    for (var i= 0; i < N_ASSETS; i++) {
      if (amounts[i] < new BN(0)) {
          throw `utxo constructor: amount cannot be negative, amounts[${i}] = ${amounts[i]}`
      }
    }

    while (amounts.length < N_ASSETS) {
      amounts.push(new BN(0))
    }
    if (!keypair) {
      keypair = new Keypair({poseidon})
    }

    // TODO: check that this does not lead to hickups since publicAmount cannot withdraw the fee asset sol
    if (assets[1].toBase58() == SystemProgram.programId.toBase58()) {
      amounts[0] = amounts[0].add(amounts[1]);
      amounts[1] = new BN(0)
    } else {  }
    
    // let appDataArray = new Array<any>;
    // for (let elem in appData) {
    //   console.log(Array.from(appData[elem].toString()));
      
    //   appDataArray.push(Array.from(appData[elem]));
    // }
    
    // console.log("appDataArray.flat() ",appDataArray.flat());
    
    if (appData.length > 0) {
      // TODO: change to poseidon hash which is reproducable in circuit
      // TODO: write function which creates the instructionTypeHash
      // this.instructionType = BigNumber.from(ethers.utils.keccak256(appData).toString()).mod(FIELD_SIZE_ETHERS);
    } else {
      this.instructionType = new BN('0');
    }

    //TODO: add check that amounts are U64
    this.amounts = amounts.map((x) => new BN(x.toString()));
    this.blinding = blinding;
    this.keypair = keypair;
    this.index = index;
    this.assets = assets;
    this._commitment = null;
    this._nullifier = null;
    this.poseidon = poseidon;
    this.appData = appData;
    this.poolType = poolType;
    
    // TODO: make variable length
    // TODO: evaluate whether to hashAndTruncate feeAsset as well
    if (assets[1].toBase58() != SystemProgram.programId.toBase58()) {
      this.assetsCircuit = [new BN(SystemProgram.programId.toBytes()), hashAndTruncateToCircuit(this.assets[1].toBytes())]    
    } else {
      this.assetsCircuit = [new BN(SystemProgram.programId.toBytes()), new BN(0)];
    }
    console.log("assetsCircuit ", this.assetsCircuit);
    
    if (verifierAddress.toBase58() == SystemProgram.programId.toBase58()) {
      this.verifierAddress = new BN(verifierAddress.toBytes());
      this.verifierAddressCircuit = new BN(0)
    } else {
      this.verifierAddress = new BN(verifierAddress.toBytes());
      this.verifierAddressCircuit = hashAndTruncateToCircuit(verifierAddress.toBytes());
    }
    
  }

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
  // TODO: find a better solution to get the private key in
  fromBytes(bytes: Uint8Array, keypairInAppDataOffset?: number) {
    console.log("here");
    
    this.blinding =  new BN(bytes.slice(0,31), undefined, 'le'), // blinding

    console.log("here1");
    console.log("new BN(bytes.slice(47,55) ", new BN(bytes.slice(47,55), undefined, 'le'));
    
    this.amounts =  [new BN(bytes.slice(31,39), undefined, 'le'), new BN(bytes.slice(39,47), undefined, 'le'), new BN(0)] // amounts
    this.assets = [SystemProgram.programId, fetchAssetByIdLookUp({assetIndex: new BN(bytes.slice(47,55), undefined, 'le')}), 0] // assets MINT
    console.log(this.assets[1]);
    console.log(this.assets[1].toBytes());

    this.assetsCircuit = [FEE_ASSET, hashAndTruncateToCircuit(Uint8Array.from(this.assets[1].toBuffer())), new BN(0)]

    console.log("here2");
    this.instructionType =  BigNumber.from(leBuff2int(Uint8Array.from(bytes.slice(55,87))).toString()) // instruction Type
    console.log("here3");
    this.poolType =  new BN(bytes.slice(87,95), undefined, 'le'); // pool Type
    console.log("here4");
    console.log("here5 ", this.blinding.toString());

    // TODO: put the verifier address into the utxo not the circuit one then I can derive the circuit verifier address
    this.verifierAddressCircuit =  new BN(bytes.slice(95,126), undefined, 'le'), // verifierAddress
    console.log("here6");
    this.appData =  Array.from(bytes.slice(127,bytes.length))
    if (keypairInAppDataOffset != undefined) {
      console.log("restoring keypair");
      console.log("bytes: ", Array.from(this.appData.slice(keypairInAppDataOffset, keypairInAppDataOffset + 32)));
      console.log("bytes: ", Array.from(this.appData).toString());

      this.keypair = new Keypair(this.poseidon, new BN(this.appData.slice(keypairInAppDataOffset, keypairInAppDataOffset + 32), undefined, 'le'))
      console.log("this.keypair ", this.keypair.pubkey);
      
    }

    // if(bytes[127]) {
    //   if (bytes.length < 129) {
    //     throw "no app data provided";
    //   }

    //   this.appData =  Array.from(bytes.slice(127,bytes.length))
    // } else {
    //   this.appData = new Array<any>();
    // }
    
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
        console.log("this.amounts ", this.amounts);
        
        let amountHash = this.poseidon.F.toString(this.poseidon(this.amounts));
        console.log("this.assetsCircuit ", this.assetsCircuit);

        let assetHash = this.poseidon.F.toString(this.poseidon(this.assetsCircuit));

        console.log("amountHash ", amountHash.toString());
        console.log("assetHash ", assetHash.toString());

        console.log("this.keypair.pubkey ", this.keypair.pubkey.toString());
        console.log("this.blinding ", this.blinding.toString());
        console.log("this.instructionType ", this.instructionType.toString());
        // console.log("this.poolType ", this.poolType.toString());

        this._commitment = this.poseidon.F.toString(this.poseidon([
            amountHash,
            this.keypair.pubkey.toString(),
            this.blinding.toString(),
            assetHash.toString(),
            this.instructionType.toString(),
            // this.poolType,
            // this.verifierPubkey
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
          console.log("this.getCommitment() ", this.getCommitment());
          console.log("this.index || 0 ", this.index || 0);
          console.log("signature ", signature);

          this._nullifier = this.poseidon.F.toString(this.poseidon([
              this.getCommitment(),
              this.index || 0,
              signature,
          ]))
      }
      console.log("this._nullifier ", this._nullifier);
      
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
  // TODO: add encrypt custom (app utxos with idl)
  encrypt() {
    console.log("at least asset missing in encrypted bytes");
    
    const bytes_message = this.toBytes()
    const nonce = newNonce();
    // CONSTANT_SECRET_AUTHKEY is used to minimize the number of bytes sent to the blockchain.
    // This results in poly135 being useless since the CONSTANT_SECRET_AUTHKEY is public.
    // However, ciphertext integrity is guaranteed since a hash of the ciphertext is included in a zero-knowledge proof.
    const ciphertext = box(bytes_message, nonce, this.keypair.encryptionPublicKey, CONSTANT_SECRET_AUTHKEY);

    return ciphertext;
  }

  // TODO: add parse asset from assetIndex
  // TODO: add decrypt custom (app utxos with idl)
  static decrypt(encryptedUtxo: Uint8Array, nonce:Uint8Array, senderThrowAwayPubkey:Uint8Array, recipientEncryptionKeypair, shieldedKeypair, assets = [], POSEIDON, index) {
    console.log("recipientEncryptionKeypair ", recipientEncryptionKeypair);
    console.log("encryptedUtxo ", encryptedUtxo);
    console.log("nonce ", nonce);
    console.log("senderThrowAwayPubkey ", senderThrowAwayPubkey);

    
    const cleartext = box.open(encryptedUtxo, nonce, senderThrowAwayPubkey, recipientEncryptionKeypair.secretKey);
    if (!cleartext) {
        return [false, null];
    }
    const buf = Buffer.from(cleartext);
    // TODO: use fromBytes()
    const utxoAmount1 = new anchor.BN(Array.from(buf.slice(31, 39)),undefined, 'le');
    const utxoAmount2 = new anchor.BN(Array.from(buf.slice(39, 47)).reverse());

    const utxoBlinding = new anchor.BN( buf.slice(0, 31));

    // TODO: find a better way to make this fail since this can be a footgun
    return [
        true,
        new Utxo({poseidon: POSEIDON, assets, amounts: [utxoAmount1, utxoAmount2], keypair: shieldedKeypair, blinding: utxoBlinding, index})
    ];
  }

}

exports.Utxo = Utxo;
