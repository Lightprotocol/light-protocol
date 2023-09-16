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
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.TransactionParameters = void 0;
const web3_js_1 = require("@solana/web3.js");
const anchor = __importStar(require("@coral-xyz/anchor"));
const spl_token_1 = require("@solana/spl-token");
const anchor_1 = require("@coral-xyz/anchor");
const constants_1 = require("../constants");
const utxo_1 = require("../utxo");
const merkleTreeConfig_1 = require("../merkleTree/merkleTreeConfig");
const index_1 = require("../index");
const sha256_1 = require("@noble/hashes/sha256");
const spl_account_compression_1 = require("@solana/spl-account-compression");
const tweetnacl_1 = __importDefault(require("tweetnacl"));
class TransactionParameters {
    constructor({ message, eventMerkleTreePubkey, transactionMerkleTreePubkey, senderSpl, recipientSpl, senderSol, recipientSol, inputUtxos, outputUtxos, relayer, encryptedUtxos, poseidon, action, ataCreationFee, verifierIdl, }) {
        if (!outputUtxos && !inputUtxos) {
            throw new index_1.TransactionParametersError(index_1.TransactionErrorCode.NO_UTXOS_PROVIDED, "constructor", "");
        }
        if (!verifierIdl) {
            throw new index_1.TransactionParametersError(index_1.TransactionParametersErrorCode.NO_VERIFIER_IDL_PROVIDED, "constructor", "");
        }
        if (!poseidon) {
            throw new index_1.TransactionParametersError(index_1.TransactionParametersErrorCode.NO_POSEIDON_HASHER_PROVIDED, "constructor", "");
        }
        if (!action) {
            throw new index_1.TransactionParametersError(index_1.TransactionParametersErrorCode.NO_ACTION_PROVIDED, "constructor", "Define an action either Action.TRANSFER, Action.SHIELD,Action.UNSHIELD");
        }
        this.verifierProgramId =
            TransactionParameters.getVerifierProgramId(verifierIdl);
        this.verifierConfig = TransactionParameters.getVerifierConfig(verifierIdl);
        this.message = message;
        this.verifierIdl = verifierIdl;
        this.poseidon = poseidon;
        this.ataCreationFee = ataCreationFee;
        this.encryptedUtxos = encryptedUtxos;
        this.action = action;
        this.inputUtxos = this.addEmptyUtxos(inputUtxos, this.verifierConfig.in);
        this.outputUtxos = this.addEmptyUtxos(outputUtxos, this.verifierConfig.out);
        if (action === index_1.Action.SHIELD && senderSol) {
            this.relayer = new index_1.Relayer(senderSol);
        }
        else if (action === index_1.Action.SHIELD && !senderSol) {
            throw new index_1.TransactionParametersError(index_1.TransactionErrorCode.SOL_SENDER_UNDEFINED, "constructor", "Sender sol always needs to be defined because we use it as the signer to instantiate the relayer object.");
        }
        if (action !== index_1.Action.SHIELD) {
            if (relayer) {
                this.relayer = relayer;
            }
            else {
                throw new index_1.TransactionParametersError(index_1.TransactionErrorCode.RELAYER_UNDEFINED, "constructor", "For a transfer or withdrawal a relayer needs to be provided.");
            }
        }
        const pubkeys = TransactionParameters.getAssetPubkeys(this.inputUtxos, this.outputUtxos);
        this.assetPubkeys = pubkeys.assetPubkeys;
        this.assetPubkeysCircuit = pubkeys.assetPubkeysCircuit;
        this.publicAmountSol = TransactionParameters.getExternalAmount(0, this.inputUtxos, this.outputUtxos, this.assetPubkeysCircuit);
        this.publicAmountSpl = TransactionParameters.getExternalAmount(1, this.inputUtxos, this.outputUtxos, this.assetPubkeysCircuit);
        // safeguard should not be possible
        if (!this.publicAmountSol.gte(index_1.BN_0))
            throw new index_1.TransactionParametersError(index_1.TransactionParametersErrorCode.PUBLIC_AMOUNT_NEGATIVE, "constructor", "Public sol amount cannot be negative.");
        if (!this.publicAmountSpl.gte(index_1.BN_0))
            throw new index_1.TransactionParametersError(index_1.TransactionParametersErrorCode.PUBLIC_AMOUNT_NEGATIVE, "constructor", "Public spl amount cannot be negative.");
        // Checking plausibility of inputs
        if (this.action === index_1.Action.SHIELD) {
            /**
             * No relayer
             * public amounts are u64s
             * senderSpl is the user
             * recipientSpl is the merkle tree
             */
            if (relayer)
                throw new index_1.TransactionParametersError(index_1.TransactionParametersErrorCode.RELAYER_DEFINED, "constructor", "For a deposit no relayer should to be provided, the user send the transaction herself.");
            try {
                this.publicAmountSol.toArray("be", 8);
            }
            catch (error) {
                throw new index_1.TransactionParametersError(index_1.TransactionParametersErrorCode.PUBLIC_AMOUNT_NOT_U64, "constructor", `Public amount sol ${this.publicAmountSol} needs to be a u64 at deposit. Check whether you defined input and output utxos correctly, for a deposit the amounts of output utxos need to be bigger than the amounts of input utxos`);
            }
            try {
                this.publicAmountSpl.toArray("be", 8);
            }
            catch (error) {
                throw new index_1.TransactionParametersError(index_1.TransactionParametersErrorCode.PUBLIC_AMOUNT_NOT_U64, "constructor", `Public amount spl ${this.publicAmountSpl} needs to be a u64 at deposit. Check whether you defined input and output utxos correctly, for a deposit the amounts of output utxos need to be bigger than the amounts of input utxos`);
            }
            if (!this.publicAmountSol.eq(index_1.BN_0) && recipientSol) {
                throw new index_1.TransactionParametersError(index_1.TransactionParametersErrorCode.SOL_RECIPIENT_DEFINED, "constructor", "");
            }
            if (!this.publicAmountSpl.eq(index_1.BN_0) && recipientSpl) {
                throw new index_1.TransactionParametersError(index_1.TransactionParametersErrorCode.SPL_RECIPIENT_DEFINED, "constructor", "");
            }
            if (!this.publicAmountSol.eq(index_1.BN_0) && !senderSol) {
                throw new index_1.TransactionParametersError(index_1.TransactionErrorCode.SOL_SENDER_UNDEFINED, "constructor", "");
            }
            if (!this.publicAmountSpl.eq(index_1.BN_0) && !senderSpl) {
                throw new index_1.TransactionParametersError(index_1.TransactionErrorCode.SPL_SENDER_UNDEFINED, "constructor", "");
            }
        }
        else if (this.action === index_1.Action.UNSHIELD) {
            /**
             * relayer is defined
             * public amounts sub FieldSize are negative or 0
             * for public amounts greater than 0 a recipientSpl needs to be defined
             * senderSpl is the merkle tree
             * recipientSpl is the user
             */
            // TODO: should I throw an error when a lookup table is defined?
            if (!relayer)
                throw new index_1.TransactionParametersError(index_1.TransactionErrorCode.RELAYER_UNDEFINED, "constructor", "For a withdrawal a relayer needs to be provided.");
            // public amount is either 0 or negative
            // this.publicAmountSol.add(FIELD_SIZE).mod(FIELD_SIZE) this changes the value
            const tmpSol = this.publicAmountSol;
            if (!tmpSol.sub(index_1.FIELD_SIZE).lte(index_1.BN_0))
                throw new index_1.TransactionParametersError(index_1.TransactionParametersErrorCode.INVALID_PUBLIC_AMOUNT, "constructor", "");
            const tmpSpl = this.publicAmountSpl;
            if (!tmpSpl.sub(index_1.FIELD_SIZE).lte(index_1.BN_0))
                throw new index_1.TransactionParametersError(index_1.TransactionParametersErrorCode.INVALID_PUBLIC_AMOUNT, "constructor", "");
            try {
                if (!tmpSol.eq(index_1.BN_0)) {
                    tmpSol.sub(index_1.FIELD_SIZE).toArray("be", 8);
                }
            }
            catch (error) {
                throw new index_1.TransactionParametersError(index_1.TransactionParametersErrorCode.PUBLIC_AMOUNT_NOT_U64, "constructor", "Public amount needs to be a u64 at deposit.");
            }
            try {
                if (!tmpSpl.eq(index_1.BN_0)) {
                    tmpSpl.sub(index_1.FIELD_SIZE).toArray("be", 8);
                }
            }
            catch (error) {
                throw new index_1.TransactionParametersError(index_1.TransactionParametersErrorCode.PUBLIC_AMOUNT_NOT_U64, "constructor", "Public amount needs to be a u64 at deposit.");
            }
            if (!this.publicAmountSol.eq(index_1.BN_0) && !recipientSol) {
                throw new index_1.TransactionParametersError(index_1.TransactionErrorCode.SOL_RECIPIENT_UNDEFINED, "constructor", "");
            }
            if (!this.publicAmountSpl.eq(index_1.BN_0) && !recipientSpl) {
                throw new index_1.TransactionParametersError(index_1.TransactionErrorCode.SPL_RECIPIENT_UNDEFINED, "constructor", "");
            }
            // && senderSol.toBase58() != merkle tree token pda
            if (!this.publicAmountSol.eq(index_1.BN_0) && senderSol) {
                throw new index_1.TransactionParametersError(index_1.TransactionParametersErrorCode.SOL_SENDER_DEFINED, "constructor", "");
            }
            if (!this.publicAmountSpl.eq(index_1.BN_0) && senderSpl) {
                throw new index_1.TransactionParametersError(index_1.TransactionParametersErrorCode.SPL_SENDER_DEFINED, "constructor", "");
            }
        }
        else if (this.action === index_1.Action.TRANSFER) {
            /**
             * relayer is defined
             * public amount spl amount is 0
             * public amount spl amount sub FieldSize is equal to the relayer fee
             * senderSpl is the merkle tree
             * recipientSpl does not exists it is an internal transfer just the relayer is paid
             */
            if (!relayer)
                throw new index_1.TransactionParametersError(index_1.TransactionErrorCode.RELAYER_UNDEFINED, "constructor", "For a transfer a relayer needs to be provided.");
            if (!this.publicAmountSpl.eq(index_1.BN_0))
                throw new index_1.TransactionParametersError(index_1.TransactionParametersErrorCode.PUBLIC_AMOUNT_SPL_NOT_ZERO, "constructor", `For a transfer public spl amount needs to be zero ${this.publicAmountSpl}`);
            const tmpSol = this.publicAmountSol;
            if (!tmpSol
                .sub(index_1.FIELD_SIZE)
                .mul(new anchor_1.BN(-1))
                .eq(relayer.getRelayerFee(ataCreationFee)))
                throw new index_1.TransactionParametersError(index_1.TransactionParametersErrorCode.PUBLIC_AMOUNT_SOL_NOT_ZERO, "constructor", `public amount ${tmpSol
                    .sub(index_1.FIELD_SIZE)
                    .mul(new anchor_1.BN(-1))}  should be ${relayer.getRelayerFee(ataCreationFee)}`);
            if (recipientSpl) {
                throw new index_1.TransactionParametersError(index_1.TransactionParametersErrorCode.SPL_RECIPIENT_DEFINED, "constructor", "This is a transfer, no spl amount should be withdrawn. To withdraw an spl amount mark the transaction as withdrawal.");
            }
            if (recipientSol) {
                throw new index_1.TransactionParametersError(index_1.TransactionParametersErrorCode.SOL_RECIPIENT_DEFINED, "constructor", "This is a transfer, no sol amount should be withdrawn. To withdraw an sol amount mark the transaction as withdrawal.");
            }
            if (senderSol) {
                throw new index_1.TransactionParametersError(index_1.TransactionParametersErrorCode.SOL_SENDER_DEFINED, "constructor", "");
            }
            if (senderSpl) {
                throw new index_1.TransactionParametersError(index_1.TransactionParametersErrorCode.SPL_SENDER_DEFINED, "constructor", "");
            }
        }
        else {
            throw new index_1.TransactionParametersError(index_1.TransactionParametersErrorCode.NO_ACTION_PROVIDED, "constructor", "");
        }
        this.accounts = {
            systemProgramId: web3_js_1.SystemProgram.programId,
            tokenProgram: spl_token_1.TOKEN_PROGRAM_ID,
            logWrapper: spl_account_compression_1.SPL_NOOP_PROGRAM_ID,
            eventMerkleTree: eventMerkleTreePubkey,
            transactionMerkleTree: transactionMerkleTreePubkey,
            registeredVerifierPda: index_1.Transaction.getRegisteredVerifierPda(index_1.merkleTreeProgramId, this.verifierProgramId),
            authority: index_1.Transaction.getSignerAuthorityPda(index_1.merkleTreeProgramId, this.verifierProgramId),
            senderSpl: senderSpl,
            recipientSpl: recipientSpl,
            senderSol: senderSol,
            recipientSol: recipientSol,
            programMerkleTree: index_1.merkleTreeProgramId,
            tokenAuthority: index_1.Transaction.getTokenAuthority(),
            verifierProgram: this.verifierProgramId,
        };
        this.assignAccounts();
        // @ts-ignore:
        this.accounts.signingAddress = this.relayer.accounts.relayerPubkey;
    }
    async toBytes() {
        let utxo;
        let coder = new anchor_1.BorshAccountsCoder(index_1.IDL_VERIFIER_PROGRAM_ZERO);
        let inputUtxosBytes = [];
        for (utxo of this.inputUtxos) {
            inputUtxosBytes.push(await utxo.toBytes());
        }
        let outputUtxosBytes = [];
        for (utxo of this.outputUtxos) {
            outputUtxosBytes.push(await utxo.toBytes());
        }
        let preparedObject = {
            outputUtxosBytes,
            inputUtxosBytes,
            relayerPubkey: this.relayer.accounts.relayerPubkey,
            relayerFee: this.relayer.relayerFee,
            ...this,
            ...this.accounts,
        };
        return await coder.encode("transactionParameters", preparedObject);
    }
    static findIdlIndex(programId, idlObjects) {
        for (let i = 0; i < idlObjects.length; i++) {
            const constants = idlObjects[i].constants;
            if (!constants)
                throw new Error(`Idl in index ${i} does not have any constants`);
            for (const constant of constants) {
                if (constant.name === "PROGRAM_ID" &&
                    constant.type === "string" &&
                    constant.value === `"${programId}"`) {
                    return i;
                }
            }
        }
        return -1; // Return -1 if the programId is not found in any IDL object
    }
    static getVerifierProgramId(verifierIdl) {
        const programIdObj = verifierIdl.constants.find((constant) => constant.name === "PROGRAM_ID");
        if (!programIdObj || typeof programIdObj.value !== "string") {
            throw new index_1.TransactionParametersError(index_1.TransactionParametersErrorCode.PROGRAM_ID_CONSTANT_UNDEFINED, 'PROGRAM_ID constant not found in idl. Example: pub const PROGRAM_ID: &str = "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS";');
        }
        // Extracting the public key string value from the object and removing quotes.
        const programIdStr = programIdObj.value.slice(1, -1);
        return new web3_js_1.PublicKey(programIdStr);
    }
    static getVerifierProgram(verifierIdl, anchorProvider) {
        const programId = TransactionParameters.getVerifierProgramId(verifierIdl);
        const verifierProgram = new anchor_1.Program(verifierIdl, programId, anchorProvider);
        return verifierProgram;
    }
    static getVerifierConfig(verifierIdl) {
        const accounts = verifierIdl.accounts;
        const resultElement = accounts.find((account) => account.name.startsWith("zK") && account.name.endsWith("ProofInputs"));
        if (!resultElement) {
            throw new Error("No matching element found");
        }
        const fields = resultElement.type.fields;
        const inputNullifierField = fields.find((field) => field.name === "inputNullifier");
        const outputCommitmentField = fields.find((field) => field.name === "outputCommitment");
        if (!inputNullifierField || !inputNullifierField.type.array) {
            throw new Error("inputNullifier field not found or has an incorrect type");
        }
        if (!outputCommitmentField || !outputCommitmentField.type.array) {
            throw new Error("outputCommitment field not found or has an incorrect type");
        }
        const inputNullifierLength = inputNullifierField.type.array[1];
        const outputCommitmentLength = outputCommitmentField.type.array[1];
        return { in: inputNullifierLength, out: outputCommitmentLength };
    }
    static async fromBytes({ poseidon, utxoIdls, bytes, relayer, verifierIdl, assetLookupTable, verifierProgramLookupTable, }) {
        let coder = new anchor_1.BorshAccountsCoder(index_1.IDL_VERIFIER_PROGRAM_ZERO);
        let decoded = coder.decodeUnchecked("transactionParameters", bytes);
        const getUtxos = (utxoBytesArray, utxoIdls) => {
            let utxos = [];
            for (var [_, utxoBytes] of utxoBytesArray.entries()) {
                let appDataIdl = undefined;
                if (utxoBytes.subarray(128, 160).toString() !==
                    Buffer.alloc(32).fill(0).toString()) {
                    if (!utxoIdls) {
                        throw new index_1.TransactionParametersError(index_1.TransactionParametersErrorCode.UTXO_IDLS_UNDEFINED, "fromBytes");
                    }
                    let idlIndex = TransactionParameters.findIdlIndex(new web3_js_1.PublicKey(utxoBytes.subarray(128, 160)).toBase58(), utxoIdls);
                    // could add option to fetch idl from chain if not found
                    appDataIdl = utxoIdls[idlIndex];
                }
                utxos.push(utxo_1.Utxo.fromBytes({
                    poseidon,
                    bytes: utxoBytes,
                    appDataIdl,
                    assetLookupTable,
                    verifierProgramLookupTable,
                }));
            }
            return utxos;
        };
        const inputUtxos = getUtxos(decoded.inputUtxosBytes, utxoIdls);
        const outputUtxos = getUtxos(decoded.outputUtxosBytes, utxoIdls);
        if (relayer &&
            relayer.accounts.relayerPubkey.toBase58() != decoded.relayerPubkey) {
            // TODO: add functionality to look up relayer or fetch info, looking up is better
            throw new index_1.TransactionParametersError(index_1.TransactionParametersErrorCode.RELAYER_INVALID, "fromBytes", "The provided relayer has a different public key as the relayer publickey decoded from bytes");
        }
        if (!relayer) {
            throw new index_1.TransactionParametersError(index_1.TransactionErrorCode.RELAYER_UNDEFINED, "fromBytes");
        }
        let action = index_1.Action.TRANSFER;
        if (decoded.recipientSol.toBase58() !== constants_1.AUTHORITY.toBase58() ||
            decoded.recipientSpl.toBase58() !== constants_1.AUTHORITY.toBase58()) {
            action = index_1.Action.UNSHIELD;
        }
        else {
            decoded.recipientSol = undefined;
            decoded.recipientSpl = undefined;
        }
        return new TransactionParameters({
            poseidon,
            inputUtxos,
            outputUtxos,
            relayer,
            ...decoded,
            action,
            transactionMerkleTreePubkey: merkleTreeConfig_1.MerkleTreeConfig.getTransactionMerkleTreePda(),
            verifierIdl: verifierIdl,
        });
    }
    static async getTxParams({ tokenCtx, publicAmountSpl = index_1.BN_0, publicAmountSol = index_1.BN_0, action, userSplAccount = constants_1.AUTHORITY, account, utxos, inUtxos, 
    // for unshield
    recipientSol, recipientSplAddress, 
    // for transfer
    outUtxos, relayer, provider, ataCreationFee, // associatedTokenAccount = ata
    appUtxo, addInUtxos = true, addOutUtxos = true, verifierIdl, mergeUtxos = false, message, assetLookupTable, verifierProgramLookupTable, separateSolUtxo = false, }) {
        if (action === index_1.Action.TRANSFER && !outUtxos && !mergeUtxos)
            throw new index_1.TransactionParametersError(index_1.UserErrorCode.SHIELDED_RECIPIENT_UNDEFINED, "getTxParams", "Recipient outUtxo not provided for transfer");
        if (action !== index_1.Action.SHIELD && !relayer?.getRelayerFee(ataCreationFee)) {
            // TODO: could make easier to read by adding separate if/cases
            throw new index_1.TransactionParametersError(index_1.RelayerErrorCode.RELAYER_FEE_UNDEFINED, "getTxParams", `No relayerFee provided for ${action.toLowerCase()}}`);
        }
        if (!account) {
            throw new index_1.TransactionParametersError(index_1.CreateUtxoErrorCode.ACCOUNT_UNDEFINED, "getTxParams", "account for change utxo is undefined");
        }
        var inputUtxos = inUtxos ? [...inUtxos] : [];
        var outputUtxos = outUtxos ? [...outUtxos] : [];
        if (addInUtxos) {
            inputUtxos = (0, index_1.selectInUtxos)({
                publicMint: tokenCtx.mint,
                publicAmountSpl,
                publicAmountSol,
                poseidon: provider.poseidon,
                inUtxos,
                outUtxos,
                utxos,
                relayerFee: relayer?.getRelayerFee(ataCreationFee),
                action,
                numberMaxInUtxos: TransactionParameters.getVerifierConfig(verifierIdl).in,
                numberMaxOutUtxos: TransactionParameters.getVerifierConfig(verifierIdl).out,
            });
        }
        if (addOutUtxos) {
            outputUtxos = (0, index_1.createOutUtxos)({
                publicMint: tokenCtx.mint,
                publicAmountSpl,
                inUtxos: inputUtxos,
                publicAmountSol,
                poseidon: provider.poseidon,
                relayerFee: relayer?.getRelayerFee(ataCreationFee),
                changeUtxoAccount: account,
                outUtxos,
                action,
                appUtxo,
                numberMaxOutUtxos: TransactionParameters.getVerifierConfig(verifierIdl).out,
                assetLookupTable,
                verifierProgramLookupTable,
                separateSolUtxo,
            });
        }
        let txParams = new TransactionParameters({
            outputUtxos,
            inputUtxos,
            transactionMerkleTreePubkey: merkleTreeConfig_1.MerkleTreeConfig.getTransactionMerkleTreePda(),
            senderSpl: action === index_1.Action.SHIELD ? userSplAccount : undefined,
            senderSol: action === index_1.Action.SHIELD ? provider.wallet.publicKey : undefined,
            recipientSpl: recipientSplAddress,
            recipientSol,
            poseidon: provider.poseidon,
            action,
            relayer: relayer,
            ataCreationFee,
            verifierIdl,
            message,
            eventMerkleTreePubkey: merkleTreeConfig_1.MerkleTreeConfig.getEventMerkleTreePda(),
        });
        return txParams;
    }
    /**
     * @description Adds empty utxos until the desired number of utxos is reached.
     * @note The zero knowledge proof circuit needs all inputs to be defined.
     * @note Therefore, we have to pass in empty inputs for values we don't use.
     * @param utxos
     * @param len
     * @returns
     */
    addEmptyUtxos(utxos = [], len) {
        while (utxos.length < len) {
            utxos.push(new utxo_1.Utxo({
                poseidon: this.poseidon,
                assetLookupTable: [web3_js_1.SystemProgram.programId.toBase58()],
                verifierProgramLookupTable: [web3_js_1.SystemProgram.programId.toBase58()],
            }));
        }
        return utxos;
    }
    /**
     * @description Assigns spl and sol senderSpl or recipientSpl accounts to transaction parameters based on action.
     */
    assignAccounts() {
        if (!this.assetPubkeys)
            throw new index_1.TransactionParametersError(index_1.TransactionErrorCode.ASSET_PUBKEYS_UNDEFINED, "assignAccounts assetPubkeys undefined", "assignAccounts");
        if (this.action.toString() === index_1.Action.UNSHIELD.toString() ||
            this.action.toString() === index_1.Action.TRANSFER.toString()) {
            this.accounts.senderSpl = merkleTreeConfig_1.MerkleTreeConfig.getSplPoolPdaToken(this.assetPubkeys[1], index_1.merkleTreeProgramId);
            this.accounts.senderSol =
                merkleTreeConfig_1.MerkleTreeConfig.getSolPoolPda(index_1.merkleTreeProgramId).pda;
            if (!this.accounts.recipientSpl) {
                // AUTHORITY is used as place holder
                this.accounts.recipientSpl = constants_1.AUTHORITY;
                if (!this.publicAmountSpl?.eq(index_1.BN_0)) {
                    throw new index_1.TransactionError(index_1.TransactionErrorCode.SPL_RECIPIENT_UNDEFINED, "assignAccounts", "Spl recipientSpl is undefined while public spl amount is != 0.");
                }
            }
            if (!this.accounts.recipientSol) {
                // AUTHORITY is used as place holder
                this.accounts.recipientSol = constants_1.AUTHORITY;
                if (!this.publicAmountSol.eq(index_1.BN_0) &&
                    !this.publicAmountSol
                        ?.sub(index_1.FIELD_SIZE)
                        .mul(new anchor_1.BN(-1))
                        .sub(new anchor_1.BN(this.relayer.getRelayerFee(this.ataCreationFee)))
                        .eq(index_1.BN_0)) {
                    throw new index_1.TransactionParametersError(index_1.TransactionErrorCode.SOL_RECIPIENT_UNDEFINED, "assignAccounts", "Sol recipientSpl is undefined while public spl amount is != 0.");
                }
            }
        }
        else {
            if (this.action.toString() !== index_1.Action.SHIELD.toString()) {
                throw new index_1.TransactionParametersError(index_1.TransactionErrorCode.ACTION_IS_NO_DEPOSIT, "assignAccounts", "Action is withdrawal but should not be. Spl & sol senderSpl accounts are provided and a relayer which is used to identify transfers and withdrawals. For a deposit do not provide a relayer.");
            }
            this.accounts.recipientSpl = merkleTreeConfig_1.MerkleTreeConfig.getSplPoolPdaToken(this.assetPubkeys[1], index_1.merkleTreeProgramId);
            this.accounts.recipientSol =
                merkleTreeConfig_1.MerkleTreeConfig.getSolPoolPda(index_1.merkleTreeProgramId).pda;
            if (!this.accounts.senderSpl) {
                /// assigning a placeholder account
                this.accounts.senderSpl = constants_1.AUTHORITY;
                if (!this.publicAmountSpl?.eq(index_1.BN_0)) {
                    throw new index_1.TransactionParametersError(index_1.TransactionErrorCode.SPL_SENDER_UNDEFINED, "assignAccounts", "Spl senderSpl is undefined while public spl amount is != 0.");
                }
            }
            this.accounts.senderSol = TransactionParameters.getEscrowPda(this.verifierProgramId);
        }
    }
    static getEscrowPda(verifierProgramId) {
        return web3_js_1.PublicKey.findProgramAddressSync([anchor.utils.bytes.utf8.encode("escrow")], verifierProgramId)[0];
    }
    static getAssetPubkeys(inputUtxos, outputUtxos) {
        let assetPubkeysCircuit = [
            (0, index_1.hashAndTruncateToCircuit)(web3_js_1.SystemProgram.programId.toBytes()).toString(),
        ];
        let assetPubkeys = [web3_js_1.SystemProgram.programId];
        if (inputUtxos) {
            inputUtxos.map((utxo) => {
                let found = false;
                if (assetPubkeysCircuit.indexOf(utxo.assetsCircuit[1].toString()) !== -1) {
                    found = true;
                }
                if (!found && utxo.assetsCircuit[1].toString() != "0") {
                    assetPubkeysCircuit.push(utxo.assetsCircuit[1].toString());
                    assetPubkeys.push(utxo.assets[1]);
                }
            });
        }
        if (outputUtxos) {
            outputUtxos.map((utxo) => {
                let found = false;
                for (var _asset in assetPubkeysCircuit) {
                    if (assetPubkeysCircuit.indexOf(utxo.assetsCircuit[1].toString()) !== -1) {
                        found = true;
                    }
                }
                if (!found && utxo.assetsCircuit[1].toString() != "0") {
                    assetPubkeysCircuit.push(utxo.assetsCircuit[1].toString());
                    assetPubkeys.push(utxo.assets[1]);
                }
            });
        }
        if ((!inputUtxos && !outputUtxos) ||
            (inputUtxos?.length == 0 && outputUtxos?.length == 0)) {
            throw new index_1.TransactionError(index_1.TransactionErrorCode.NO_UTXOS_PROVIDED, "getAssetPubkeys", "No input or output utxos provided.");
        }
        // TODO: test this better
        // if (assetPubkeys.length > params?.verifier.config.out) {
        //   throw new TransactionError(
        //     TransactionErrorCode.EXCEEDED_MAX_ASSETS,
        //     "getAssetPubkeys",
        //     `Utxos contain too many different assets ${params?.verifier.config.out} > max allowed: ${N_ASSET_PUBKEYS}`,
        //   );
        // }
        if (assetPubkeys.length > utxo_1.N_ASSET_PUBKEYS) {
            throw new index_1.TransactionError(index_1.TransactionErrorCode.EXCEEDED_MAX_ASSETS, "getAssetPubkeys", `Utxos contain too many different assets ${assetPubkeys.length} > max allowed: ${utxo_1.N_ASSET_PUBKEYS}`);
        }
        while (assetPubkeysCircuit.length < utxo_1.N_ASSET_PUBKEYS) {
            assetPubkeysCircuit.push(index_1.BN_0.toString());
            assetPubkeys.push(web3_js_1.SystemProgram.programId);
        }
        return { assetPubkeysCircuit, assetPubkeys };
    }
    /**
     * @description Calculates the external amount for one asset.
     * @note This function might be too specific since the circuit allows assets to be in any index
     * @param assetIndex the index of the asset the external amount should be computed for
     * @returns {BN} the public amount of the asset
     */
    static getExternalAmount(assetIndex, 
    // params: TransactionParameters,
    inputUtxos, outputUtxos, assetPubkeysCircuit) {
        return new anchor.BN(0)
            .add(outputUtxos
            .filter((utxo) => {
            return (utxo.assetsCircuit[assetIndex].toString() ==
                assetPubkeysCircuit[assetIndex]);
        })
            .reduce((sum, utxo) => 
        // add all utxos of the same asset
        sum.add(utxo.amounts[assetIndex]), new anchor.BN(0)))
            .sub(inputUtxos
            .filter((utxo) => {
            return (utxo.assetsCircuit[assetIndex].toString() ==
                assetPubkeysCircuit[assetIndex]);
        })
            .reduce((sum, utxo) => sum.add(utxo.amounts[assetIndex]), new anchor.BN(0)))
            .add(index_1.FIELD_SIZE)
            .mod(index_1.FIELD_SIZE);
    }
    /**
     * Computes the integrity Poseidon hash over transaction inputs that are not part of
     * the proof, but are included to prevent the relayer from changing any input of the
     * transaction.
     *
     * The hash is computed over the following inputs in the given order:
     * 1. Recipient SPL Account
     * 2. Recipient Solana Account
     * 3. Relayer Public Key
     * 4. Relayer Fee
     * 5. Encrypted UTXOs (limited to 512 bytes)
     *
     * @param {any} poseidon - Poseidon hash function instance.
     * @returns {Promise<BN>} A promise that resolves to the computed transaction integrity hash.
     * @throws {TransactionError} Throws an error if the relayer, recipient SPL or Solana accounts,
     * relayer fee, or encrypted UTXOs are undefined, or if the encryption of UTXOs fails.
     *
     * @example
     * const integrityHash = await getTxIntegrityHash(poseidonInstance);
     */
    async getTxIntegrityHash(poseidon) {
        if (!this.relayer)
            throw new index_1.TransactionError(index_1.TransactionErrorCode.RELAYER_UNDEFINED, "getTxIntegrityHash", "");
        if (!this.accounts.recipientSpl)
            throw new index_1.TransactionError(index_1.TransactionErrorCode.SPL_RECIPIENT_UNDEFINED, "getTxIntegrityHash", "");
        if (!this.accounts.recipientSol)
            throw new index_1.TransactionError(index_1.TransactionErrorCode.SOL_RECIPIENT_UNDEFINED, "getTxIntegrityHash", "");
        if (!this.relayer.getRelayerFee(this.ataCreationFee))
            throw new index_1.TransactionError(index_1.TransactionErrorCode.RELAYER_FEE_UNDEFINED, "getTxIntegrityHash", "");
        if (this.encryptedUtxos &&
            this.encryptedUtxos.length > 128 * this.verifierConfig.out)
            throw new index_1.TransactionParametersError(index_1.TransactionParametersErrorCode.ENCRYPTED_UTXOS_TOO_LONG, "getTxIntegrityHash", `Encrypted utxos are too long: ${this.encryptedUtxos.length} > ${128 * this.verifierConfig.out}`);
        if (!this.encryptedUtxos) {
            this.encryptedUtxos = await this.encryptOutUtxos(poseidon);
        }
        if (this.encryptedUtxos) {
            const relayerFee = new Uint8Array(this.relayer.getRelayerFee(this.ataCreationFee).toArray("le", 8));
            let nullifiersHasher = sha256_1.sha256.create();
            this.inputUtxos.forEach((x) => {
                const nullifier = x.getNullifier(poseidon);
                if (nullifier) {
                    let nullifierBytes = new anchor.BN(nullifier).toArray("be", 32);
                    nullifiersHasher.update(new Uint8Array(nullifierBytes));
                }
            });
            const nullifiersHash = nullifiersHasher.digest();
            let leavesHasher = sha256_1.sha256.create();
            this.outputUtxos.forEach((x) => {
                const commitment = new anchor.BN(x.getCommitment(poseidon)).toArray("be", 32);
                leavesHasher.update(new Uint8Array(commitment));
            });
            const leavesHash = leavesHasher.digest();
            const messageHash = this.message
                ? (0, sha256_1.sha256)(this.message)
                : new Uint8Array(32);
            const encryptedUtxosHash = sha256_1.sha256
                .create()
                .update(this.encryptedUtxos)
                .digest();
            const amountHash = sha256_1.sha256
                .create()
                .update(new Uint8Array(this.publicAmountSol.toArray("be", 32)))
                .update(new Uint8Array(this.publicAmountSpl.toArray("be", 32)))
                .update(relayerFee)
                .digest();
            const eventHash = sha256_1.sha256
                .create()
                .update(nullifiersHash)
                .update(leavesHash)
                .update(messageHash)
                .update(encryptedUtxosHash)
                .update(amountHash)
                .digest();
            // TODO(vadorovsky): Try to get rid of this hack during Verifier class
            // refactoring / removal
            // For example, we could derive which accounts exist in the IDL of the
            // verifier program method.
            const recipientSpl = this.verifierProgramId.toBase58() ===
                constants_1.verifierProgramStorageProgramId.toBase58()
                ? new Uint8Array(32)
                : this.accounts.recipientSpl.toBytes();
            const hash = sha256_1.sha256
                .create()
                .update(eventHash)
                .update(recipientSpl)
                .update(this.accounts.recipientSol.toBytes())
                .update(this.relayer.accounts.relayerPubkey.toBytes())
                .update(relayerFee)
                .update(this.encryptedUtxos)
                .digest();
            this.txIntegrityHash = new anchor.BN(hash).mod(index_1.FIELD_SIZE);
            return this.txIntegrityHash;
        }
        else {
            throw new index_1.TransactionError(index_1.TransactionErrorCode.ENCRYPTING_UTXOS_FAILED, "getTxIntegrityHash", "");
        }
    }
    async encryptOutUtxos(poseidon, encryptedUtxos) {
        let encryptedOutputs = new Array();
        if (encryptedUtxos) {
            encryptedOutputs = Array.from(encryptedUtxos);
        }
        else if (this && this.outputUtxos) {
            for (var utxo in this.outputUtxos) {
                if (this.outputUtxos[utxo].appDataHash.toString() !== "0" &&
                    this.outputUtxos[utxo].includeAppData)
                    throw new index_1.TransactionError(index_1.TransactionErrorCode.UNIMPLEMENTED, "encryptUtxos", "Automatic encryption for utxos with application data is not implemented.");
                encryptedOutputs.push(await this.outputUtxos[utxo].encrypt(poseidon, this.accounts.transactionMerkleTree));
            }
            encryptedOutputs = encryptedOutputs
                .map((elem) => Array.from(elem))
                .flat();
            if (encryptedOutputs.length < 128 * this.verifierConfig.out &&
                this.verifierConfig.out == 2) {
                return new Uint8Array([
                    ...encryptedOutputs,
                    ...new Array(128 * this.verifierConfig.out - encryptedOutputs.length).fill(0),
                    // for verifier zero and one these bytes are not sent and just added for the integrity hash
                    // to be consistent, if the bytes were sent to the chain use rnd bytes for padding
                ]);
            }
            if (encryptedOutputs.length < 128 * this.verifierConfig.out) {
                return new Uint8Array([
                    ...encryptedOutputs,
                    ...tweetnacl_1.default.randomBytes(128 * this.verifierConfig.out - encryptedOutputs.length),
                ]);
            }
        }
    }
}
exports.TransactionParameters = TransactionParameters;
//# sourceMappingURL=transactionParameters.js.map