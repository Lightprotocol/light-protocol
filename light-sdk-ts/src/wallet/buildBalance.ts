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
  committedTransactionNonce: number;
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
  addUtxo(commitment: string, utxo: Utxo, attribute: VariableType) {
    this[attribute].set(commitment, utxo);

    if (attribute === ("utxos" as VariableType)) {
      this.totalBalanceSol = this.totalBalanceSol.add(utxo.amounts[0]);
      if (utxo.amounts[1])
        this.totalBalanceSpl = this.totalBalanceSpl.add(utxo.amounts[1]);
    }
  }

  movetToCommittedUtxos(commitment: string) {
    let utxo = this.utxos.get(commitment);
    if (!utxo)
      throw new TokenUtxoBalanceError(
        TokenUtxoBalanceErrorCode.UTXO_UNDEFINED,
        "moveToCommittedUtxos",
        `utxo with committment ${commitment} does not exist in utxos`,
      );
    this.totalBalanceSol = this.totalBalanceSol.sub(utxo.amounts[0]);
    if (utxo.amounts[1])
      this.totalBalanceSpl = this.totalBalanceSpl.sub(utxo.amounts[1]);
    this.committedUtxos.set(commitment, utxo);
    this.utxos.delete(commitment);
  }

  movetToSpentUtxos(commitment: string) {
    let utxo = this.committedUtxos.get(commitment);
    if (!utxo)
      throw new TokenUtxoBalanceError(
        TokenUtxoBalanceErrorCode.UTXO_UNDEFINED,
        "movetToSpentUtxos",
        `utxo with committment ${commitment} does not exist in committed utxos`,
      );
    this.spentUtxos.set(commitment, utxo);
    this.committedUtxos.delete(commitment);
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
}): Promise<void> {
  let decryptedUtxo = await Utxo.decrypt({
    poseidon,
    encBytes: encBytes,
    account: account,
    index: index,
    commitment,
    aes,
    merkleTreePdaPublicKey,
    transactionNonce: balance.transactionNonce,
  });

  // null if utxo did not decrypt -> return nothing and continue
  if (!decryptedUtxo) return;
  // found utxo and increment transactionNonce
  balance.transactionNonce += 1;
  const nullifier = decryptedUtxo.getNullifier(poseidon);
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
    decryptedUtxo.amounts[1].toString() !== "0" ||
    decryptedUtxo.amounts[0].toString() !== "0";
  const assetIndex = decryptedUtxo.amounts[0].toString() !== "0" ? 1 : 0;

  if (amountsValid) {
    // TODO: add is native to utxo
    // if !asset try to add asset and then push
    if (
      assetIndex &&
      !balance.tokenBalances.get(decryptedUtxo.assets[assetIndex].toBase58())
    ) {
      // TODO: several maps or unify somehow
      let tokenBalanceUsdc = new TokenUtxoBalance(TOKEN_REGISTRY.get("USDC"));
      balance.tokenBalances.set(
        tokenBalanceUsdc.tokenData.mint.toBase58(),
        tokenBalanceUsdc,
      );
    }

    if (queuedLeavesPdaExists) {
      balance.tokenBalances
        .get(decryptedUtxo.assets[1].toBase58())
        ?.addUtxo(
          decryptedUtxo.getCommitment(poseidon),
          decryptedUtxo,
          "committedUtxos",
        );
    } else if (!nullifierExists) {
      balance.tokenBalances
        .get(decryptedUtxo.assets[assetIndex].toBase58())
        ?.addUtxo(
          decryptedUtxo.getCommitment(poseidon),
          decryptedUtxo,
          "utxos",
        );
    } else if (nullifierExists) {
      balance.tokenBalances
        .get(decryptedUtxo.assets[assetIndex].toBase58())
        ?.addUtxo(
          decryptedUtxo.getCommitment(poseidon),
          decryptedUtxo,
          "spentUtxos",
        );
    }
  }
}

// TODO: rename to decryptUtxoPair or decryptLeavesPairUtxos
/**
 *  Fetches the decrypted and spent UTXOs for an account from the provided leavesPDAs.
 * @param {Array} leavesPdas - An array of leaf PDAs containing the UTXO data.
 * @param {anchor.Provider} provider - The Anchor provider to interact with the Solana network.
 * @param {Account} account - The user account for which to fetch the UTXOs.
 * @param {Object} poseidon - The Poseidon object used for cryptographic operations.
 * @returns {Promise<{ decryptedUtxos: Utxo[], spentUtxos: Utxo[] }>} A Promise that resolves to an object containing two arrays:
 * - decryptedUtxos: The decrypted UTXOs that have not been spent.
 * - spentUtxos: The decrypted UTXOs that have been spent.
 */
// export async function getAccountUtxos({
//   leavesPdas,
//   provider,
//   account,
//   poseidon,
//   aes,
//   merkleTreePdaPublicKey,
//   balance,
// }: {
//   leavesPdas: any;
//   provider: anchor.Provider;
//   account: Account;
//   merkleTreePdaPublicKey: PublicKey;
//   poseidon: any;
//   aes: boolean;
//   balance: Balance;
// }): Promise<Balance> {

//   for (var leafPda of leavesPdas) {
//     const decrypted = [
//       await Utxo.decrypt({
//         poseidon: poseidon,
//         encBytes: new Uint8Array(
//           Array.from(
//             leafPda.account.encryptedUtxos.slice(
//               0,
//               NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH,
//             ),
//           ),
//         ),
//         account,
//         index: leafPda.account.leftLeafIndex.toNumber(),
//         commitment: Uint8Array.from([...leafPda.account.nodeLeft]),
//         aes,
//         merkleTreePdaPublicKey,
//         transactionNonce: balance.transactionNonce,
//       }),
//       await Utxo.decrypt({
//         poseidon: poseidon,
//         encBytes: new Uint8Array(
//           Array.from(
//             leafPda.account.encryptedUtxos.slice(
//               128,
//               128 + NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH,
//             ),
//           ),
//         ),
//         account,
//         index: leafPda.account.leftLeafIndex.toNumber() + 1,
//         commitment: Uint8Array.from([...leafPda.account.nodeRight]),
//         aes,
//         merkleTreePdaPublicKey,
//         transactionNonce: balance.transactionNonce,
//       }),
//     ];
//     if (decrypted[0]) {
//       // checks that
//       // - is not spent
//       // - amounts > 0
//       // - get spl pubkey
//       // -
//       await addUtxoToBalance({
//         decryptedUtxo: decrypted[0],
//         poseidon,
//         connection: provider.connection,
//         balance
//       });
//       balance.transactionNonce += 1;

//     }
//     if (decrypted[1]) {
//       await addUtxoToBalance({
//         decryptedUtxo: decrypted[1],
//         poseidon,
//         connection: provider.connection,
//         balance
//       });
//       balance.transactionNonce += 1;
//     }
//   }
//   return balance;
// }
