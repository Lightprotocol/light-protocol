import { Utxo } from "../utxo";
import * as anchor from "@coral-xyz/anchor";
import { BN } from "@coral-xyz/anchor";

import { Connection, PublicKey, SystemProgram } from "@solana/web3.js";
import { Account } from "../account";
import {
  fetchNullifierAccountInfo,
  fetchQueuedLeavesAccountInfo,
} from "../utils";
import { QueuedLeavesPda } from "merkleTree";
import {
  ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH,
  NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH,
  TokenUtxoBalanceError,
  TokenUtxoBalanceErrorCode,
  TOKEN_REGISTRY,
  TokenData,
} from "../index";

// mint | programAdress for programUtxos
export type Balance = {
  tokenBalances: Map<string, TokenUtxoBalance>;
  programBalances?: Map<string, ProgramUtxoBalance>;
  nftBalances?: Map<string, TokenUtxoBalance>;
  transactionNonce: number;
  decryptionTransactionNonce: number;
  committedTransactionNonce: number;
  totalSolBalance: BN;
};

export type ProgramUtxoBalance = TokenUtxoBalance & {
  programAddress: PublicKey;
  programUtxoIdl: anchor.Idl;
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
  utxos: Map<string, Utxo>; // commitmenthash as key
  committedUtxos: Map<string, Utxo>; // utxos which are
  spentUtxos: Map<string, Utxo>; // ordered for slot spent - maybe this should just be an UserIndexedTransaction
  constructor(tokenData: TokenData) {
    this.tokenData = tokenData;
    this.totalBalanceSol = new BN(0);
    this.totalBalanceSpl = new BN(0);
    this.utxos = new Map();
    this.committedUtxos = new Map();
    this.spentUtxos = new Map();
  }

  static initSol(): TokenUtxoBalance {
    return new TokenUtxoBalance(TOKEN_REGISTRY.get("SOL")!);
  }

  addUtxo(commitment: string, utxo: Utxo, attribute: VariableType): boolean {
    let utxoExists =
      this[attribute].get(commitment) !== undefined ? true : false;
    this[attribute].set(commitment, utxo);

    if (attribute === ("utxos" as VariableType) && !utxoExists) {
      this.totalBalanceSol = this.totalBalanceSol.add(utxo.amounts[0]);
      if (utxo.amounts[1])
        this.totalBalanceSpl = this.totalBalanceSpl.add(utxo.amounts[1]);
    }
    return !utxoExists;
  }

  // movetToCommittedUtxos(commitment: string) {
  //   let utxo = this.utxos.get(commitment);
  //   if (!utxo)
  //     throw new TokenUtxoBalanceError(
  //       TokenUtxoBalanceErrorCode.UTXO_UNDEFINED,
  //       "moveToCommittedUtxos",
  //       `utxo with committment ${commitment} does not exist in utxos`,
  //     );
  //   this.totalBalanceSol = this.totalBalanceSol.sub(utxo.amounts[0]);
  //   if (utxo.amounts[1])
  //     this.totalBalanceSpl = this.totalBalanceSpl.sub(utxo.amounts[1]);
  //   this.committedUtxos.set(commitment, utxo);
  //   this.utxos.delete(commitment);
  // }

  movetToSpentUtxos(commitment: string) {
    let utxo = this.utxos.get(commitment);
    if (!utxo)
      throw new TokenUtxoBalanceError(
        TokenUtxoBalanceErrorCode.UTXO_UNDEFINED,
        "movetToSpentUtxos",
        `utxo with committment ${commitment} does not exist in committed utxos`,
      );
    this.totalBalanceSol = this.totalBalanceSol.sub(utxo.amounts[0]);
    if (utxo.amounts[1])
      this.totalBalanceSpl = this.totalBalanceSpl.sub(utxo.amounts[1]);
    this.spentUtxos.set(commitment, utxo);
    this.utxos.delete(commitment);
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
  decryptionTransactionNonce,
}: {
  encBytes: Uint8Array;
  index: number;
  commitment: Uint8Array;
  account: Account;
  merkleTreePdaPublicKey: PublicKey;
  poseidon: any;
  connection: Connection;
  balance: Balance;
  leftLeaf: Uint8Array;
  aes: boolean;
  decryptionTransactionNonce: number;
}): Promise<number> {
  let decryptedUtxo = await Utxo.decrypt({
    poseidon,
    encBytes: encBytes,
    account: account,
    index: index,
    commitment,
    aes,
    merkleTreePdaPublicKey,
    transactionNonce: decryptionTransactionNonce,
  });

  // null if utxo did not decrypt -> return nothing and continue
  if (!decryptedUtxo) return decryptionTransactionNonce;
  // found utxo and increment transactionNonce

  const nullifier = decryptedUtxo.getNullifier(poseidon);
  if (!nullifier) return decryptionTransactionNonce;
  decryptionTransactionNonce += 1;

  const nullifierExists = await fetchNullifierAccountInfo(
    nullifier,
    connection,
  );
  const queuedLeavesPdaExists = await fetchQueuedLeavesAccountInfo(
    leftLeaf,
    connection,
  );

  const amountsValid =
    decryptedUtxo.amounts[1].toString() !== "0" ||
    decryptedUtxo.amounts[0].toString() !== "0";
  const assetIndex = decryptedUtxo.amounts[1].toString() !== "0" ? 1 : 0;

  if (amountsValid) {
    // TODO: add is native to utxo
    // if !asset try to add asset and then push
    if (
      assetIndex &&
      !balance.tokenBalances.get(decryptedUtxo.assets[assetIndex].toBase58())
    ) {
      // TODO: several maps or unify somehow
      let tokenBalanceUsdc = new TokenUtxoBalance(TOKEN_REGISTRY.get("USDC")!);
      balance.tokenBalances.set(
        tokenBalanceUsdc.tokenData.mint.toBase58(),
        tokenBalanceUsdc,
      );
    }
    const assetKey =
      decryptedUtxo.assets[queuedLeavesPdaExists ? 1 : assetIndex].toBase58();
    const utxoType = queuedLeavesPdaExists
      ? "committedUtxos"
      : nullifierExists
      ? "spentUtxos"
      : "utxos";

    let addedUtxo = balance.tokenBalances
      .get(assetKey)
      ?.addUtxo(decryptedUtxo.getCommitment(poseidon), decryptedUtxo, utxoType);
  }
  return decryptionTransactionNonce;
}
