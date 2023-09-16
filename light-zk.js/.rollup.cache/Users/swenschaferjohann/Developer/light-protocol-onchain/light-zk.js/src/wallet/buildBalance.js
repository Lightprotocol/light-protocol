import { Utxo } from "../utxo";
import { PublicKey } from "@solana/web3.js";
import { fetchNullifierAccountInfo, fetchQueuedLeavesAccountInfo, } from "../utils";
import { TokenUtxoBalanceError, TokenUtxoBalanceErrorCode, TOKEN_REGISTRY, ProgramUtxoBalanceError, ProgramUtxoBalanceErrorCode, TOKEN_PUBKEY_SYMBOL, UserErrorCode, BN_0, } from "../index";
// TODO: add nfts
export class TokenUtxoBalance {
    constructor(tokenData) {
        this.tokenData = tokenData;
        this.totalBalanceSol = BN_0;
        this.totalBalanceSpl = BN_0;
        this.utxos = new Map();
        this.committedUtxos = new Map();
        this.spentUtxos = new Map();
    }
    static initSol() {
        return new TokenUtxoBalance(TOKEN_REGISTRY.get("SOL"));
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
            throw new TokenUtxoBalanceError(TokenUtxoBalanceErrorCode.UTXO_UNDEFINED, "moveToSpentUtxos", `utxo with committment ${commitment} does not exist in utxos`);
        this.totalBalanceSol = this.totalBalanceSol.sub(utxo.amounts[0]);
        if (utxo.amounts[1])
            this.totalBalanceSpl = this.totalBalanceSpl.sub(utxo.amounts[1]);
        this.spentUtxos.set(commitment, utxo);
        this.utxos.delete(commitment);
    }
}
export class ProgramUtxoBalance {
    constructor(programAddress, programUtxoIdl) {
        this.programAddress = programAddress;
        this.programUtxoIdl = programUtxoIdl;
        this.tokenBalances = new Map();
    }
    addUtxo(commitment, utxo, attribute) {
        var _a;
        if (!utxo.verifierAddress) {
            throw new ProgramUtxoBalanceError(ProgramUtxoBalanceErrorCode.INVALID_PROGRAM_ADDRESS, "addUtxo", `Verifier address in utxo ${utxo._commitment} does not exist in utxo (trying to add utxo to program utxos balance)`);
        }
        if (!utxo.verifierAddress.equals(this.programAddress)) {
            throw new ProgramUtxoBalanceError(ProgramUtxoBalanceErrorCode.INVALID_PROGRAM_ADDRESS, "addUtxo", `Verifier address ${utxo.verifierAddress} does not match the program address (trying to add utxo to program utxos balance)`);
        }
        let utxoAsset = utxo.amounts[1].toString() === "0"
            ? new PublicKey(0).toBase58()
            : utxo.assets[1].toBase58();
        let tokenBalance = (_a = this.tokenBalances) === null || _a === void 0 ? void 0 : _a.get(utxoAsset);
        // if not token balance for utxoAsset create token balance
        if (!tokenBalance) {
            const tokenSymbol = TOKEN_PUBKEY_SYMBOL.get(utxoAsset);
            if (!tokenSymbol)
                throw new ProgramUtxoBalanceError(UserErrorCode.TOKEN_NOT_FOUND, "addUtxo", `Token ${utxoAsset} not found when trying to add tokenBalance to PrograUtxoBalance for verifier ${this.programAddress.toBase58()}`);
            const tokenData = TOKEN_REGISTRY.get(tokenSymbol);
            if (!tokenData)
                throw new ProgramUtxoBalanceError(ProgramUtxoBalanceErrorCode.TOKEN_DATA_NOT_FOUND, "addUtxo", `Token ${utxoAsset} not found when trying to add tokenBalance to PrograUtxoBalance for verifier ${this.programAddress.toBase58()}`);
            this.tokenBalances.set(utxoAsset, new TokenUtxoBalance(tokenData));
        }
        return this.tokenBalances
            .get(utxoAsset)
            .addUtxo(commitment, utxo, attribute);
    }
}
export class ProgramBalance extends TokenUtxoBalance {
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
export async function decryptAddUtxoToBalance({ account, encBytes, index, commitment, poseidon, connection, balance, merkleTreePdaPublicKey, leftLeaf, aes, verifierProgramLookupTable, assetLookupTable, }) {
    var _a;
    let decryptedUtxo = await Utxo.decrypt({
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
    const nullifierExists = await fetchNullifierAccountInfo(nullifier, connection);
    const queuedLeavesPdaExists = await fetchQueuedLeavesAccountInfo(leftLeaf, connection);
    const amountsValid = decryptedUtxo.amounts[1].toString() !== "0" ||
        decryptedUtxo.amounts[0].toString() !== "0";
    const assetIndex = decryptedUtxo.amounts[1].toString() !== "0" ? 1 : 0;
    // valid amounts and is not app utxo
    if (amountsValid &&
        decryptedUtxo.verifierAddress.toBase58() === new PublicKey(0).toBase58() &&
        decryptedUtxo.appDataHash.toString() === "0") {
        // TODO: add is native to utxo
        // if !asset try to add asset and then push
        if (assetIndex &&
            !balance.tokenBalances.get(decryptedUtxo.assets[assetIndex].toBase58())) {
            // TODO: several maps or unify somehow
            let tokenBalanceUsdc = new TokenUtxoBalance(TOKEN_REGISTRY.get("USDC"));
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
//# sourceMappingURL=buildBalance.js.map