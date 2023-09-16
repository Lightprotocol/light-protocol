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
const tweetnacl_1 = __importStar(require("tweetnacl"));
const randomBN = (nbytes = 30) => {
    return new anchor.BN(tweetnacl_1.default.randomBytes(nbytes));
};
const aes_1 = require("ethereum-cryptography/aes");
const { sha3_256 } = require("@noble/hashes/sha3");
exports.randomBN = randomBN;
const anchor = require("@coral-xyz/anchor");
const web3_js_1 = require("@solana/web3.js");
var ffjavascript = require("ffjavascript");
const { unstringifyBigInts, leInt2Buff } = ffjavascript.utils;
const anchor_1 = require("@coral-xyz/anchor");
const chai_1 = require("chai");
const index_1 = require("./index");
const bytes_1 = require("@coral-xyz/anchor/dist/cjs/utils/bytes");
const newNonce = () => tweetnacl_1.default.randomBytes(tweetnacl_1.default.box.nonceLength);
exports.newNonce = newNonce;
// TODO: move to constants
exports.N_ASSETS = 2;
exports.N_ASSET_PUBKEYS = 3;
// TODO: Idl support for U256
// TODO: add static createSolUtxo()
// TODO: remove account as attribute and from constructor, replace with shieldedPublicKey
class Utxo {
    /**
     * @description Initialize a new utxo - unspent transaction output or input. Note, a full TX consists of 2 inputs and 2 outputs
     *
     * @param {BN[]} amounts array of utxo amounts, amounts[0] is the sol amount amounts[1] is the spl amount
     * @param {PublicKey[]} assets  array of utxo assets, assets[0] is the sol asset assets[1] is the spl asset
     * @param {BN} blinding Blinding factor, a 31 bytes big number, to add randomness to the commitment hash.
     * @param {Account} account the account owning the utxo.
     * @param {index} index? the index of the utxo's commitment hash in the Merkle tree.
     * @param {Array<any>} appData application data of app utxos not provided for normal utxos.
     * @param {PublicKey} verifierAddress the solana address of the verifier, SystemProgramId/BN(0) for system verifiers.
     * @param {BN} appDataHash is the poseidon hash of app utxo data. This compresses the app data and ties it to the app utxo.
     * @param {any} poseidon poseidon hasher instance.
     * @param {boolean} includeAppData flag whether to include app data when serializing utxo to bytes.
     * @param {function} appDataFromBytesFn function to deserialize appData from bytes.
     * @param {appData} appData array of application data, is used to compute the instructionDataHash.
     */
    constructor({ poseidon, 
    // TODO: reduce to one (the first will always be 0 and the third is not necessary)
    assets = [web3_js_1.SystemProgram.programId], amounts = [index_1.BN_0], account, blinding = new anchor_1.BN(randomBN(), 31, "be"), poolType = index_1.BN_0, verifierAddress = web3_js_1.SystemProgram.programId, index, appDataHash, appData, appDataIdl, includeAppData = true, assetLookupTable, verifierProgramLookupTable, }) {
        this.assetsCircuit = [];
        if (!blinding.eq(blinding.mod(index_1.FIELD_SIZE))) {
            throw new index_1.UtxoError(index_1.UtxoErrorCode.BLINDING_EXCEEDS_FIELD_SIZE, "constructor", `Blinding ${blinding}, exceeds field size.`);
        }
        if (assets.length != amounts.length) {
            throw new index_1.UtxoError(index_1.UtxoErrorCode.INVALID_ASSET_OR_AMOUNTS_LENGTH, "constructor", `Length mismatch assets: ${assets.length} != amounts: ${amounts.length}`);
        }
        if (assets.length > exports.N_ASSETS) {
            throw new index_1.UtxoError(index_1.UtxoErrorCode.EXCEEDED_MAX_ASSETS, "constructor", `assets.length ${assets.length} > N_ASSETS ${exports.N_ASSETS}`);
        }
        if (assets.findIndex((asset) => !asset) !== -1) {
            throw new index_1.UtxoError(index_1.UtxoErrorCode.ASSET_UNDEFINED, "constructor", `asset in index ${index} is undefined. All assets: ${assets}`);
        }
        if (assets.findIndex((asset) => !asset) !== -1) {
            throw new index_1.UtxoError(index_1.UtxoErrorCode.ASSET_UNDEFINED, "constructor", `asset in index ${index} is undefined. All assets: ${assets}`);
        }
        while (assets.length < exports.N_ASSETS) {
            assets.push(web3_js_1.SystemProgram.programId);
        }
        let i = 0;
        while (i < exports.N_ASSETS) {
            const amount = amounts[i];
            if (amount?.lt?.(index_1.BN_0)) {
                throw new index_1.UtxoError(index_1.UtxoErrorCode.NEGATIVE_AMOUNT, "constructor", `amount cannot be negative, amounts[${i}] = ${amount ?? "undefined"}`);
            }
            i++;
        }
        while (amounts.length < exports.N_ASSETS) {
            amounts.push(index_1.BN_0);
        }
        // TODO: check that this does not lead to hiccups since publicAmountSpl cannot withdraw the fee asset sol
        if (assets[1].toBase58() == web3_js_1.SystemProgram.programId.toBase58()) {
            amounts[0] = amounts[0].add(amounts[1]);
            amounts[1] = index_1.BN_0;
        }
        // checks that amounts are U64
        this.amounts = amounts.map((x) => {
            try {
                x.toArray("be", 8);
            }
            catch (_) {
                throw new index_1.UtxoError(index_1.UtxoErrorCode.NOT_U64, "constructor", `amount ${x} not a u64`);
            }
            return new anchor_1.BN(x.toString());
        });
        this.account = account || new index_1.Account({ poseidon });
        this.blinding = blinding;
        this.index = index;
        this.assets = assets;
        this.appData = appData;
        this.poolType = poolType;
        this.includeAppData = includeAppData;
        this.transactionVersion = "0";
        if (assets[1].toBase58() === web3_js_1.SystemProgram.programId.toBase58() &&
            !amounts[1].isZero()) {
            throw new index_1.UtxoError(index_1.UtxoErrorCode.POSITIVE_AMOUNT, "constructor", `spl amount cannot be positive, amounts[1] = ${amounts[1] ?? "undefined"}`);
        }
        // TODO: make variable length
        else if (assets[1].toBase58() != web3_js_1.SystemProgram.programId.toBase58()) {
            this.assetsCircuit = [
                (0, index_1.hashAndTruncateToCircuit)(web3_js_1.SystemProgram.programId.toBytes()),
                (0, index_1.hashAndTruncateToCircuit)(this.assets[1].toBytes()),
            ];
        }
        else if (this.amounts[0].isZero()) {
            this.assetsCircuit = [index_1.BN_0, index_1.BN_0];
        }
        // else if (!this.amounts[0].isZero()) {
        //   throw new UtxoError(
        //     UtxoErrorCode.NON_ZERO_AMOUNT,
        //     "constructor",
        //     `amount not zero, amounts[0] = ${this.amounts[0] ?? "undefined"}`,
        //   );
        // }
        else {
            this.assetsCircuit = [
                (0, index_1.hashAndTruncateToCircuit)(web3_js_1.SystemProgram.programId.toBytes()),
                index_1.BN_0,
            ];
        }
        if (verifierAddress.toBase58() == web3_js_1.SystemProgram.programId.toBase58()) {
            this.verifierAddress = verifierAddress;
            this.verifierAddressCircuit = index_1.BN_0;
            this.verifierProgramIndex = index_1.BN_0;
        }
        else {
            this.verifierAddress = verifierAddress;
            this.verifierAddressCircuit = (0, index_1.hashAndTruncateToCircuit)(verifierAddress.toBytes());
            this.verifierProgramIndex = new anchor_1.BN(verifierProgramLookupTable.findIndex((verifierAddress) => {
                return verifierAddress === this.verifierAddress.toBase58();
            }));
            if (this.verifierProgramIndex.isNeg())
                throw new index_1.UtxoError(index_1.UtxoErrorCode.VERIFIER_INDEX_NOT_FOUND, "constructor", `verifier pubkey ${this.verifierAddress}, not found in lookup table`);
        }
        this.splAssetIndex = (0, index_1.getAssetIndex)(this.assets[1], assetLookupTable);
        if (this.splAssetIndex.isNeg())
            throw new index_1.UtxoError(index_1.UtxoErrorCode.ASSET_NOT_FOUND, "constructor", `asset pubkey ${this.assets[1]}, not found in lookup table`);
        // if appDataBytes parse appData from bytes
        if (appData) {
            if (!appDataIdl)
                throw new index_1.UtxoError(index_1.UtxoErrorCode.APP_DATA_IDL_UNDEFINED, "constructor", "");
            if (!appDataIdl.accounts)
                throw new index_1.UtxoError(index_1.UtxoErrorCode.APP_DATA_IDL_DOES_NOT_HAVE_ACCOUNTS, "APP_DATA_IDL_DOES_NOT_HAVE_ACCOUNTS");
            let i = appDataIdl.accounts.findIndex((acc) => {
                return acc.name === "utxo";
            });
            if (i === -1)
                throw new index_1.UtxoError(index_1.UtxoErrorCode.UTXO_APP_DATA_NOT_FOUND_IN_IDL, "constructor");
            // TODO: add inputs type check
            // TODO: unify with Prover.ts
            // perform type check that appData has all the attributes
            const checkAppData = (appData, idl) => {
                const circuitName = "utxoAppData";
                const circuitIdlObject = idl.accounts.find((account) => account.name === circuitName);
                if (!circuitIdlObject) {
                    throw new Error(`${`${circuitName}`} does not exist in anchor idl`);
                }
                const fieldNames = circuitIdlObject.type.fields.map((field) => field.name);
                const inputKeys = [];
                fieldNames.forEach((fieldName) => {
                    inputKeys.push(fieldName);
                });
                let inputsObject = {};
                inputKeys.forEach((key) => {
                    inputsObject[key] = appData[key];
                    if (!inputsObject[key])
                        throw new Error(`Missing input --> ${key.toString()} in circuit ==> ${circuitName}`);
                });
            };
            checkAppData(appData, appDataIdl);
            let hashArray = [];
            for (var attribute in appData) {
                hashArray.push(appData[attribute]);
            }
            hashArray = hashArray.flat();
            if (hashArray.length > 16) {
                throw new index_1.UtxoError(index_1.UtxoErrorCode.INVALID_APP_DATA, "constructor", "appData length exceeds 16");
            }
            this.appDataHash = new anchor_1.BN(leInt2Buff(unstringifyBigInts(poseidon.F.toString(poseidon(hashArray))), 32), undefined, "le");
            if (appDataHash && appDataHash.toString() !== this.appDataHash.toString())
                throw new index_1.UtxoError(index_1.UtxoErrorCode.INVALID_APP_DATA, "constructor", "appDataHash and appData are inconsistent, appData produced a different hash than appDataHash");
            this.appData = appData;
            this.appDataIdl = appDataIdl;
        }
        else if (appDataHash) {
            this.appDataHash = appDataHash;
        }
        else {
            this.appDataHash = index_1.BN_0;
        }
    }
    /**
     * @description Parses a utxo to bytes.
     * @returns {Uint8Array}
     */
    async toBytes(compressed = false) {
        let serializeObject = {
            ...this,
            accountShieldedPublicKey: this.account.pubkey,
            accountEncryptionPublicKey: this.account.encryptionKeypair.publicKey,
            verifierAddressIndex: this.verifierProgramIndex,
        };
        let serializedData;
        if (!this.appDataIdl || !this.includeAppData) {
            let coder = new anchor_1.BorshAccountsCoder(index_1.IDL_VERIFIER_PROGRAM_ZERO);
            serializedData = await coder.encode("utxo", serializeObject);
        }
        else if (this.appDataIdl) {
            let coder = new anchor_1.BorshAccountsCoder(this.appDataIdl);
            serializeObject = {
                ...serializeObject,
                ...this.appData,
                verifierAddressIndex: this.verifierProgramIndex,
            };
            serializedData = await coder.encode("utxo", serializeObject);
        }
        else {
            throw new index_1.UtxoError(index_1.UtxoErrorCode.APP_DATA_IDL_UNDEFINED, "constructor", "Should include app data but no appDataIdl provided");
        }
        // Compressed serialization does not store the account since for an encrypted utxo
        // we assume that the user who is able to decrypt the utxo knows the corresponding account.
        return compressed
            ? serializedData.subarray(0, index_1.COMPRESSED_UTXO_BYTES_LENGTH)
            : serializedData;
    }
    /**
     * @description Parses a utxo from bytes.
     * @param poseidon poseidon hasher instance
     * @param bytes byte array of a serialized utxo
     * @param account account of the utxo
     * @param appDataFromBytesFn function to parse app data from bytes
     * @param includeAppData whether to include app data when encrypting or not
     * @returns {Utxo}
     */
    // TODO: make robust and versatile for any combination of filled in fields or not
    // TODO: find a better solution to get the private key in
    // TODO: take array of idls as input and select the idl with the correct verifierIndex
    static fromBytes({ poseidon, bytes, account, includeAppData = true, index, appDataIdl, verifierAddress, assetLookupTable, verifierProgramLookupTable, }) {
        // assumes it is compressed and adds 64 0 bytes padding
        if (bytes.length === index_1.COMPRESSED_UTXO_BYTES_LENGTH) {
            let tmp = Uint8Array.from([...Array.from(bytes)]);
            bytes = Buffer.from([
                ...tmp,
                ...new Uint8Array(index_1.UNCOMPRESSED_UTXO_BYTES_LENGTH - index_1.COMPRESSED_UTXO_BYTES_LENGTH).fill(0),
            ]);
            includeAppData = false;
            if (!account)
                throw new index_1.UtxoError(index_1.CreateUtxoErrorCode.ACCOUNT_UNDEFINED, "fromBytes", "For deserializing a compressed utxo an account is required.");
        }
        let decodedUtxoData;
        let assets;
        let appData = undefined;
        // TODO: should I check whether an account is passed or not?
        if (!appDataIdl) {
            let coder = new anchor_1.BorshAccountsCoder(index_1.IDL_VERIFIER_PROGRAM_ZERO);
            decodedUtxoData = coder.decode("utxo", bytes);
        }
        else {
            if (!appDataIdl.accounts)
                throw new index_1.UtxoError(index_1.UtxoErrorCode.APP_DATA_IDL_DOES_NOT_HAVE_ACCOUNTS, "fromBytes");
            let coder = new anchor_1.BorshAccountsCoder(appDataIdl);
            decodedUtxoData = coder.decode("utxo", bytes);
            appData = (0, index_1.createAccountObject)(decodedUtxoData, appDataIdl.accounts, "utxoAppData");
        }
        assets = [
            web3_js_1.SystemProgram.programId,
            (0, index_1.fetchAssetByIdLookUp)(decodedUtxoData.splAssetIndex, assetLookupTable),
        ];
        verifierAddress = (0, index_1.fetchVerifierByIdLookUp)(decodedUtxoData.verifierAddressIndex, verifierProgramLookupTable);
        if (!account) {
            let concatPublicKey = bytes_1.bs58.encode(new Uint8Array([
                ...decodedUtxoData.accountShieldedPublicKey.toArray("be", 32),
                ...decodedUtxoData.accountEncryptionPublicKey,
            ]));
            account = index_1.Account.fromPubkey(concatPublicKey, poseidon);
        }
        return new Utxo({
            assets,
            account,
            index,
            poseidon,
            appDataIdl,
            includeAppData,
            appData,
            verifierAddress,
            ...decodedUtxoData,
            verifierProgramLookupTable,
            assetLookupTable,
        });
    }
    /**
     * @description Returns commitment for this utxo
     * @description PoseidonHash(amountHash, shieldedPubkey, blinding, assetHash, appDataHash, poolType, verifierAddressCircuit)
     * @returns {string}
     */
    getCommitment(poseidon) {
        if (!this._commitment) {
            let amountHash = poseidon.F.toString(poseidon(this.amounts));
            let assetHash = poseidon.F.toString(poseidon(this.assetsCircuit.map((x) => x.toString())));
            let publicKey;
            if (this.account) {
                publicKey = this.account.pubkey;
            }
            else {
                throw new index_1.UtxoError(index_1.CreateUtxoErrorCode.ACCOUNT_UNDEFINED, "getCommitment", "Neither Account nor shieldedPublicKey was provided");
            }
            // console.log("this.assetsCircuit ", this.assetsCircuit);
            // console.log("amountHash ", amountHash.toString());
            // console.log("this.keypair.pubkey ", this.account.pubkey.toString());
            // console.log("this.blinding ", this.blinding.toString());
            // console.log("assetHash ", assetHash.toString());
            // console.log("this.appDataHash ", this.appDataHash.toString());
            // console.log("this.poolType ", this.poolType.toString());
            let commitment = poseidon.F.toString(poseidon([
                this.transactionVersion,
                amountHash,
                publicKey.toString(),
                this.blinding.toString(),
                assetHash.toString(),
                this.appDataHash.toString(),
                this.poolType,
                this.verifierAddressCircuit,
            ]));
            this._commitment = commitment;
            return this._commitment;
        }
        else {
            return this._commitment;
        }
    }
    /**
     * @description Computes the nullifier for this utxo.
     * @description PoseidonHash(commitment, index, signature)
     * @param {number} index Merkle tree index of the utxo commitment (Optional)
     *
     * @returns {string}
     */
    getNullifier(poseidon, index) {
        if (this.index === undefined) {
            if (index) {
                this.index = index;
            }
            else if (this.amounts[0].isZero() && this.amounts[1].isZero()) {
                this.index = 0;
            }
            else {
                throw new index_1.UtxoError(index_1.UtxoErrorCode.INDEX_NOT_PROVIDED, "getNullifier", "The index of a UTXO in the Merkle tree is required to compute the nullifier hash.");
            }
        }
        if ((!this.amounts[0].eq(index_1.BN_0) || !this.amounts[1].eq(index_1.BN_0)) &&
            this.account.privkey.toString() === "0") {
            throw new index_1.UtxoError(index_1.UtxoErrorCode.ACCOUNT_HAS_NO_PRIVKEY, "getNullifier", "The index of an utxo in the merkle tree is required to compute the nullifier hash.");
        }
        if (!this._nullifier) {
            const signature = this.account.privkey
                ? this.account.sign(poseidon, this.getCommitment(poseidon), this.index || 0)
                : 0;
            // console.log("this.getCommitment() ", this.getCommitment());
            // console.log("this.index || 0 ", this.index || 0);
            // console.log("signature ", signature);
            this._nullifier = poseidon.F.toString(poseidon([this.getCommitment(poseidon), this.index || 0, signature]));
        }
        return this._nullifier;
    }
    /**
     * @description Encrypts the utxo to the utxo's accounts public key with nacl.box.
     *
     * @returns {Uint8Array} with the first 24 bytes being the nonce
     */
    async encrypt(poseidon, merkleTreePdaPublicKey, compressed = true) {
        const bytes_message = await this.toBytes(compressed);
        const commitment = new anchor_1.BN(this.getCommitment(poseidon)).toArrayLike(Buffer, "le", 32);
        var nonce = commitment.subarray(0, 24);
        if (!this.account.aesSecret) {
            // CONSTANT_SECRET_AUTHKEY is used to minimize the number of bytes sent to the blockchain.
            // This results in poly135 being useless since the CONSTANT_SECRET_AUTHKEY is public.
            // However, ciphertext integrity is guaranteed since a hash of the ciphertext is included in a zero-knowledge proof.
            const ciphertext = (0, tweetnacl_1.box)(bytes_message, nonce, this.account.encryptionKeypair.publicKey, index_1.CONSTANT_SECRET_AUTHKEY);
            return Uint8Array.from([...ciphertext]);
        }
        else {
            if (!merkleTreePdaPublicKey)
                throw new index_1.UtxoError(index_1.UtxoErrorCode.MERKLE_TREE_PDA_PUBLICKEY_UNDEFINED, "encrypt", "For aes encryption the merkle tree pda publickey is necessary to derive the viewingkey");
            (0, index_1.setEnvironment)();
            const iv16 = nonce.slice(0, 16);
            const ciphertext = await (0, aes_1.encrypt)(bytes_message, this.account.getAesUtxoViewingKey(merkleTreePdaPublicKey, bytes_1.bs58.encode(commitment)), iv16, "aes-256-cbc", true);
            if (!compressed)
                return ciphertext;
            const padding = sha3_256
                .create()
                .update(Uint8Array.from([...nonce, ...bytes_message]))
                .digest();
            // adding the 8 bytes as padding at the end to make the ciphertext the same length as nacl box ciphertexts of 120 bytes
            return Uint8Array.from([...ciphertext, ...padding.subarray(0, 8)]);
        }
    }
    // TODO: unify compressed and includeAppData into a parsingConfig or just keep one
    /**
     * @description Decrypts a utxo from an array of bytes, the last 24 bytes are the nonce.
     * @param {any} poseidon
     * @param {Uint8Array} encBytes
     * @param {Account} account
     * @param {number} index
     * @returns {Utxo | null}
     */
    static async decrypt({ poseidon, encBytes, account, index, merkleTreePdaPublicKey, aes = true, commitment, appDataIdl, compressed = true, assetLookupTable, verifierProgramLookupTable, }) {
        if (aes) {
            if (!account.aesSecret) {
                throw new index_1.UtxoError(index_1.UtxoErrorCode.AES_SECRET_UNDEFINED, "decrypt");
            }
            if (!merkleTreePdaPublicKey)
                throw new index_1.UtxoError(index_1.UtxoErrorCode.MERKLE_TREE_PDA_PUBLICKEY_UNDEFINED, "encrypt", "For aes decryption the merkle tree pda publickey is necessary to derive the viewingkey");
            if (compressed) {
                encBytes = encBytes.slice(0, index_1.ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH);
            }
            (0, index_1.setEnvironment)();
            const iv16 = commitment.slice(0, 16);
            try {
                const cleartext = await (0, aes_1.decrypt)(encBytes, account.getAesUtxoViewingKey(merkleTreePdaPublicKey, bytes_1.bs58.encode(commitment)), iv16, "aes-256-cbc", true);
                const bytes = Buffer.from(cleartext);
                return Utxo.fromBytes({
                    poseidon,
                    bytes,
                    account,
                    index,
                    appDataIdl,
                    assetLookupTable,
                    verifierProgramLookupTable,
                });
            }
            catch (e) {
                // TODO: return errors - omitted for now because of different error messages on different systems
                return null;
            }
        }
        else {
            const nonce = commitment.slice(0, 24);
            if (compressed) {
                encBytes = encBytes.slice(0, index_1.NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH);
            }
            if (account.encryptionKeypair.secretKey) {
                const cleartext = tweetnacl_1.box.open(encBytes, nonce, tweetnacl_1.default.box.keyPair.fromSecretKey(index_1.CONSTANT_SECRET_AUTHKEY).publicKey, account.encryptionKeypair.secretKey);
                if (!cleartext) {
                    return null;
                }
                const bytes = Buffer.from(cleartext);
                return Utxo.fromBytes({
                    poseidon,
                    bytes,
                    account,
                    index,
                    appDataIdl,
                    assetLookupTable,
                    verifierProgramLookupTable,
                });
            }
            else {
                return null;
            }
        }
    }
    static async fastDecrypt({ merkleTreePdaPublicKey, compressed, encBytes, commitment, aesSecret, asymSecret, }) {
        if (aesSecret) {
            if (asymSecret)
                throw new Error("Asymmetric Secret provided for AES decryption");
            if (!merkleTreePdaPublicKey)
                throw new index_1.UtxoError(index_1.UtxoErrorCode.MERKLE_TREE_PDA_PUBLICKEY_UNDEFINED, "fastDecrypt", "For aes decryption the merkle tree pda publickey is necessary to derive the viewingkey");
            if (compressed) {
                encBytes = encBytes.slice(0, index_1.ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH);
            }
            (0, index_1.setEnvironment)();
            const iv16 = commitment.slice(0, 16);
            try {
                const cleartext = await (0, aes_1.decrypt)(encBytes, index_1.Account.getAesUtxoViewingKey(merkleTreePdaPublicKey, bytes_1.bs58.encode(commitment), aesSecret), iv16, "aes-256-cbc", true);
                const bytes = Buffer.from(cleartext);
                return bytes;
            }
            catch (e) {
                // TODO: return errors - omitted for now because of different error messages on different systems
                return null;
            }
        }
        else {
            // asymmetric decryption
            const nonce = commitment.slice(0, 24);
            if (compressed) {
                encBytes = encBytes.slice(0, index_1.NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH);
            }
            if (asymSecret) {
                const cleartext = tweetnacl_1.box.open(encBytes, nonce, tweetnacl_1.default.box.keyPair.fromSecretKey(index_1.CONSTANT_SECRET_AUTHKEY).publicKey, asymSecret);
                if (!cleartext) {
                    return null;
                }
                const bytes = Buffer.from(cleartext);
                return bytes;
            }
            else {
                return null;
            }
        }
    }
    /**
     * Creates a new Utxo from a given base58 encoded string.
     * @static
     * @param {string} string - The base58 encoded string representation of the Utxo.
     * @returns {Utxo} The newly created Utxo.
     */
    static fromString(string, poseidon, assetLookupTable, verifierProgramLookupTable) {
        return Utxo.fromBytes({
            bytes: bytes_1.bs58.decode(string),
            poseidon,
            assetLookupTable,
            verifierProgramLookupTable,
        });
    }
    /**
     * Converts the Utxo instance into a base58 encoded string.
     * @async
     * @returns {Promise<string>} A promise that resolves to the base58 encoded string representation of the Utxo.
     */
    async toString() {
        const bytes = await this.toBytes();
        return bytes_1.bs58.encode(bytes);
    }
    /**
     * @description Compares two Utxos.
     * @param {Utxo} utxo0
     * @param {Utxo} utxo1
     */
    static equal(poseidon, utxo0, utxo1, skipNullifier = false) {
        chai_1.assert.equal(utxo0.amounts[0].toString(), utxo1.amounts[0].toString(), "solAmount");
        chai_1.assert.equal(utxo0.amounts[1].toString(), utxo1.amounts[1].toString(), "splAmount");
        chai_1.assert.equal(utxo0.assets[0].toBase58(), utxo1.assets[0].toBase58(), "solAsset");
        chai_1.assert.equal(utxo0.assets[1].toBase58(), utxo1.assets[1].toBase58()),
            "splAsset";
        chai_1.assert.equal(utxo0.assetsCircuit[0].toString(), utxo1.assetsCircuit[0].toString(), "solAsset circuit");
        chai_1.assert.equal(utxo0.assetsCircuit[1].toString(), utxo1.assetsCircuit[1].toString(), "splAsset circuit");
        chai_1.assert.equal(utxo0.appDataHash.toString(), utxo1.appDataHash.toString(), "appDataHash");
        chai_1.assert.equal(utxo0.poolType.toString(), utxo1.poolType.toString(), "poolType");
        chai_1.assert.equal(utxo0.verifierAddress.toString(), utxo1.verifierAddress.toString(), "verifierAddress");
        chai_1.assert.equal(utxo0.verifierAddressCircuit.toString(), utxo1.verifierAddressCircuit.toString(), "verifierAddressCircuit");
        chai_1.assert.equal(utxo0.getCommitment(poseidon)?.toString(), utxo1.getCommitment(poseidon)?.toString(), "commitment");
        if (!skipNullifier) {
            if (utxo0.index || utxo1.index) {
                if (utxo0.account.privkey || utxo1.account.privkey) {
                    chai_1.assert.equal(utxo0.getNullifier(poseidon)?.toString(), utxo1.getNullifier(poseidon)?.toString(), "nullifier");
                }
            }
        }
    }
    static getAppInUtxoIndices(appUtxos) {
        let isAppInUtxo = [];
        for (const i in appUtxos) {
            let array = new Array(4).fill(new anchor_1.BN(0));
            if (appUtxos[i].appData) {
                array[i] = new anchor_1.BN(1);
                isAppInUtxo.push(array);
            }
        }
        return isAppInUtxo;
    }
}
exports.Utxo = Utxo;
exports.Utxo = Utxo;
//# sourceMappingURL=utxo.js.map