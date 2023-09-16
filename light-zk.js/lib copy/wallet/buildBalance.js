"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.addUtxoToBalance = exports.buildUtxoBalanceFromUtxoBytes = exports.createUtxoBatches = exports.decryptAddUtxoToBalance = exports.ProgramBalance = exports.ProgramUtxoBalance = exports.TokenUtxoBalance = void 0;
const utxo_1 = require("../utxo");
const anchor_1 = require("@coral-xyz/anchor");
const web3_js_1 = require("@solana/web3.js");
const utils_1 = require("../utils");
const index_1 = require("../index");
// TODO: add nfts
class TokenUtxoBalance {
    constructor(tokenData) {
        this.tokenData = tokenData;
        this.totalBalanceSol = index_1.BN_0;
        this.totalBalanceSpl = index_1.BN_0;
        this.utxos = new Map();
        this.committedUtxos = new Map();
        this.spentUtxos = new Map();
    }
    static initSol() {
        return new TokenUtxoBalance(index_1.TOKEN_REGISTRY.get("SOL"));
    }
    addUtxo(commitment, utxo, attribute) {
        let utxoExists = this[attribute].get(commitment) !== undefined ? true : false;
        this[attribute].set(commitment, utxo);
        if (attribute === "utxos" && !utxoExists) {
            this.committedUtxos.delete(commitment);
            this.totalBalanceSol = this.totalBalanceSol.add(utxo.amounts[0]);
            if (utxo.amounts[1])
                this.totalBalanceSpl = this.totalBalanceSpl.add(utxo.amounts[1]);
        }
        return !utxoExists;
    }
    moveToSpentUtxos(commitment) {
        let utxo = this.utxos.get(commitment);
        if (!utxo)
            throw new index_1.TokenUtxoBalanceError(index_1.TokenUtxoBalanceErrorCode.UTXO_UNDEFINED, "moveToSpentUtxos", `utxo with committment ${commitment} does not exist in utxos`);
        this.totalBalanceSol = this.totalBalanceSol.sub(utxo.amounts[0]);
        if (utxo.amounts[1])
            this.totalBalanceSpl = this.totalBalanceSpl.sub(utxo.amounts[1]);
        this.spentUtxos.set(commitment, utxo);
        this.utxos.delete(commitment);
    }
}
exports.TokenUtxoBalance = TokenUtxoBalance;
class ProgramUtxoBalance {
    constructor(programAddress, programUtxoIdl) {
        this.programAddress = programAddress;
        this.programUtxoIdl = programUtxoIdl;
        this.tokenBalances = new Map();
    }
    addUtxo(commitment, utxo, attribute) {
        var _a;
        if (!utxo.verifierAddress) {
            throw new index_1.ProgramUtxoBalanceError(index_1.ProgramUtxoBalanceErrorCode.INVALID_PROGRAM_ADDRESS, "addUtxo", `Verifier address in utxo ${utxo._commitment} does not exist in utxo (trying to add utxo to program utxos balance)`);
        }
        if (!utxo.verifierAddress.equals(this.programAddress)) {
            throw new index_1.ProgramUtxoBalanceError(index_1.ProgramUtxoBalanceErrorCode.INVALID_PROGRAM_ADDRESS, "addUtxo", `Verifier address ${utxo.verifierAddress} does not match the program address (trying to add utxo to program utxos balance)`);
        }
        let utxoAsset = utxo.amounts[1].toString() === "0"
            ? new web3_js_1.PublicKey(0).toBase58()
            : utxo.assets[1].toBase58();
        let tokenBalance = (_a = this.tokenBalances) === null || _a === void 0 ? void 0 : _a.get(utxoAsset);
        // if not token balance for utxoAsset create token balance
        if (!tokenBalance) {
            const tokenSymbol = index_1.TOKEN_PUBKEY_SYMBOL.get(utxoAsset);
            if (!tokenSymbol)
                throw new index_1.ProgramUtxoBalanceError(index_1.UserErrorCode.TOKEN_NOT_FOUND, "addUtxo", `Token ${utxoAsset} not found when trying to add tokenBalance to PrograUtxoBalance for verifier ${this.programAddress.toBase58()}`);
            const tokenData = index_1.TOKEN_REGISTRY.get(tokenSymbol);
            if (!tokenData)
                throw new index_1.ProgramUtxoBalanceError(index_1.ProgramUtxoBalanceErrorCode.TOKEN_DATA_NOT_FOUND, "addUtxo", `Token ${utxoAsset} not found when trying to add tokenBalance to PrograUtxoBalance for verifier ${this.programAddress.toBase58()}`);
            this.tokenBalances.set(utxoAsset, new TokenUtxoBalance(tokenData));
        }
        return this.tokenBalances
            .get(utxoAsset)
            .addUtxo(commitment, utxo, attribute);
    }
}
exports.ProgramUtxoBalance = ProgramUtxoBalance;
class ProgramBalance extends TokenUtxoBalance {
    constructor(tokenData, programAddress, programUtxoIdl) {
        super(tokenData);
        this.programAddress = programAddress;
        this.programUtxoIdl = programUtxoIdl;
    }
    addProgramUtxo(commitment, utxo, attribute) {
        const utxoExists = this[attribute].get(commitment) !== undefined ? true : false;
        this[attribute].set(commitment, utxo);
        if (attribute === "utxos" && !utxoExists) {
            this.totalBalanceSol = this.totalBalanceSol.add(utxo.amounts[0]);
            if (utxo.amounts[1]) {
                this.totalBalanceSpl = this.totalBalanceSpl.add(utxo.amounts[1]);
            }
        }
        return !utxoExists;
    }
}
exports.ProgramBalance = ProgramBalance;
async function decryptAddUtxoToBalance({ account, encBytes, index, commitment, poseidon, connection, balance, merkleTreePdaPublicKey, leftLeaf, aes, verifierProgramLookupTable, assetLookupTable, }) {
    var _a;
    let decryptedUtxo = await utxo_1.Utxo.decrypt({
        poseidon,
        encBytes: encBytes,
        account: account,
        index: index,
        commitment,
        aes,
        merkleTreePdaPublicKey,
        verifierProgramLookupTable,
        assetLookupTable,
    });
    // null if utxo did not decrypt -> return nothing and continue
    if (!decryptedUtxo)
        return;
    const nullifier = decryptedUtxo.getNullifier(poseidon);
    if (!nullifier)
        return;
    const nullifierExists = await (0, utils_1.fetchNullifierAccountInfo)(nullifier, connection);
    const queuedLeavesPdaExists = await (0, utils_1.fetchQueuedLeavesAccountInfo)(leftLeaf, connection);
    const amountsValid = decryptedUtxo.amounts[1].toString() !== "0" ||
        decryptedUtxo.amounts[0].toString() !== "0";
    const assetIndex = decryptedUtxo.amounts[1].toString() !== "0" ? 1 : 0;
    // valid amounts and is not app utxo
    if (amountsValid &&
        decryptedUtxo.verifierAddress.toBase58() === new web3_js_1.PublicKey(0).toBase58() &&
        decryptedUtxo.appDataHash.toString() === "0") {
        // TODO: add is native to utxo
        // if !asset try to add asset and then push
        if (assetIndex &&
            !balance.tokenBalances.get(decryptedUtxo.assets[assetIndex].toBase58())) {
            // TODO: several maps or unify somehow
            let tokenBalanceUsdc = new TokenUtxoBalance(index_1.TOKEN_REGISTRY.get("USDC"));
            balance.tokenBalances.set(tokenBalanceUsdc.tokenData.mint.toBase58(), tokenBalanceUsdc);
        }
        const assetKey = decryptedUtxo.assets[assetIndex].toBase58();
        const utxoType = queuedLeavesPdaExists
            ? "committedUtxos"
            : nullifierExists
                ? "spentUtxos"
                : "utxos";
        (_a = balance.tokenBalances
            .get(assetKey)) === null || _a === void 0 ? void 0 : _a.addUtxo(decryptedUtxo.getCommitment(poseidon), decryptedUtxo, utxoType);
    }
}
exports.decryptAddUtxoToBalance = decryptAddUtxoToBalance;
function createUtxoBatches(indexedTransactions) {
    let utxoBatches = [];
    for (const trx of indexedTransactions) {
        let leftLeafIndex = new anchor_1.BN(trx.firstLeafIndex).toNumber();
        for (let index = 0; index < trx.leaves.length; index += 2) {
            const leafLeft = trx.leaves[index];
            const leafRight = trx.leaves[index + 1];
            let batch = {
                leftLeafIndex: leftLeafIndex,
                encryptedUtxos: [
                    {
                        index: leftLeafIndex,
                        commitment: Buffer.from([...leafLeft]),
                        leftLeaf: Uint8Array.from([...leafLeft]),
                        encBytes: trx.encryptedUtxos.slice((index / 2) * 240, (index / 2) * 240 + index_1.NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH),
                    },
                    {
                        index: leftLeafIndex + 1,
                        commitment: Buffer.from([...leafRight]),
                        leftLeaf: Uint8Array.from([...leafLeft]),
                        encBytes: trx.encryptedUtxos.slice((index / 2) * 240 + 120, (index / 2) * 240 +
                            index_1.NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH +
                            120),
                    },
                ],
            };
            utxoBatches.push(batch);
            // transaction nonce is the same for all utxos in one transaction
            // await decryptAddUtxoToBalance({
            //   encBytes: Buffer.from(
            //     trx.encryptedUtxos.slice(
            //       (index / 2) * 240,
            //       (index / 2) * 240 + NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH,
            //     ),
            //   ),
            //   index: leftLeafIndex,
            //   commitment: Buffer.from([...leafLeft]),
            //   account: this.account,
            //   poseidon: this.provider.poseidon,
            //   connection: this.provider.provider.connection,
            //   balance,
            //   merkleTreePdaPublicKey,
            //   leftLeaf: Uint8Array.from([...leafLeft]),
            //   aes,
            //   verifierProgramLookupTable:
            //     this.provider.lookUpTables.verifierProgramLookupTable,
            //   assetLookupTable: this.provider.lookUpTables.assetLookupTable,
            // });
            // await decryptAddUtxoToBalance({
            //   encBytes: Buffer.from(
            //     trx.encryptedUtxos.slice(
            //       (index / 2) * 240 + 120,
            //       (index / 2) * 240 +
            //         NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH +
            //         120,
            //     ),
            //   ),
            //   index: leftLeafIndex + 1,
            //   commitment: Buffer.from([...leafRight]),
            //   account: this.account,
            //   poseidon: this.provider.poseidon,
            //   connection: this.provider.provider.connection,
            //   balance,
            //   merkleTreePdaPublicKey,
            //   leftLeaf: Uint8Array.from([...leafLeft]),
            //   aes,
            //   verifierProgramLookupTable:
            //     this.provider.lookUpTables.verifierProgramLookupTable,
            //   assetLookupTable: this.provider.lookUpTables.assetLookupTable,
            // });
        }
    }
    return utxoBatches;
}
exports.createUtxoBatches = createUtxoBatches;
async function buildUtxoBalanceFromUtxoBytes({ utxoBytes, poseidon, account, appDataIdl, assetLookupTable, verifierProgramLookupTable, connection, balance, }) {
    for (const bytes of utxoBytes) {
        let decryptedUtxo = utxo_1.Utxo.fromBytes({
            poseidon,
            bytes: bytes.bytes,
            account,
            index: bytes.index,
            appDataIdl,
            assetLookupTable,
            verifierProgramLookupTable,
        });
        addUtxoToBalance({
            decryptedUtxo,
            poseidon,
            connection,
            balance,
            leftLeaf: bytes.leftLeaf,
        });
    }
}
exports.buildUtxoBalanceFromUtxoBytes = buildUtxoBalanceFromUtxoBytes;
async function addUtxoToBalance({ decryptedUtxo, poseidon, connection, balance, leftLeaf, }) {
    var _a;
    // null if utxo did not decrypt -> return nothing and continue
    if (!decryptedUtxo)
        return;
    const nullifier = decryptedUtxo.getNullifier(poseidon);
    if (!nullifier)
        return;
    const nullifierExists = await (0, utils_1.fetchNullifierAccountInfo)(nullifier, connection);
    const queuedLeavesPdaExists = await (0, utils_1.fetchQueuedLeavesAccountInfo)(leftLeaf, connection);
    const amountsValid = decryptedUtxo.amounts[1].toString() !== "0" ||
        decryptedUtxo.amounts[0].toString() !== "0";
    const assetIndex = decryptedUtxo.amounts[1].toString() !== "0" ? 1 : 0;
    // valid amounts and is not app utxo
    if (amountsValid &&
        decryptedUtxo.verifierAddress.toBase58() === new web3_js_1.PublicKey(0).toBase58() &&
        decryptedUtxo.appDataHash.toString() === "0") {
        // TODO: add is native to utxo
        // if !asset try to add asset and then push
        if (assetIndex &&
            !balance.tokenBalances.get(decryptedUtxo.assets[assetIndex].toBase58())) {
            // TODO: several maps or unify somehow
            let tokenBalanceUsdc = new TokenUtxoBalance(index_1.TOKEN_REGISTRY.get("USDC"));
            balance.tokenBalances.set(tokenBalanceUsdc.tokenData.mint.toBase58(), tokenBalanceUsdc);
        }
        const assetKey = decryptedUtxo.assets[assetIndex].toBase58();
        const utxoType = queuedLeavesPdaExists
            ? "committedUtxos"
            : nullifierExists
                ? "spentUtxos"
                : "utxos";
        (_a = balance.tokenBalances
            .get(assetKey)) === null || _a === void 0 ? void 0 : _a.addUtxo(decryptedUtxo.getCommitment(poseidon), decryptedUtxo, utxoType);
    }
}
exports.addUtxoToBalance = addUtxoToBalance;
//# sourceMappingURL=buildBalance.js.map