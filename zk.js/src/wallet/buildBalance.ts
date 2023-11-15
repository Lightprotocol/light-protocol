import { Utxo } from "../utxo";
import * as anchor from "@coral-xyz/anchor";
import { BN } from "@coral-xyz/anchor";

import { Connection, PublicKey } from "@solana/web3.js";
import { Account } from "../account";
import {
  fetchNullifierAccountInfo,
  fetchQueuedLeavesAccountInfo,
} from "../utils";
import {
  TokenUtxoBalanceError,
  TokenUtxoBalanceErrorCode,
  TOKEN_REGISTRY,
  TokenData,
  ProgramUtxoBalanceError,
  ProgramUtxoBalanceErrorCode,
  TOKEN_PUBKEY_SYMBOL,
  UserErrorCode,
  BN_0,
  Poseidon,
} from "../index";

// mint | programAddress for programUtxos
export type Balance = {
  tokenBalances: Map<string, TokenUtxoBalance>;
  programBalances: Map<string, ProgramUtxoBalance>;
  nftBalances: Map<string, TokenUtxoBalance>;
  totalSolBalance: BN;
};

export type InboxBalance = Balance & {
  numberInboxUtxos: number;
};

type VariableType = "utxos" | "committedUtxos" | "spentUtxos";

// TODO: add nfts
export class TokenUtxoBalance {
  tokenData: TokenData;
  totalBalanceSpl: BN;
  totalBalanceSol: BN;
  utxos: Map<string, Utxo>; // commitment hash as key
  committedUtxos: Map<string, Utxo>; // utxos which are
  spentUtxos: Map<string, Utxo>; // ordered for slot spent - maybe this should just be an UserIndexedTransaction
  constructor(tokenData: TokenData) {
    this.tokenData = tokenData;
    this.totalBalanceSol = BN_0;
    this.totalBalanceSpl = BN_0;
    this.utxos = new Map();
    this.committedUtxos = new Map();
    this.spentUtxos = new Map();
  }

  static initSol(): TokenUtxoBalance {
    return new TokenUtxoBalance(TOKEN_REGISTRY.get("SOL")!);
  }

  addUtxo(commitment: string, utxo: Utxo, attribute: VariableType): boolean {
    const utxoExists = this[attribute].get(commitment) !== undefined;
    this[attribute].set(commitment, utxo);

    if (attribute === ("utxos" as VariableType) && !utxoExists) {
      this.committedUtxos.delete(commitment);
      this.totalBalanceSol = this.totalBalanceSol.add(utxo.amounts[0]);
      if (utxo.amounts[1])
        this.totalBalanceSpl = this.totalBalanceSpl.add(utxo.amounts[1]);
    }
    return !utxoExists;
  }

  moveToSpentUtxos(commitment: string) {
    const utxo = this.utxos.get(commitment);
    if (!utxo)
      throw new TokenUtxoBalanceError(
        TokenUtxoBalanceErrorCode.UTXO_UNDEFINED,
        "moveToSpentUtxos",
        `utxo with commitment ${commitment} does not exist in utxos`,
      );
    this.totalBalanceSol = this.totalBalanceSol.sub(utxo.amounts[0]);
    if (utxo.amounts[1])
      this.totalBalanceSpl = this.totalBalanceSpl.sub(utxo.amounts[1]);
    this.spentUtxos.set(commitment, utxo);
    this.utxos.delete(commitment);
  }
}

export class ProgramUtxoBalance {
  programAddress: PublicKey;
  programUtxoIdl: anchor.Idl;
  tokenBalances: Map<string, TokenUtxoBalance>;

  constructor(programAddress: PublicKey, programUtxoIdl: anchor.Idl) {
    this.programAddress = programAddress;
    this.programUtxoIdl = programUtxoIdl;
    this.tokenBalances = new Map();
  }

  addUtxo(commitment: string, utxo: Utxo, attribute: VariableType): boolean {
    if (!utxo.verifierAddress) {
      throw new ProgramUtxoBalanceError(
        ProgramUtxoBalanceErrorCode.INVALID_PROGRAM_ADDRESS,
        "addUtxo",
        `Verifier address in utxo ${utxo._commitment} does not exist in utxo (trying to add utxo to program utxos balance)`,
      );
    }
    if (!utxo.verifierAddress.equals(this.programAddress)) {
      throw new ProgramUtxoBalanceError(
        ProgramUtxoBalanceErrorCode.INVALID_PROGRAM_ADDRESS,
        "addUtxo",
        `Verifier address ${utxo.verifierAddress} does not match the program address (trying to add utxo to program utxos balance)`,
      );
    }
    const utxoAsset =
      utxo.amounts[1].toString() === "0"
        ? new PublicKey(0).toBase58()
        : utxo.assets[1].toBase58();
    const tokenBalance = this.tokenBalances?.get(utxoAsset);
    // if not token balance for utxoAsset create token balance
    if (!tokenBalance) {
      const tokenSymbol = TOKEN_PUBKEY_SYMBOL.get(utxoAsset);
      if (!tokenSymbol)
        throw new ProgramUtxoBalanceError(
          UserErrorCode.TOKEN_NOT_FOUND,
          "addUtxo",
          `Token ${utxoAsset} not found when trying to add tokenBalance to ProgramUtxoBalance for verifier ${this.programAddress.toBase58()}`,
        );
      const tokenData = TOKEN_REGISTRY.get(tokenSymbol);
      if (!tokenData)
        throw new ProgramUtxoBalanceError(
          ProgramUtxoBalanceErrorCode.TOKEN_DATA_NOT_FOUND,
          "addUtxo",
          `Token ${utxoAsset} not found when trying to add tokenBalance to ProgramUtxoBalance for verifier ${this.programAddress.toBase58()}`,
        );
      this.tokenBalances.set(utxoAsset, new TokenUtxoBalance(tokenData));
    }
    return this.tokenBalances
      .get(utxoAsset)!
      .addUtxo(commitment, utxo, attribute);
  }
}

