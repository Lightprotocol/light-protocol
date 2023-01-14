"use strict";
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.Transaction = exports.TransactionParameters = exports.createEncryptionKeypair = void 0;
const anchor = require("@coral-xyz/anchor");
const nacl = require("tweetnacl");
const createEncryptionKeypair = () => nacl.box.keyPair();
exports.createEncryptionKeypair = createEncryptionKeypair;
var assert = require("assert");
let circomlibjs = require("circomlibjs");
var ffjavascript = require("ffjavascript");
const { unstringifyBigInts, stringifyBigInts, leInt2Buff, leBuff2int } = ffjavascript.utils;
const fs_1 = require("fs");
const snarkjs = require("snarkjs");
const { keccak_256 } = require("@noble/hashes/sha3");
const web3_js_1 = require("@solana/web3.js");
const spl_token_1 = require("@solana/spl-token");
const anchor_1 = require("@coral-xyz/anchor");
const constants_1 = require("./constants");
const utxo_1 = require("./utxo");
const testChecks_1 = require("./test-utils/testChecks");
const merkleTreeConfig_1 = require("./merkleTree/merkleTreeConfig");
const index_1 = require("./index");
const merkle_tree_program_1 = require("./idls/merkle_tree_program");
class TransactionParameters {
    constructor({ merkleTreePubkey, verifier, sender, recipient, senderFee, recipientFee, inputUtxos, outputUtxos, }) {
        try {
            this.merkleTreeProgram = new anchor_1.Program(merkle_tree_program_1.MerkleTreeProgram, index_1.merkleTreeProgramId);
        }
        catch (error) {
            console.log(error);
            console.log("assuming test mode thus continuing");
            this.merkleTreeProgram = {
                programId: index_1.merkleTreeProgramId
            };
        }
        this.accounts = {
            systemProgramId: web3_js_1.SystemProgram.programId,
            tokenProgram: spl_token_1.TOKEN_PROGRAM_ID,
            merkleTree: merkleTreePubkey,
            registeredVerifierPda: Transaction.getRegisteredVerifierPda(this.merkleTreeProgram.programId, verifier.verifierProgram.programId),
            authority: Transaction.getSignerAuthorityPda(this.merkleTreeProgram.programId, verifier.verifierProgram.programId),
            preInsertedLeavesIndex: constants_1.PRE_INSERTED_LEAVES_INDEX,
            sender: sender,
            recipient: recipient,
            senderFee: senderFee,
            recipientFee: recipientFee,
            programMerkleTree: this.merkleTreeProgram.programId,
        };
        this.verifier = verifier;
        this.outputUtxos = outputUtxos;
        this.inputUtxos = inputUtxos;
        if (!this.outputUtxos && !inputUtxos) {
            throw new Error("No utxos provided.");
        }
    }
}
exports.TransactionParameters = TransactionParameters;
// add verifier class which is passed in with the constructor
// this class replaces the send transaction, also configures path the provingkey and witness, the inputs for the integrity hash
// input custom verifier with three functions by default prepare, proof, send
// include functions from sdk in shieldedTransaction
// TODO: add log option that enables logs
// TODO: write functional test for every method
class Transaction {
    /**
     * Initialize transaction
     *
     * @param instance encryptionKeypair used for encryption
     * @param relayer recipient of the unshielding
     * @param payer
     * @param shuffleEnabled
     */
    constructor({ instance, relayer, payer, shuffleEnabled = true, }) {
        if (relayer) {
            this.action = "WITHDRAWAL";
            this.relayer = relayer;
            this.payer = payer;
            console.log("withdrawal");
        }
        else if (!relayer && payer) {
            this.action = "DEPOSIT";
            this.payer = payer;
            this.relayer = new index_1.Relayer(payer.publicKey, instance.lookUpTable);
        }
        else {
            throw new Error("No payer and relayer provided.");
        }
        this.instance = instance;
        this.shuffleEnabled = shuffleEnabled;
    }
    // Returns serialized instructions
    proveAndCreateInstructionsJson(params) {
        return __awaiter(this, void 0, void 0, function* () {
            yield this.compileAndProve(params);
            return yield this.getInstructionsJson();
        });
    }
    proveAndCreateInstructions(params) {
        return __awaiter(this, void 0, void 0, function* () {
            yield this.compileAndProve(params);
            return yield this.params.verifier.getInstructions(this);
        });
    }
    compileAndProve(params) {
        return __awaiter(this, void 0, void 0, function* () {
            yield this.compile(params);
            yield this.getProof();
        });
    }
    compile(params) {
        return __awaiter(this, void 0, void 0, function* () {
            // TODO: create and check for existence of merkleTreeAssetPubkey depending on utxo asset
            this.poseidon = yield circomlibjs.buildPoseidonOpt();
            this.params = params;
            this.params.accounts.signingAddress = this.relayer.accounts.relayerPubkey;
            // prepare utxos
            const pubkeys = this.getAssetPubkeys(params.inputUtxos, params.outputUtxos);
            this.assetPubkeys = pubkeys.assetPubkeys;
            this.assetPubkeysCircuit = pubkeys.assetPubkeysCircuit;
            this.params.inputUtxos = this.addEmptyUtxos(params.inputUtxos, params.verifier.config.in);
            this.params.outputUtxos = this.addEmptyUtxos(params.outputUtxos, params.verifier.config.out);
            this.shuffleUtxos(this.params.inputUtxos);
            this.shuffleUtxos(this.params.outputUtxos);
            // prep and get proof inputs
            this.publicAmount = this.getExternalAmount(1);
            this.feeAmount = this.getExternalAmount(0);
            this.assignAccounts(params);
            this.getMerkleProofs();
            this.getProofInput();
            yield this.getRootIndex();
        });
    }
    getProofInput() {
        var _a, _b, _c, _d, _e, _f, _g, _h, _j, _k, _l, _m;
        if (this.params &&
            this.instance.solMerkleTree.merkleTree &&
            this.params.inputUtxos &&
            this.params.outputUtxos &&
            this.assetPubkeysCircuit) {
            this.proofInput = {
                root: this.instance.solMerkleTree.merkleTree.root(),
                inputNullifier: this.params.inputUtxos.map((x) => x.getNullifier()),
                outputCommitment: this.params.outputUtxos.map((x) => x.getCommitment()),
                // TODO: move public and fee amounts into tx preparation
                publicAmount: this.getExternalAmount(1).toString(),
                feeAmount: this.getExternalAmount(0).toString(),
                extDataHash: this.getTxIntegrityHash().toString(),
                mintPubkey: this.assetPubkeysCircuit[1],
                // data for 2 transaction inputUtxos
                inAmount: (_a = this.params.inputUtxos) === null || _a === void 0 ? void 0 : _a.map((x) => x.amounts),
                inPrivateKey: (_b = this.params.inputUtxos) === null || _b === void 0 ? void 0 : _b.map((x) => x.keypair.privkey),
                inBlinding: (_c = this.params.inputUtxos) === null || _c === void 0 ? void 0 : _c.map((x) => x.blinding),
                inPathIndices: this.inputMerklePathIndices,
                inPathElements: this.inputMerklePathElements,
                assetPubkeys: this.assetPubkeysCircuit,
                // data for 2 transaction outputUtxos
                outAmount: (_d = this.params.outputUtxos) === null || _d === void 0 ? void 0 : _d.map((x) => x.amounts),
                outBlinding: (_e = this.params.outputUtxos) === null || _e === void 0 ? void 0 : _e.map((x) => x.blinding),
                outPubkey: (_f = this.params.outputUtxos) === null || _f === void 0 ? void 0 : _f.map((x) => x.keypair.pubkey),
                inIndices: this.getIndices(this.params.inputUtxos),
                outIndices: this.getIndices(this.params.outputUtxos),
                inInstructionType: (_g = this.params.inputUtxos) === null || _g === void 0 ? void 0 : _g.map((x) => x.instructionType),
                outInstructionType: (_h = this.params.outputUtxos) === null || _h === void 0 ? void 0 : _h.map((x) => x.instructionType),
                inPoolType: (_j = this.params.inputUtxos) === null || _j === void 0 ? void 0 : _j.map((x) => x.poolType),
                outPoolType: (_k = this.params.outputUtxos) === null || _k === void 0 ? void 0 : _k.map((x) => x.poolType),
                inVerifierPubkey: (_l = this.params.inputUtxos) === null || _l === void 0 ? void 0 : _l.map((x) => x.verifierAddressCircuit),
                outVerifierPubkey: (_m = this.params.outputUtxos) === null || _m === void 0 ? void 0 : _m.map((x) => x.verifierAddressCircuit),
                connectingHash: this.getConnectingHash(),
                verifier: this.params.verifier.pubkey
            };
        }
        else {
            throw new Error(`getProofInput has undefined inputs`);
        }
    }
    getProof() {
        return __awaiter(this, void 0, void 0, function* () {
            if (!this.instance.solMerkleTree.merkleTree) {
                throw new Error("merkle tree not built");
            }
            if (!this.proofInput) {
                throw new Error("transaction not compiled");
            }
            if (!this.params) {
                throw new Error("params undefined probably not compiled");
            }
            else {
                // console.log("this.proofInput ", this.proofInput);
                const path = require("path");
                const firstPath = path.resolve(__dirname, "../build-circuits/");
                const completePathWtns = firstPath + "/" + this.params.verifier.wtnsGenPath;
                const completePathZkey = firstPath + "/" + this.params.verifier.zkeyPath;
                const buffer = (0, fs_1.readFileSync)(completePathWtns);
                let witnessCalculator = yield this.params.verifier.calculateWtns(buffer);
                console.time("Proof generation");
                let wtns = yield witnessCalculator.calculateWTNSBin(stringifyBigInts(this.proofInput), 0);
                const { proof, publicSignals } = yield snarkjs.groth16.prove(
                // `${this.params.verifier.zkeyPath}.zkey`,
                completePathZkey, wtns);
                // this.params.verifier.zkeyPath
                const proofJson = JSON.stringify(proof, null, 1);
                const publicInputsJson = JSON.stringify(publicSignals, null, 1);
                console.timeEnd("Proof generation");
                const vKey = yield snarkjs.zKey.exportVerificationKey(completePathZkey);
                const res = yield snarkjs.groth16.verify(vKey, publicSignals, proof);
                if (res === true) {
                    console.log("Verification OK");
                }
                else {
                    console.log("Invalid proof");
                    throw new Error("Invalid Proof");
                }
                this.publicInputsBytes = JSON.parse(publicInputsJson.toString());
                for (var i in this.publicInputsBytes) {
                    this.publicInputsBytes[i] = Array.from(leInt2Buff(unstringifyBigInts(this.publicInputsBytes[i]), 32)).reverse();
                }
                // console.log("publicInputsBytes ", this.publicInputsBytes);
                this.proofBytes = yield Transaction.parseProofToBytesArray(proofJson);
                this.publicInputs = this.params.verifier.parsePublicInputsFromArray(this);
                // await this.checkProof()
                if (this.instance.provider) {
                    yield this.getPdaAddresses();
                }
            }
        });
    }
    getConnectingHash() {
        var _a, _b, _c, _d;
        const inputHasher = this.poseidon.F.toString(this.poseidon((_b = (_a = this.params) === null || _a === void 0 ? void 0 : _a.inputUtxos) === null || _b === void 0 ? void 0 : _b.map((utxo) => utxo.getCommitment())));
        const outputHasher = this.poseidon.F.toString(this.poseidon((_d = (_c = this.params) === null || _c === void 0 ? void 0 : _c.outputUtxos) === null || _d === void 0 ? void 0 : _d.map((utxo) => utxo.getCommitment())));
        return this.poseidon.F.toString(this.poseidon([inputHasher, outputHasher]));
    }
    assignAccounts(params) {
        if (this.assetPubkeys && this.params) {
            if (!this.params.accounts.sender && !this.params.accounts.senderFee) {
                if (this.action !== "WITHDRAWAL") {
                    throw new Error("No relayer provided for withdrawal");
                }
                this.params.accounts.sender = merkleTreeConfig_1.MerkleTreeConfig.getSplPoolPdaToken(this.assetPubkeys[1], index_1.merkleTreeProgramId);
                this.params.accounts.senderFee =
                    merkleTreeConfig_1.MerkleTreeConfig.getSolPoolPda(index_1.merkleTreeProgramId).pda;
                if (!this.params.accounts.recipient) {
                    this.params.accounts.recipient = web3_js_1.SystemProgram.programId;
                    if (this.publicAmount != new anchor_1.BN(0)) {
                        throw new Error("sth is wrong assignAccounts !params.accounts.recipient");
                    }
                }
                if (!this.params.accounts.recipientFee) {
                    this.params.accounts.recipientFee = web3_js_1.SystemProgram.programId;
                    if (this.feeAmount != new anchor_1.BN(0)) {
                        throw new Error("sth is wrong assignAccounts !params.accounts.recipientFee");
                    }
                }
            }
            else {
                if (this.action !== "DEPOSIT") {
                    throw new Error("Relayer should not be provided for deposit.");
                }
                this.params.accounts.recipient = merkleTreeConfig_1.MerkleTreeConfig.getSplPoolPdaToken(this.assetPubkeys[1], index_1.merkleTreeProgramId);
                this.params.accounts.recipientFee =
                    merkleTreeConfig_1.MerkleTreeConfig.getSolPoolPda(index_1.merkleTreeProgramId).pda;
                if (!this.params.accounts.sender) {
                    this.params.accounts.sender = web3_js_1.SystemProgram.programId;
                    if (this.publicAmount != new anchor_1.BN(0)) {
                        throw new Error("sth is wrong assignAccounts !params.accounts.sender");
                    }
                }
                if (!this.params.accounts.senderFee) {
                    this.params.accounts.senderFee = web3_js_1.SystemProgram.programId;
                    if (this.feeAmount != new anchor_1.BN(0)) {
                        throw new Error("sth is wrong assignAccounts !params.accounts.senderFee");
                    }
                }
            }
        }
        else {
            throw new Error("assignAccounts assetPubkeys undefined");
        }
    }
    getAssetPubkeys(inputUtxos, outputUtxos) {
        let assetPubkeysCircuit = [new anchor_1.BN(0)];
        let assetPubkeys = [web3_js_1.SystemProgram.programId];
        if (inputUtxos) {
            inputUtxos.map((utxo) => {
                if (assetPubkeysCircuit.indexOf(utxo.assetsCircuit[1]) == -1) {
                    assetPubkeysCircuit.push(utxo.assetsCircuit[1]);
                    assetPubkeys.push(utxo.assets[1]);
                }
            });
        }
        if (outputUtxos) {
            outputUtxos.map((utxo) => {
                if (assetPubkeysCircuit.indexOf(utxo.assetsCircuit[1]) == -1) {
                    assetPubkeysCircuit.push(utxo.assetsCircuit[1]);
                    assetPubkeys.push(utxo.assets[1]);
                }
            });
        }
        if (assetPubkeys.length == 0) {
            throw new Error("No utxos provided.");
        }
        if (assetPubkeys.length > utxo_1.N_ASSET_PUBKEYS) {
            throw new Error("Utxos contain too many different assets.");
        }
        while (assetPubkeysCircuit.length < utxo_1.N_ASSET_PUBKEYS) {
            assetPubkeysCircuit.push(new anchor_1.BN(0));
        }
        return { assetPubkeysCircuit, assetPubkeys };
    }
    getRootIndex() {
        return __awaiter(this, void 0, void 0, function* () {
            if (this.instance.provider && this.instance.solMerkleTree.merkleTree) {
                this.merkleTreeProgram = new anchor_1.Program(merkle_tree_program_1.MerkleTreeProgram, index_1.merkleTreeProgramId);
                let root = Uint8Array.from(leInt2Buff(unstringifyBigInts(this.instance.solMerkleTree.merkleTree.root()), 32));
                let merkle_tree_account_data = yield this.merkleTreeProgram.account.merkleTree.fetch(this.instance.solMerkleTree.pubkey);
                merkle_tree_account_data.roots.map((x, index) => {
                    if (x.toString() === root.toString()) {
                        this.rootIndex = index;
                    }
                });
            }
            else {
                console.log("provider not defined did not fetch rootIndex set root index to 0");
                this.rootIndex = 0;
            }
        });
    }
    addEmptyUtxos(utxos = [], len) {
        if (this.params && this.params.verifier.config) {
            while (utxos.length < len) {
                utxos.push(new utxo_1.Utxo({ poseidon: this.poseidon }));
            }
        }
        else {
            throw new Error(`input utxos ${utxos}, config ${this.params.verifier.config}`);
        }
        return utxos;
    }
    // the fee plus the amount to pay has to be bigger than the amount in the input utxo
    // which doesn't make sense it should be the other way arround right
    // the external amount can only be made up of utxos of asset[0]
    // This might be too specific since the circuit allows assets to be in any index
    // TODO: write test
    getExternalAmount(assetIndex) {
        if (this.params &&
            this.params.inputUtxos &&
            this.params.outputUtxos &&
            this.assetPubkeysCircuit) {
            return new anchor.BN(0)
                .add(this.params.outputUtxos
                .filter((utxo) => {
                return (utxo.assetsCircuit[assetIndex].toString("hex") ==
                    this.assetPubkeysCircuit[assetIndex].toString("hex"));
            })
                .reduce((sum, utxo) => 
            // add all utxos of the same asset
            sum.add(utxo.amounts[assetIndex]), new anchor.BN(0)))
                .sub(this.params.inputUtxos
                .filter((utxo) => {
                return (utxo.assetsCircuit[assetIndex].toString("hex") ==
                    this.assetPubkeysCircuit[assetIndex].toString("hex"));
            })
                .reduce((sum, utxo) => sum.add(utxo.amounts[assetIndex]), new anchor.BN(0)))
                .add(index_1.FIELD_SIZE)
                .mod(index_1.FIELD_SIZE);
        }
        else {
            new Error(`this.params.inputUtxos ${this.params.inputUtxos} && this.params.outputUtxos ${this.params.outputUtxos} && this.assetPubkeysCircuit ${this.assetPubkeysCircuit}`);
        }
    }
    // TODO: write test
    // TODO: make this work for edge case of two 2 different assets plus fee asset in the same transaction
    getIndices(utxos) {
        let inIndices = [];
        utxos.map((utxo) => {
            let tmpInIndices = [];
            for (var a = 0; a < utxo.assets.length; a++) {
                let tmpInIndices1 = [];
                for (var i = 0; i < utxo_1.N_ASSET_PUBKEYS; i++) {
                    try {
                        if (utxo.assetsCircuit[i].toString() ===
                            this.assetPubkeysCircuit[a].toString() &&
                            utxo.amounts[a].toString() > "0" &&
                            !tmpInIndices1.includes("1")) {
                            tmpInIndices1.push("1");
                        }
                        else {
                            tmpInIndices1.push("0");
                        }
                    }
                    catch (error) {
                        tmpInIndices1.push("0");
                    }
                }
                tmpInIndices.push(tmpInIndices1);
            }
            inIndices.push(tmpInIndices);
        });
        return inIndices;
    }
    getMerkleProofs() {
        this.inputMerklePathIndices = [];
        this.inputMerklePathElements = [];
        // getting merkle proofs
        for (const inputUtxo of this.params.inputUtxos) {
            if (inputUtxo.amounts[0] > new anchor_1.BN(0) ||
                inputUtxo.amounts[1] > new anchor_1.BN(0)) {
                inputUtxo.index = this.instance.solMerkleTree.merkleTree.indexOf(inputUtxo.getCommitment());
                if (inputUtxo.index || inputUtxo.index == 0) {
                    if (inputUtxo.index < 0) {
                        throw new Error(`Input commitment ${inputUtxo.getCommitment()} was not found`);
                    }
                    this.inputMerklePathIndices.push(inputUtxo.index);
                    this.inputMerklePathElements.push(this.instance.solMerkleTree.merkleTree.path(inputUtxo.index).pathElements);
                }
            }
            else {
                this.inputMerklePathIndices.push(0);
                this.inputMerklePathElements.push(new Array(this.instance.solMerkleTree.merkleTree.levels).fill(0));
            }
        }
    }
    getTxIntegrityHash() {
        var _a;
        if (!this.params.accounts.recipient ||
            !this.params.accounts.recipientFee ||
            !this.relayer.relayerFee) {
            throw new Error(`getTxIntegrityHash: recipient ${this.params.accounts.recipient} recipientFee ${this.params.accounts.recipientFee} relayerFee ${this.relayer.relayerFee}`);
        }
        else {
            this.encryptedUtxos = this.encryptOutUtxos();
            if (this.encryptedUtxos) {
                let extDataBytes = new Uint8Array([
                    ...(_a = this.params.accounts.recipient) === null || _a === void 0 ? void 0 : _a.toBytes(),
                    ...this.params.accounts.recipientFee.toBytes(),
                    ...this.payer.publicKey.toBytes(),
                    ...this.relayer.relayerFee.toArray("le", 8),
                    ...this.encryptedUtxos,
                ]);
                const hash = keccak_256
                    .create({ dkLen: 32 })
                    .update(Buffer.from(extDataBytes))
                    .digest();
                return new anchor.BN(hash).mod(index_1.FIELD_SIZE);
            }
            else {
                throw new Error("Encrypting Utxos failed");
            }
        }
    }
    encryptOutUtxos(encryptedUtxos) {
        let encryptedOutputs = new Array();
        if (encryptedUtxos) {
            encryptedOutputs = Array.from(encryptedUtxos);
        }
        else {
            this.params.outputUtxos.map((utxo, index) => encryptedOutputs.push(utxo.encrypt()));
            if (this.params.verifier.config.out == 2) {
                return new Uint8Array([
                    ...encryptedOutputs[0],
                    ...encryptedOutputs[1],
                    ...new Array(256 - 190).fill(0),
                ]);
            }
            else {
                let tmpArray = new Array();
                for (var i = 0; i < this.params.verifier.config.out; i++) {
                    tmpArray.push(...encryptedOutputs[i]);
                }
                if (tmpArray.length < 512) {
                    tmpArray.push(new Array(this.params.verifier.config.out * 128 - tmpArray.length).fill(0));
                }
                // return new Uint8Array(tmpArray.flat());
                return new Uint8Array([
                    ...tmpArray
                ]);
            }
        }
    }
    // need this for the marketplace rn
    overWriteEncryptedUtxos(bytes, offSet = 0) {
        // this.encryptedUtxos.slice(offSet, bytes.length + offSet) = bytes;
        this.encryptedUtxos = Uint8Array.from([
            ...this.encryptedUtxos.slice(0, offSet),
            ...bytes,
            ...this.encryptedUtxos.slice(offSet + bytes.length, this.encryptedUtxos.length),
        ]);
    }
    getPublicInputs() {
        this.publicInputs = this.params.verifier.parsePublicInputsFromArray(this);
    }
    // send transaction should be the same for both deposit and withdrawal
    // the function should just send the tx to the rpc or relayer respectively
    // in case there is more than one transaction to be sent to the verifier these can be sent separately
    getTestValues() {
        return __awaiter(this, void 0, void 0, function* () {
            try {
                this.recipientBalancePriorTx = (yield (0, spl_token_1.getAccount)(this.instance.provider.connection, this.params.accounts.recipient, spl_token_1.TOKEN_PROGRAM_ID)).amount;
            }
            catch (e) {
                // covers the case of the recipient being a native sol address not a spl token address
                try {
                    this.recipientBalancePriorTx =
                        yield this.instance.provider.connection.getBalance(this.params.accounts.recipient);
                }
                catch (e) { }
            }
            try {
                this.recipientFeeBalancePriorTx =
                    yield this.instance.provider.connection.getBalance(this.params.accounts.recipientFee);
            }
            catch (error) {
                console.log("this.recipientFeeBalancePriorTx fetch failed ", this.params.accounts.recipientFee);
            }
            this.senderFeeBalancePriorTx =
                yield this.instance.provider.connection.getBalance(this.params.accounts.senderFee);
            this.relayerRecipientAccountBalancePriorLastTx =
                yield this.instance.provider.connection.getBalance(this.relayer.accounts.relayerRecipient);
        });
    }
    static getSignerAuthorityPda(merkleTreeProgramId, verifierProgramId) {
        return web3_js_1.PublicKey.findProgramAddressSync([merkleTreeProgramId.toBytes()], verifierProgramId)[0];
    }
    static getRegisteredVerifierPda(merkleTreeProgramId, verifierProgramId) {
        return web3_js_1.PublicKey.findProgramAddressSync([verifierProgramId.toBytes()], merkleTreeProgramId)[0];
    }
    getInstructionsJson() {
        return __awaiter(this, void 0, void 0, function* () {
            const instructions = yield this.params.verifier.getInstructions(this);
            let serialized = instructions.map((ix) => JSON.stringify(ix));
            return serialized;
        });
    }
    sendTransaction(ix) {
        return __awaiter(this, void 0, void 0, function* () {
            if (!this.payer) {
                // send tx to relayer
                let txJson = yield this.getInstructionsJson();
                // request to relayer
                throw new Error("withdrawal with relayer is not implemented");
            }
            else {
                const recentBlockhash = (yield this.instance.provider.connection.getRecentBlockhash("confirmed")).blockhash;
                const txMsg = new web3_js_1.TransactionMessage({
                    payerKey: this.payer.publicKey,
                    instructions: [
                        web3_js_1.ComputeBudgetProgram.setComputeUnitLimit({ units: 1400000 }),
                        ix,
                    ],
                    recentBlockhash: recentBlockhash,
                });
                const lookupTableAccount = yield this.instance.provider.connection.getAccountInfo(this.relayer.accounts.lookUpTable, "confirmed");
                const unpackedLookupTableAccount = web3_js_1.AddressLookupTableAccount.deserialize(lookupTableAccount.data);
                const compiledTx = txMsg.compileToV0Message([
                    {
                        state: unpackedLookupTableAccount,
                        key: this.relayer.accounts.lookUpTable,
                        isActive: () => {
                            return true;
                        },
                    },
                ]);
                compiledTx.addressTableLookups[0].accountKey =
                    this.relayer.accounts.lookUpTable;
                const tx = new web3_js_1.VersionedTransaction(compiledTx);
                let retries = 3;
                let res;
                while (retries > 0) {
                    tx.sign([this.payer]);
                    try {
                        let serializedTx = tx.serialize();
                        console.log("serializedTx: ");
                        res = yield this.instance.provider.connection.sendRawTransaction(serializedTx, constants_1.confirmConfig);
                        retries = 0;
                        console.log(res);
                    }
                    catch (e) {
                        retries--;
                        if (retries == 0 || e.logs != undefined) {
                            console.log(e);
                            return e;
                        }
                    }
                }
                return res;
            }
        });
    }
    sendAndConfirmTransaction() {
        var _a;
        return __awaiter(this, void 0, void 0, function* () {
            if (!this.payer) {
                throw new Error("Cannot use sendAndConfirmTransaction without payer");
            }
            yield this.getTestValues();
            const instructions = yield this.params.verifier.getInstructions(this);
            let tx = "";
            for (var ix in instructions) {
                let txTmp = yield this.sendTransaction(instructions[ix]);
                if (txTmp) {
                    yield ((_a = this.instance.provider) === null || _a === void 0 ? void 0 : _a.connection.confirmTransaction(txTmp, "confirmed"));
                    tx = txTmp;
                }
                else {
                    throw new Error("send transaction failed");
                }
            }
            return tx;
        });
    }
    checkProof() {
        return __awaiter(this, void 0, void 0, function* () {
            let publicSignals = [
                leBuff2int(Buffer.from(this.publicInputs.root.reverse())).toString(),
                leBuff2int(Buffer.from(this.publicInputs.publicAmount.reverse())).toString(),
                leBuff2int(Buffer.from(this.publicInputs.extDataHash.reverse())).toString(),
                leBuff2int(Buffer.from(this.publicInputs.feeAmount.reverse())).toString(),
                leBuff2int(Buffer.from(this.publicInputs.mintPubkey.reverse())).toString(),
                leBuff2int(Buffer.from(this.publicInputs.nullifiers[0].reverse())).toString(),
                leBuff2int(Buffer.from(this.publicInputs.nullifiers[1].reverse())).toString(),
                leBuff2int(Buffer.from(this.publicInputs.leaves[0].reverse())).toString(),
                leBuff2int(Buffer.from(this.publicInputs.leaves[1].reverse())).toString(),
            ];
            let pi_b_0 = this.proofBytes.slice(64, 128).reverse();
            let pi_b_1 = this.proofBytes.slice(128, 192).reverse();
            let proof = {
                pi_a: [
                    leBuff2int(Buffer.from(this.proofBytes.slice(0, 32).reverse())).toString(),
                    leBuff2int(Buffer.from(this.proofBytes.slice(32, 64).reverse())).toString(),
                    "1",
                ],
                pi_b: [
                    [
                        leBuff2int(Buffer.from(pi_b_0.slice(0, 32))).toString(),
                        leBuff2int(Buffer.from(pi_b_0.slice(32, 64))).toString(),
                    ],
                    [
                        leBuff2int(Buffer.from(pi_b_1.slice(0, 32))).toString(),
                        leBuff2int(Buffer.from(pi_b_1.slice(32, 64))).toString(),
                    ],
                    ["1", "0"],
                ],
                pi_c: [
                    leBuff2int(Buffer.from(this.proofBytes.slice(192, 224).reverse())).toString(),
                    leBuff2int(Buffer.from(this.proofBytes.slice(224, 256).reverse())).toString(),
                    "1",
                ],
                protocol: "groth16",
                curve: "bn128",
            };
            const vKey = yield snarkjs.zKey.exportVerificationKey(`${this.params.verifier.zkeyPath}.zkey`);
            const res = yield snarkjs.groth16.verify(vKey, publicSignals, proof);
            if (res === true) {
                console.log("Verification OK");
            }
            else {
                console.log("Invalid proof");
                throw new Error("Invalid Proof");
            }
        });
    }
    getPdaAddresses() {
        return __awaiter(this, void 0, void 0, function* () {
            if (this.params && this.publicInputs && this.merkleTreeProgram) {
                let nullifiers = this.publicInputs.nullifiers;
                let merkleTreeProgram = this.merkleTreeProgram;
                let signer = this.relayer.accounts.relayerPubkey;
                this.params.nullifierPdaPubkeys = [];
                for (var i in nullifiers) {
                    this.params.nullifierPdaPubkeys.push({
                        isSigner: false,
                        isWritable: true,
                        pubkey: web3_js_1.PublicKey.findProgramAddressSync([Buffer.from(nullifiers[i]), anchor.utils.bytes.utf8.encode("nf")], merkleTreeProgram.programId)[0],
                    });
                }
                this.params.leavesPdaPubkeys = [];
                for (var i in this.publicInputs.leaves) {
                    this.params.leavesPdaPubkeys.push({
                        isSigner: false,
                        isWritable: true,
                        pubkey: web3_js_1.PublicKey.findProgramAddressSync([
                            Buffer.from(Array.from(this.publicInputs.leaves[i][0]).reverse()),
                            anchor.utils.bytes.utf8.encode("leaves"),
                        ], merkleTreeProgram.programId)[0],
                    });
                }
                this.params.accounts.escrow = web3_js_1.PublicKey.findProgramAddressSync([anchor.utils.bytes.utf8.encode("escrow")], this.params.verifier.verifierProgram.programId)[0];
                this.params.accounts.verifierState = web3_js_1.PublicKey.findProgramAddressSync([signer.toBytes(), anchor.utils.bytes.utf8.encode("VERIFIER_STATE")], this.params.verifier.verifierProgram.programId)[0];
                this.params.accounts.tokenAuthority = web3_js_1.PublicKey.findProgramAddressSync([anchor.utils.bytes.utf8.encode("spl")], merkleTreeProgram.programId)[0];
            }
            else {
                throw new Error(`${this.params} && ${this.publicInputs} && ${this.merkleTreeProgram}`);
            }
        });
    }
    checkBalances() {
        var _a, _b, _c, _d, _e, _f, _g, _h;
        return __awaiter(this, void 0, void 0, function* () {
            // Checking that nullifiers were inserted
            this.is_token = true;
            for (var i in this.params.nullifierPdaPubkeys) {
                var nullifierAccount = yield this.instance.provider.connection.getAccountInfo(this.params.nullifierPdaPubkeys[i].pubkey, {
                    commitment: "confirmed",
                });
                yield (0, testChecks_1.checkRentExemption)({
                    account: nullifierAccount,
                    connection: this.instance.provider.connection,
                });
            }
            let leavesAccount;
            var leavesAccountData;
            // Checking that leaves were inserted
            for (var i in this.params.leavesPdaPubkeys) {
                leavesAccountData =
                    yield this.merkleTreeProgram.account.twoLeavesBytesPda.fetch(this.params.leavesPdaPubkeys[i].pubkey);
                assert(leavesAccountData.nodeLeft.toString() ==
                    this.publicInputs.leaves[i][0].reverse().toString(), "left leaf not inserted correctly");
                assert(leavesAccountData.nodeRight.toString() ==
                    this.publicInputs.leaves[i][1].reverse().toString(), "right leaf not inserted correctly");
                assert(leavesAccountData.merkleTreePubkey.toBase58() ==
                    this.instance.solMerkleTree.pubkey.toBase58(), "merkleTreePubkey not inserted correctly");
                for (var j = 0; j < this.encryptedUtxos.length / 256; j++) {
                    // console.log(j);
                    if (leavesAccountData.encryptedUtxos.toString() !==
                        this.encryptedUtxos.toString()) {
                        // console.log(j);
                        // throw `encrypted utxo ${i} was not stored correctly`;
                    }
                    // console.log(
                    //   `${leavesAccountData.encryptedUtxos} !== ${this.encryptedUtxos}`
                    // );
                    // assert(leavesAccountData.encryptedUtxos === this.encryptedUtxos, "encryptedUtxos not inserted correctly");
                    let decryptedUtxo1 = utxo_1.Utxo.decrypt({
                        poseidon: this.poseidon,
                        encBytes: this.encryptedUtxos,
                        keypair: this.params.outputUtxos[0].keypair,
                    });
                    const utxoEqual = (utxo0, utxo1) => {
                        assert.equal(utxo0.amounts[0].toString(), utxo1.amounts[0].toString());
                        assert.equal(utxo0.amounts[1].toString(), utxo1.amounts[1].toString());
                        assert.equal(utxo0.assets[0].toString(), utxo1.assets[0].toString());
                        assert.equal(utxo0.assets[1].toString(), utxo1.assets[1].toString());
                        assert.equal(utxo0.assetsCircuit[0].toString(), utxo1.assetsCircuit[0].toString());
                        assert.equal(utxo0.assetsCircuit[1].toString(), utxo1.assetsCircuit[1].toString());
                        assert.equal(utxo0.instructionType.toString(), utxo1.instructionType.toString());
                        assert.equal(utxo0.poolType.toString(), utxo1.poolType.toString());
                        assert.equal(utxo0.verifierAddress.toString(), utxo1.verifierAddress.toString());
                        assert.equal(utxo0.verifierAddressCircuit.toString(), utxo1.verifierAddressCircuit.toString());
                    };
                    // console.log("decryptedUtxo ", decryptedUtxo1);
                    // console.log("this.params.outputUtxos[0] ", this.params.outputUtxos[0]);
                    utxoEqual(decryptedUtxo1, this.params.outputUtxos[0]);
                }
            }
            console.log(`mode ${this.action}, this.is_token ${this.is_token}`);
            try {
                var preInsertedLeavesIndexAccount = yield this.instance.provider.connection.getAccountInfo(constants_1.PRE_INSERTED_LEAVES_INDEX);
                const preInsertedLeavesIndexAccountAfterUpdate = this.merkleTreeProgram.account.preInsertedLeavesIndex._coder.accounts.decode("PreInsertedLeavesIndex", preInsertedLeavesIndexAccount.data);
                console.log("Number(preInsertedLeavesIndexAccountAfterUpdate.nextIndex) ", Number(preInsertedLeavesIndexAccountAfterUpdate.nextIndex));
                console.log(`${Number(leavesAccountData.leftLeafIndex)} + ${this.params.leavesPdaPubkeys.length * 2}`);
                assert(Number(preInsertedLeavesIndexAccountAfterUpdate.nextIndex) ==
                    Number(leavesAccountData.leftLeafIndex) +
                        this.params.leavesPdaPubkeys.length * 2);
            }
            catch (e) {
                console.log("preInsertedLeavesIndex: ", e);
            }
            if (this.action == "DEPOSIT" && this.is_token == false) {
                var recipientAccount = yield this.instance.provider.connection.getAccountInfo(this.params.accounts.recipient);
                assert(recipientAccount.lamports ==
                    I64(this.recipientBalancePriorTx)
                        .add(this.publicAmount.toString())
                        .toString(), "amount not transferred correctly");
            }
            else if (this.action == "DEPOSIT" && this.is_token == true) {
                console.log("DEPOSIT and token");
                var recipientAccount = yield (0, spl_token_1.getAccount)(this.instance.provider.connection, this.params.accounts.recipient, spl_token_1.TOKEN_PROGRAM_ID);
                var recipientFeeAccountBalance = yield this.instance.provider.connection.getBalance(this.params.accounts.recipientFee);
                // console.log(`Balance now ${senderAccount.amount} balance beginning ${senderAccountBalancePriorLastTx}`)
                // assert(senderAccount.lamports == (I64(senderAccountBalancePriorLastTx) - I64.readLE(this.extAmount, 0)).toString(), "amount not transferred correctly");
                console.log(`Balance now ${recipientAccount.amount} balance beginning ${this.recipientBalancePriorTx}`);
                console.log(`Balance now ${recipientAccount.amount} balance beginning ${Number(this.recipientBalancePriorTx) + Number(this.publicAmount)}`);
                assert(recipientAccount.amount ==
                    (Number(this.recipientBalancePriorTx) + Number(this.publicAmount)).toString(), "amount not transferred correctly");
                console.log(`Blanace now ${recipientFeeAccountBalance} ${Number(this.recipientFeeBalancePriorTx) + Number(this.feeAmount)}`);
                console.log("fee amount: ", this.feeAmount);
                console.log("fee amount from inputs. ", new anchor.BN(this.publicInputs.feeAmount.slice(24, 32)).toString());
                console.log("pub amount from inputs. ", new anchor.BN(this.publicInputs.publicAmount.slice(24, 32)).toString());
                console.log("recipientFeeBalancePriorTx: ", this.recipientFeeBalancePriorTx);
                var senderFeeAccountBalance = yield this.instance.provider.connection.getBalance(this.params.accounts.senderFee);
                console.log("senderFeeAccountBalance: ", senderFeeAccountBalance);
                console.log("this.senderFeeBalancePriorTx: ", this.senderFeeBalancePriorTx);
                assert(recipientFeeAccountBalance ==
                    Number(this.recipientFeeBalancePriorTx) + Number(this.feeAmount));
                console.log(`${Number(this.senderFeeBalancePriorTx)} - ${Number(this.feeAmount)} == ${senderFeeAccountBalance}`);
                assert(Number(this.senderFeeBalancePriorTx) -
                    Number(this.feeAmount) -
                    5000 * ((_a = this.params.verifier.instructions) === null || _a === void 0 ? void 0 : _a.length) ==
                    Number(senderFeeAccountBalance));
            }
            else if (this.action == "WITHDRAWAL" && this.is_token == false) {
                var senderAccount = yield this.instance.provider.connection.getAccountInfo(this.params.accounts.sender);
                var recipientAccount = yield this.instance.provider.connection.getAccountInfo(this.params.accounts.recipient);
                // console.log("senderAccount.lamports: ", senderAccount.lamports)
                // console.log("I64(senderAccountBalancePriorLastTx): ", I64(senderAccountBalancePriorLastTx).toString())
                // console.log("Sum: ", ((I64(senderAccountBalancePriorLastTx).add(I64.readLE(this.extAmount, 0))).sub(I64(relayerFee))).toString())
                assert.equal(senderAccount.lamports, I64(senderAccountBalancePriorLastTx)
                    .add(I64.readLE(this.extAmount, 0))
                    .sub(I64(relayerFee))
                    .toString(), "amount not transferred correctly");
                var recipientAccount = yield this.instance.provider.connection.getAccountInfo(recipient);
                // console.log(`recipientAccount.lamports: ${recipientAccount.lamports} == sum ${((I64(Number(this.recipientBalancePriorTx)).sub(I64.readLE(this.extAmount, 0))).add(I64(relayerFee))).toString()}
                assert(recipientAccount.lamports ==
                    I64(Number(this.recipientBalancePriorTx))
                        .sub(I64.readLE(this.extAmount, 0))
                        .toString(), "amount not transferred correctly");
            }
            else if (this.action == "WITHDRAWAL" && this.is_token == true) {
                var senderAccount = yield (0, spl_token_1.getAccount)(this.instance.provider.connection, this.params.accounts.sender, spl_token_1.TOKEN_PROGRAM_ID);
                var recipientAccount = yield (0, spl_token_1.getAccount)(this.instance.provider.connection, this.params.accounts.recipient, spl_token_1.TOKEN_PROGRAM_ID);
                // assert(senderAccount.amount == ((I64(Number(senderAccountBalancePriorLastTx)).add(I64.readLE(this.extAmount, 0))).sub(I64(relayerFee))).toString(), "amount not transferred correctly");
                console.log("this.recipientBalancePriorTx ", this.recipientBalancePriorTx);
                console.log("this.publicAmount ", this.publicAmount);
                console.log("this.publicAmount ", (_b = this.publicAmount) === null || _b === void 0 ? void 0 : _b.sub(index_1.FIELD_SIZE).mod(index_1.FIELD_SIZE));
                console.log(`${recipientAccount.amount}, ${new anchor.BN(this.recipientBalancePriorTx)
                    .sub((_c = this.publicAmount) === null || _c === void 0 ? void 0 : _c.sub(index_1.FIELD_SIZE).mod(index_1.FIELD_SIZE))
                    .toString()}`);
                assert.equal(recipientAccount.amount.toString(), new anchor.BN(this.recipientBalancePriorTx)
                    .sub((_d = this.publicAmount) === null || _d === void 0 ? void 0 : _d.sub(index_1.FIELD_SIZE).mod(index_1.FIELD_SIZE))
                    .toString(), "amount not transferred correctly");
                var relayerAccount = yield this.instance.provider.connection.getBalance(this.relayer.accounts.relayerRecipient);
                var recipientFeeAccount = yield this.instance.provider.connection.getBalance(this.params.accounts.recipientFee);
                // console.log("recipientFeeAccount ", recipientFeeAccount);
                // console.log("this.feeAmount: ", this.feeAmount);
                // console.log(
                //   "recipientFeeBalancePriorTx ",
                //   this.recipientFeeBalancePriorTx
                // );
                console.log(`recipientFeeAccount ${new anchor.BN(recipientFeeAccount)
                    .add(new anchor.BN(this.relayer.relayerFee.toString()))
                    .add(new anchor.BN("5000"))
                    .toString()} == ${new anchor.BN(this.recipientFeeBalancePriorTx)
                    .sub((_e = this.feeAmount) === null || _e === void 0 ? void 0 : _e.sub(index_1.FIELD_SIZE).mod(index_1.FIELD_SIZE))
                    .toString()}`);
                // console.log("relayerAccount ", relayerAccount);
                // console.log("this.relayer.relayerFee: ", this.relayer.relayerFee);
                console.log("relayerRecipientAccountBalancePriorLastTx ", this.relayerRecipientAccountBalancePriorLastTx);
                console.log(`relayerFeeAccount ${new anchor.BN(relayerAccount)
                    .sub(this.relayer.relayerFee)
                    .toString()} == ${new anchor.BN(this.relayerRecipientAccountBalancePriorLastTx)}`);
                console.log(`relayerAccount ${new anchor.BN(relayerAccount).toString()} == ${new anchor.BN(this.relayerRecipientAccountBalancePriorLastTx)
                    .sub(new anchor.BN(this.relayer.relayerFee))
                    .toString()}`);
                console.log(`recipientFeeAccount ${new anchor.BN(recipientFeeAccount)
                    .add(new anchor.BN(this.relayer.relayerFee.toString()))
                    .toString()}  == ${new anchor.BN(this.recipientFeeBalancePriorTx)
                    .sub((_f = this.feeAmount) === null || _f === void 0 ? void 0 : _f.sub(index_1.FIELD_SIZE).mod(index_1.FIELD_SIZE))
                    .toString()}`);
                assert.equal(new anchor.BN(recipientFeeAccount)
                    .add(new anchor.BN(this.relayer.relayerFee.toString()))
                    .toString(), new anchor.BN(this.recipientFeeBalancePriorTx)
                    .sub((_g = this.feeAmount) === null || _g === void 0 ? void 0 : _g.sub(index_1.FIELD_SIZE).mod(index_1.FIELD_SIZE))
                    .toString());
                // console.log(`this.relayer.relayerFee ${this.relayer.relayerFee} new anchor.BN(relayerAccount) ${new anchor.BN(relayerAccount)}`);
                assert.equal(new anchor.BN(relayerAccount)
                    .sub(this.relayer.relayerFee)
                    // .add(new anchor.BN("5000"))
                    .toString(), (_h = this.relayerRecipientAccountBalancePriorLastTx) === null || _h === void 0 ? void 0 : _h.toString());
            }
            else {
                throw Error("mode not supplied");
            }
        });
    }
    // TODO: use higher entropy rnds
    shuffleUtxos(utxos) {
        if (this.shuffleEnabled) {
            console.log("shuffling utxos");
        }
        else {
            console.log("commented shuffle");
            return;
        }
        let currentIndex = utxos.length;
        let randomIndex;
        // While there remain elements to shuffle...
        while (0 !== currentIndex) {
            // Pick a remaining element...
            randomIndex = Math.floor(Math.random() * currentIndex);
            currentIndex--;
            // And swap it with the current element.
            [utxos[currentIndex], utxos[randomIndex]] = [
                utxos[randomIndex],
                utxos[currentIndex],
            ];
        }
        return utxos;
    }
    // also converts lE to BE
    static parseProofToBytesArray(data) {
        return __awaiter(this, void 0, void 0, function* () {
            var mydata = JSON.parse(data.toString());
            for (var i in mydata) {
                if (i == "pi_a" || i == "pi_c") {
                    for (var j in mydata[i]) {
                        mydata[i][j] = Array.from(leInt2Buff(unstringifyBigInts(mydata[i][j]), 32)).reverse();
                    }
                }
                else if (i == "pi_b") {
                    for (var j in mydata[i]) {
                        for (var z in mydata[i][j]) {
                            mydata[i][j][z] = Array.from(leInt2Buff(unstringifyBigInts(mydata[i][j][z]), 32));
                        }
                    }
                }
            }
            return [
                mydata.pi_a[0],
                mydata.pi_a[1],
                mydata.pi_b[0].flat().reverse(),
                mydata.pi_b[1].flat().reverse(),
                mydata.pi_c[0],
                mydata.pi_c[1],
            ].flat();
        });
    }
    ;
}
exports.Transaction = Transaction;
