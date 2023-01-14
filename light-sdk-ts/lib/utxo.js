"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || function (mod) {
    if (mod && mod.__esModule) return mod;
    var result = {};
    if (mod != null) for (var k in mod) if (k !== "default" && Object.prototype.hasOwnProperty.call(mod, k)) __createBinding(result, mod, k);
    __setModuleDefault(result, mod);
    return result;
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.Utxo = exports.N_ASSET_PUBKEYS = exports.N_ASSETS = exports.newNonce = void 0;
const keypair_1 = require("./keypair");
const tweetnacl_1 = __importStar(require("tweetnacl"));
const crypto = require("crypto");
const randomBN = (nbytes = 30) => new anchor.BN(crypto.randomBytes(nbytes));
exports.randomBN = randomBN;
const anchor = require("@coral-xyz/anchor");
const utils_1 = require("./utils");
const web3_js_1 = require("@solana/web3.js");
var ffjavascript = require("ffjavascript");
const { unstringifyBigInts, leInt2Buff } = ffjavascript.utils;
const anchor_1 = require("@coral-xyz/anchor");
const constants_1 = require("./constants");
const newNonce = () => tweetnacl_1.default.randomBytes(tweetnacl_1.default.box.nonceLength);
exports.newNonce = newNonce;
// TODO: move to constants
exports.N_ASSETS = 2;
exports.N_ASSET_PUBKEYS = 3;
// TODO: write test
class Utxo {
    constructor({ poseidon, 
    // TODO: reduce to one (the first will always be 0 and the third is not necessary)
    assets = [web3_js_1.SystemProgram.programId], amounts = [new anchor_1.BN("0")], keypair, // shielded pool keypair that is derived from seedphrase. OutUtxo: supply pubkey
    blinding = new anchor_1.BN(randomBN(), 31, "le"), poolType = new anchor_1.BN("0"), verifierAddress = web3_js_1.SystemProgram.programId, appData = [], index = null, }) {
        // check that blinding is 31 bytes
        blinding.toArray("le", 31);
        if (assets.length != amounts.length) {
            throw `utxo constructor: asset.length  ${assets.length}!= amount.length ${amounts.length}`;
        }
        if (assets.length > exports.N_ASSETS) {
            throw `assets.lengt ${assets.length} > N_ASSETS ${exports.N_ASSETS}`;
        }
        while (assets.length < exports.N_ASSETS) {
            assets.push(web3_js_1.SystemProgram.programId);
        }
        for (var i = 0; i < exports.N_ASSETS; i++) {
            if (amounts[i] < new anchor_1.BN(0)) {
                throw `utxo constructor: amount cannot be negative, amounts[${i}] = ${amounts[i]}`;
            }
        }
        while (amounts.length < exports.N_ASSETS) {
            amounts.push(new anchor_1.BN(0));
        }
        if (!keypair) {
            keypair = new keypair_1.Keypair({ poseidon });
        }
        // TODO: check that this does not lead to hickups since publicAmount cannot withdraw the fee asset sol
        if (assets[1].toBase58() == web3_js_1.SystemProgram.programId.toBase58()) {
            amounts[0] = amounts[0].add(amounts[1]);
            amounts[1] = new anchor_1.BN(0);
        }
        else {
        }
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
        }
        else {
            this.instructionType = new anchor_1.BN("0");
        }
        this.amounts = amounts.map((x) => {
            try {
                // check that amounts are U64
                // TODO: add test
                x.toArray("be", 8);
            }
            catch (_) {
                throw new Error("amount not u64");
            }
            return new anchor_1.BN(x.toString());
        });
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
        if (assets[1].toBase58() != web3_js_1.SystemProgram.programId.toBase58()) {
            this.assetsCircuit = [
                new anchor_1.BN(0),
                (0, utils_1.hashAndTruncateToCircuit)(this.assets[1].toBytes()),
            ];
        }
        else {
            this.assetsCircuit = [new anchor_1.BN(0), new anchor_1.BN(0)];
        }
        if (verifierAddress.toBase58() == web3_js_1.SystemProgram.programId.toBase58()) {
            this.verifierAddress = new anchor_1.BN(verifierAddress.toBytes());
            this.verifierAddressCircuit = new anchor_1.BN(0);
        }
        else {
            this.verifierAddress = new anchor_1.BN(verifierAddress.toBytes());
            this.verifierAddressCircuit = (0, utils_1.hashAndTruncateToCircuit)(verifierAddress.toBytes());
        }
    }
    toBytes() {
        //TODO: get assetIndex(this.asset[1])
        const assetIndex = (0, utils_1.getAssetIndex)(this.assets[1]);
        if (assetIndex.toString() == "-1") {
            throw new Error("Asset not found in lookup table");
        }
        // case no appData
        if (this.instructionType.toString() == "0") {
            return new Uint8Array([
                ...this.blinding.toArray("le", 31),
                ...this.amounts[0].toArray("le", 8),
                ...this.amounts[1].toArray("le", 8),
                ...new anchor_1.BN(assetIndex).toArray("le", 8),
            ]);
        }
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
    // TODO: check length to rule out parsing app utxo
    static fromBytes({ poseidon, bytes, keypair, keypairInAppDataOffset, }) {
        if (keypair) {
            const blinding = new anchor_1.BN(bytes.slice(0, 31), undefined, "le"); // blinding
            const amounts = [
                new anchor_1.BN(bytes.slice(31, 39), undefined, "le"),
                new anchor_1.BN(bytes.slice(39, 47), undefined, "le"),
            ]; // amounts
            const assets = [
                web3_js_1.SystemProgram.programId,
                (0, utils_1.fetchAssetByIdLookUp)(new anchor_1.BN(bytes.slice(47, 55), undefined, "le")),
            ]; // assets MINT
            return new Utxo({
                poseidon,
                assets,
                amounts,
                keypair,
                blinding,
            });
        }
        else {
            throw new Error("fromBytes only implemented for standard utxo");
            return new Utxo({ poseidon });
        }
        // console.log("here2");
        // this.instructionType =  BigNumber.from(leBuff2int(Uint8Array.from(bytes.slice(55,87))).toString()) // instruction Type
        // console.log("here3");
        // this.poolType =  new BN(bytes.slice(87,95), undefined, 'le'); // pool Type
        // console.log("here4");
        // console.log("here5 ", this.blinding.toString());
        // // TODO: put the verifier address into the utxo not the circuit one then I can derive the circuit verifier address
        // this.verifierAddressCircuit =  new BN(bytes.slice(95,126), undefined, 'le'), // verifierAddress
        // console.log("here6");
        // this.appData =  Array.from(bytes.slice(127,bytes.length))
        // if (keypairInAppDataOffset != undefined) {
        //   console.log("restoring keypair");
        //   console.log("bytes: ", Array.from(this.appData.slice(keypairInAppDataOffset, keypairInAppDataOffset + 32)));
        //   console.log("bytes: ", Array.from(this.appData).toString());
        //   this.keypair = new Keypair(this.poseidon, new BN(this.appData.slice(keypairInAppDataOffset, keypairInAppDataOffset + 32), undefined, 'le'))
        //   console.log("this.keypair ", this.keypair.pubkey);
        // }
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
        // return this
    }
    /**
     * Returns commitment for this UTXO
     *signature:
     * @returns {BN}
     */
    getCommitment() {
        if (!this._commitment) {
            let amountHash = this.poseidon.F.toString(this.poseidon(this.amounts));
            let assetHash = this.poseidon.F.toString(this.poseidon(this.assetsCircuit));
            // console.log("amountHash ", amountHash.toString());
            // console.log("assetHash ", assetHash.toString());
            // console.log("this.keypair.pubkey ", this.keypair.pubkey.toString());
            // console.log("this.blinding ", this.blinding.toString());
            // console.log("this.instructionType ", this.instructionType.toString());
            // console.log("this.poolType ", this.poolType.toString());
            this._commitment = this.poseidon.F.toString(this.poseidon([
                amountHash,
                this.keypair.pubkey.toString(),
                this.blinding.toString(),
                assetHash.toString(),
                this.instructionType.toString(),
                this.poolType,
                this.verifierAddressCircuit,
            ]));
        }
        return this._commitment;
    }
    /**
     * Returns nullifier for this UTXO
     *
     * @returns {BN}
     */
    getNullifier() {
        if (!this._nullifier) {
            if (
            //(this.amounts[0] > new BN(0) || this.amounts[0] > new BN(0))
            false &&
                (this.index === undefined ||
                    this.index === null ||
                    this.keypair.privkey === undefined ||
                    this.keypair.privkey === null)) {
                throw new Error("Can not compute nullifier without utxo index or private key");
            }
            const signature = this.keypair.privkey
                ? this.keypair.sign(this.getCommitment(), this.index || 0)
                : 0;
            // console.log("this.getCommitment() ", this.getCommitment());
            // console.log("this.index || 0 ", this.index || 0);
            // console.log("signature ", signature);
            this._nullifier = this.poseidon.F.toString(this.poseidon([this.getCommitment(), this.index || 0, signature]));
        }
        // console.log("this._nullifier ", this._nullifier);
        return this._nullifier;
    }
    /**
     * Encrypt UTXO to recipient pubkey
     *
     * @returns {string}
     */
    // TODO: add fill option to 128 bytes to be filled with 0s
    // TODO: add encrypt custom (app utxos with idl)
    encrypt() {
        const bytes_message = this.toBytes();
        const nonce = (0, exports.newNonce)();
        // CONSTANT_SECRET_AUTHKEY is used to minimize the number of bytes sent to the blockchain.
        // This results in poly135 being useless since the CONSTANT_SECRET_AUTHKEY is public.
        // However, ciphertext integrity is guaranteed since a hash of the ciphertext is included in a zero-knowledge proof.
        const ciphertext = (0, tweetnacl_1.box)(bytes_message, nonce, this.keypair.encryptionPublicKey, constants_1.CONSTANT_SECRET_AUTHKEY);
        return new Uint8Array([...ciphertext, ...nonce]);
    }
    // TODO: add decrypt custom (app utxos with idl)
    static decrypt({ poseidon, encBytes, keypair, }) {
        const encryptedUtxo = new Uint8Array(Array.from(encBytes.slice(0, 71)));
        const nonce = new Uint8Array(Array.from(encBytes.slice(71, 71 + 24)));
        if (keypair.encPrivateKey) {
            const cleartext = tweetnacl_1.box.open(encryptedUtxo, nonce, tweetnacl_1.default.box.keyPair.fromSecretKey(constants_1.CONSTANT_SECRET_AUTHKEY).publicKey, keypair.encPrivateKey);
            if (!cleartext) {
                return null;
            }
            const bytes = Buffer.from(cleartext);
            return Utxo.fromBytes({ poseidon, bytes, keypair });
        }
        else {
            return null;
        }
    }
}
exports.Utxo = Utxo;
exports.Utxo = Utxo;