export class ProgramBalance extends TokenUtxoBalance {
  programAddress: PublicKey;
  programUtxoIdl: anchor.Idl;

  constructor(
    tokenData: TokenData,
    programAddress: PublicKey,
    programUtxoIdl: anchor.Idl,
  ) {
    super(tokenData);
    this.programAddress = programAddress;
    this.programUtxoIdl = programUtxoIdl;
  }

  addProgramUtxo(
    commitment: string,
    utxo: Utxo,
    attribute: VariableType,
  ): boolean {
    const utxoExists = this[attribute].get(commitment) !== undefined;
    this[attribute].set(commitment, utxo);

    if (attribute === ("utxos" as VariableType) && !utxoExists) {
      this.totalBalanceSol = this.totalBalanceSol.add(utxo.amounts[0]);
      if (utxo.amounts[1]) {
        this.totalBalanceSpl = this.totalBalanceSpl.add(utxo.amounts[1]);
      }
    }
    return !utxoExists;
  }
}

export async function decryptAddUtxoToBalance({
  account,
  encBytes,
  index,
  commitment,
  poseidon,
  connection,
  balance,
  merkleTreePdaPublicKey,
  leftLeaf,
  aes,
  assetLookupTable,
}: {
  encBytes: Uint8Array;
  index: number;
  commitment: Uint8Array;
  account: Account;
  merkleTreePdaPublicKey: PublicKey;
  poseidon: Poseidon;
  connection: Connection;
  balance: Balance;
  leftLeaf: Uint8Array;
  aes: boolean;
  assetLookupTable: string[];
}): Promise<void> {
  const decryptedUtxo = aes
    ? await Utxo.decrypt({
        poseidon,
        encBytes: encBytes,
        account: account,
        index: index,
        commitment,
        aes,
        merkleTreePdaPublicKey,
        assetLookupTable,
      })
    : await Utxo.decryptUnchecked({
        poseidon,
        encBytes: encBytes,
        account: account,
        index: index,
        commitment,
        aes,
        merkleTreePdaPublicKey,
        assetLookupTable,
      });

  // null if utxo did not decrypt -> return nothing and continue
  if (!decryptedUtxo.value || decryptedUtxo.error) return;
  const utxo = decryptedUtxo.value;
  const nullifier = utxo.getNullifier({ poseidon, account });
  if (!nullifier) return;

  const nullifierExists = await fetchNullifierAccountInfo(
    nullifier,
    connection,
  );
  const queuedLeavesPdaExists = await fetchQueuedLeavesAccountInfo(
    leftLeaf,
    connection,
  );

  const amountsValid =
    utxo.amounts[1].toString() !== "0" || utxo.amounts[0].toString() !== "0";
  const assetIndex = utxo.amounts[1].toString() !== "0" ? 1 : 0;

  // valid amounts and is not app utxo
  if (
    amountsValid &&
    utxo.verifierAddress.toBase58() === new PublicKey(0).toBase58() &&
    utxo.appDataHash.toString() === "0"
  ) {
    // TODO: add is native to utxo
    // if !asset try to add asset and then push
    if (
      assetIndex &&
      !balance.tokenBalances.get(utxo.assets[assetIndex].toBase58())
    ) {
      // TODO: several maps or unify somehow
      const tokenBalanceUsdc = new TokenUtxoBalance(
        TOKEN_REGISTRY.get("USDC")!,
      );
      balance.tokenBalances.set(
        tokenBalanceUsdc.tokenData.mint.toBase58(),
        tokenBalanceUsdc,
      );
    }
    const assetKey = utxo.assets[assetIndex].toBase58();
    const utxoType = queuedLeavesPdaExists
      ? "committedUtxos"
      : nullifierExists
      ? "spentUtxos"
      : "utxos";

    balance.tokenBalances
      .get(assetKey)
      ?.addUtxo(utxo.getCommitment(poseidon), utxo, utxoType);
  }
}
