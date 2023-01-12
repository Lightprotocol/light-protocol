"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.Utxo = void 0;
const keypair_1 = require("./keypair");
const ethers_1 = require("ethers");
const tweetnacl_1 = require("tweetnacl");
const crypto = require("crypto");
const randomBN = (nbytes = 31) => new anchor.BN(crypto.randomBytes(nbytes));
exports.randomBN = randomBN;
const anchor = require("@project-serum/anchor");
const constants_1 = require("./constants");
const utils_1 = require("./utils");
const web3_js_1 = require("@solana/web3.js");
// import { BN } from 'bn.js';
var ffjavascript = require("ffjavascript");
const { unstringifyBigInts, stringifyBigInts, leInt2Buff, leBuff2int } =
  ffjavascript.utils;
const N_ASSETS = 3;
const anchor_1 = require("@project-serum/anchor");
// TODO: write test
class Utxo {
  constructor({
    poseidon,
    // TODO: reduce to one (the first will always be 0 and the third is not necessary)
    assets = [
      web3_js_1.SystemProgram.programId,
      web3_js_1.SystemProgram.programId,
      web3_js_1.SystemProgram.programId,
    ],
    amounts = [
      new anchor_1.BN("0"),
      new anchor_1.BN("0"),
      new anchor_1.BN("0"),
    ],
    keypair, // shielded pool keypair that is derived from seedphrase. OutUtxo: supply pubkey
    blinding = new anchor_1.BN(randomBN()),
    poolType = new anchor_1.BN("0"),
    verifierAddress = web3_js_1.SystemProgram.programId,
    appData = [],
    index = null,
  }) {
    if (assets.length != amounts.length) {
      throw `utxo constructor: asset.length  ${assets.length}!= amount.length ${amounts.length}`;
    }
    while (assets.length < N_ASSETS) {
      assets.push(new anchor.BN(0));
    }
    for (var i = 0; i < N_ASSETS; i++) {
      if (amounts[i] < 0) {
        throw `utxo constructor: amount cannot be negative, amounts[${i}] = ${amounts[i]}`;
      }
    }
    while (amounts.length < N_ASSETS) {
      amounts.push(0);
    }
    if (!keypair) {
      keypair = new keypair_1.Keypair(poseidon);
    }
    if (assets[1].toBase58() == web3_js_1.SystemProgram.programId.toBase58()) {
      amounts[0] = amounts[0].add(amounts[1]);
      amounts[1] = new anchor_1.BN(0);
    } else {
    }
    // let appDataArray = new Array<any>;
    // for (let elem in appData) {
    //   console.log(Array.from(appData[elem].toString()));
    //   appDataArray.push(Array.from(appData[elem]));
    // }
    // console.log("appDataArray.flat() ",appDataArray.flat());
    if (appData.length > 0) {
      // TODO: change to poseidon hash which is reproducable in circuit
      this.instructionType = ethers_1.BigNumber.from(
        ethers_1.ethers.utils.keccak256(appData).toString()
      ).mod(constants_1.FIELD_SIZE_ETHERS);
    } else {
      this.instructionType = new anchor_1.BN("0");
    }
    //TODO: add check that amounts are U64
    this.amounts = amounts.map((x) => new anchor_1.BN(x.toString()));
    this.blinding = blinding;
    this.keypair = keypair;
    this.index = index;
    console.log("assets ", assets);
    this.assets = assets;
    // TODO: make variable length
    if (assets[1].toBase58() != web3_js_1.SystemProgram.programId.toBase58()) {
      this.assetsCircuit = [
        constants_1.FEE_ASSET,
        (0, utils_1.hashAndTruncateToCircuit)(this.assets[1].toBytes()),
        new anchor_1.BN(0),
      ];
      // console.log("this.assetsCircuit ", this.assetsCircuit);
      // console.log("MINT_CIRCUIT ", MINT_CIRCUIT);
      // assert(MINT_CIRCUIT.toString() == this.assetsCircuit[1].toString(), "MINT_CIRCUIT != this.assetsCircuit[1]");
    } else {
      this.assetsCircuit = [
        constants_1.FEE_ASSET,
        new anchor_1.BN(0),
        new anchor_1.BN(0),
      ];
    }
    console.log("assetsCircuit ", this.assetsCircuit);
    this._commitment = null;
    this._nullifier = null;
    this.poseidon = poseidon;
    this.appData = appData;
    this.poolType = poolType;
    // TODO: add hashAndTruncateToCircuit for verifierAddress
    if (
      verifierAddress.toBase58() == web3_js_1.SystemProgram.programId.toBase58()
    ) {
      this.verifierAddress = verifierAddress;
      this.verifierAddressCircuit = new anchor_1.BN(0);
    } else {
      this.verifierAddress = verifierAddress;
      this.verifierAddressCircuit = (0, utils_1.hashAndTruncateToCircuit)(
        verifierAddress.toBytes()
      );
    }
  }
  toBytes() {
    //TODO: get assetIndex(this.asset[1])
    let assetIndex = new anchor_1.BN("0");
    // case no appData
    if (this.instructionType.toString() == "0") {
      console.log("isntruction type is 0");
      return new Uint8Array([
        ...this.blinding.toBuffer(),
        ...this.amounts[0].toArray("le", 8),
        ...this.amounts[1].toArray("le", 8),
        ...new anchor_1.BN(assetIndex).toArray("le", 8),
      ]);
    }
    console.log("this.instructionType", this.instructionType);
    return new Uint8Array([
      ...this.blinding.toArray("le", 31),
      ...this.amounts[0].toArray("le", 8),
      ...this.amounts[1].toArray("le", 8),
      ...assetIndex.toArray("le", 8),
      ...leInt2Buff(unstringifyBigInts(this.instructionType.toString()), 32),
      ...this.poolType.toArray("le", 8),
      ...this.verifierAddressCircuit.toArray("le", 31),
      ...new Array(1),
      ...this.appData,
    ]);
  }
  // take a decrypted byteArray as input
  // TODO: make robust and versatile for any combination of filled in fields or not
  // TODO: find a better solution to get the private key in
  fromBytes(bytes, keypairInAppDataOffset) {
    console.log("here");
    (this.blinding = new anchor_1.BN(bytes.slice(0, 31), undefined, "le")), // blinding
      console.log("here1");
    console.log(
      "new BN(bytes.slice(47,55) ",
      new anchor_1.BN(bytes.slice(47, 55), undefined, "le")
    );
    this.amounts = [
      new anchor_1.BN(bytes.slice(31, 39), undefined, "le"),
      new anchor_1.BN(bytes.slice(39, 47), undefined, "le"),
      new anchor_1.BN(0),
    ]; // amounts
    this.assets = [
      constants_1.FEE_ASSET,
      (0, utils_1.fetchAssetByIdLookUp)({
        assetIndex: new anchor_1.BN(bytes.slice(47, 55), undefined, "le"),
      }),
      0,
    ]; // assets MINT
    console.log(this.assets[1]);
    console.log(this.assets[1].toBytes());
    this.assetsCircuit = [
      constants_1.FEE_ASSET,
      (0, utils_1.hashAndTruncateToCircuit)(
        Uint8Array.from(this.assets[1].toBuffer())
      ),
      new anchor_1.BN(0),
    ];
    console.log("here2");
    this.instructionType = ethers_1.BigNumber.from(
      leBuff2int(Uint8Array.from(bytes.slice(55, 87))).toString()
    ); // instruction Type
    console.log("here3");
    this.poolType = new anchor_1.BN(bytes.slice(87, 95), undefined, "le"); // pool Type
    console.log("here4");
    console.log("here5 ", this.blinding.toString());
    // TODO: put the verifier address into the utxo not the circuit one then I can derive the circuit verifier address
    (this.verifierAddressCircuit = new anchor_1.BN(
      bytes.slice(95, 126),
      undefined,
      "le"
    )), // verifierAddress
      console.log("here6");
    this.appData = Array.from(bytes.slice(127, bytes.length));
    if (keypairInAppDataOffset != undefined) {
      console.log("restoring keypair");
      console.log(
        "bytes: ",
        Array.from(
          this.appData.slice(
            keypairInAppDataOffset,
            keypairInAppDataOffset + 32
          )
        )
      );
      console.log("bytes: ", Array.from(this.appData).toString());
      this.keypair = new keypair_1.Keypair(
        this.poseidon,
        new anchor_1.BN(
          this.appData.slice(
            keypairInAppDataOffset,
            keypairInAppDataOffset + 32
          ),
          undefined,
          "le"
        )
      );
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
    return this;
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
      let assetHash = this.poseidon.F.toString(
        this.poseidon(this.assetsCircuit)
      );
      console.log("amountHash ", amountHash);
      console.log("assetHash ", assetHash);
      console.log("this.keypair.pubkey ", this.keypair.pubkey);
      console.log("this.blinding ", this.blinding);
      console.log("this.instructionType ", this.instructionType);
      console.log("this.poolType ", this.poolType);
      this._commitment = this.poseidon.F.toString(
        this.poseidon([
          amountHash,
          this.keypair.pubkey,
          this.blinding,
          assetHash,
          this.instructionType,
          // this.poolType
        ])
      );
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
      if (
        this.amount > 0 &&
        (this.index === undefined ||
          this.index === null ||
          this.keypair.privkey === undefined ||
          this.keypair.privkey === null)
      ) {
        throw new Error(
          "Can not compute nullifier without utxo index or private key"
        );
      }
      const signature = this.keypair.privkey
        ? this.keypair.sign(this.getCommitment(), this.index || 0)
        : 0;
      console.log("this.getCommitment() ", this.getCommitment());
      console.log("this.index || 0 ", this.index || 0);
      console.log("signature ", signature);
      this._nullifier = this.poseidon.F.toString(
        this.poseidon([this.getCommitment(), this.index || 0, signature])
      );
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
  encrypt(nonce, encryptionKeypair, senderThrowAwayKeypair) {
    console.log("at least asset missing in encrypted bytes");
    const bytes_message = this.toBytes();
    const ciphertext = (0, tweetnacl_1.box)(
      bytes_message,
      nonce,
      encryptionKeypair.PublicKey,
      senderThrowAwayKeypair.secretKey
    );
    return ciphertext;
  }
  // TODO: add parse asset from assetIndex
  static decrypt(
    encryptedUtxo,
    nonce,
    senderThrowAwayPubkey,
    recipientEncryptionKeypair,
    shieldedKeypair,
    assets = [],
    POSEIDON,
    index
  ) {
    console.log("recipientEncryptionKeypair ", recipientEncryptionKeypair);
    console.log("encryptedUtxo ", encryptedUtxo);
    console.log("nonce ", nonce);
    console.log("senderThrowAwayPubkey ", senderThrowAwayPubkey);
    const cleartext = tweetnacl_1.box.open(
      encryptedUtxo,
      nonce,
      senderThrowAwayPubkey,
      recipientEncryptionKeypair.secretKey
    );
    if (!cleartext) {
      return [false, null];
    }
    const buf = Buffer.from(cleartext);
    // TODO: use fromBytes()
    const utxoAmount1 = new anchor.BN(Array.from(buf.slice(31, 39)).reverse());
    const utxoAmount2 = new anchor.BN(Array.from(buf.slice(39, 47)).reverse());
    const utxoBlinding = new anchor.BN(buf.slice(0, 31));
    // TODO: find a better way to make this fail since this can be a footgun
    return [
      true,
      new Utxo({
        poseidon: POSEIDON,
        assets,
        amounts: [utxoAmount1, utxoAmount2],
        keypair: shieldedKeypair,
        blinding: utxoBlinding,
        index,
      }),
    ];
  }
}
exports.Utxo = Utxo;
exports.Utxo = Utxo;
