import { Utxo } from "../utxo";
import { Connection, PublicKey } from "@solana/web3.js";
import {
  BN_0,
  NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH,
  TOKEN_REGISTRY,
  UTXO_ASSET_SOL_INDEX,
  UTXO_ASSET_SPL_INDEX,
  UTXO_PREFIX_LENGTH,
} from "../constants";
import { Provider } from "../wallet";
import { Poseidon } from "../types/poseidon";
import {
  Balance,
  TokenBalance,
  TokenData,
  SerializedTokenBalance,
  SerializedBalance,
} from "../types/balance";
import {
  ProgramUtxoBalanceError,
  ProgramUtxoBalanceErrorCode,
} from "../errors";
import {
  fetchNullifierAccountInfo,
  fetchQueuedLeavesAccountInfo,
} from "../utils";
import {
  Account,
  MerkleTreeConfig,
  ParsedIndexedTransaction,
  Relayer,
} from "../index";
import { BN } from "@coral-xyz/anchor";
import { Hasher } from "@lightprotocol/account.rs";

export const isSPLUtxo = (utxo: Utxo): boolean => {
  return !utxo.amounts[UTXO_ASSET_SPL_INDEX].eqn(0);
};

/**
 * Sorts biggest to smallest by amount of the mint.
 * Worst-case: O(n log n) complexity, which is fine for small n.
 * for 1000 utxos = 10k operations = roughly .01ms
 * If we eventually need to optimize, we can use a heap to get O(log n).
 * @param utxos
 * @returns sorted utxos
 */
export function sortUtxos(utxos: Utxo[]): Utxo[] {
  const mint = utxos[0].assets[UTXO_ASSET_SPL_INDEX];
  for (const utxo of utxos) {
    if (!utxo.assets[UTXO_ASSET_SPL_INDEX].equals(mint)) {
      throw new ProgramUtxoBalanceError(
        ProgramUtxoBalanceErrorCode.INVALID_UTXO_MINT,
        "sortUtxos",
        `Utxo mints don't match each other. Expecting: ${mint.toBase58()}, found ${utxo.assets[
          UTXO_ASSET_SPL_INDEX
        ].toBase58()}]}`,
      );
    }
  }
  return utxos.sort((a, b) => {
    const aAmount = a.amounts[UTXO_ASSET_SPL_INDEX] ?? BN_0;
    const bAmount = b.amounts[UTXO_ASSET_SPL_INDEX] ?? BN_0;
    if (aAmount.isZero() && bAmount.isZero()) {
      return b.amounts[UTXO_ASSET_SOL_INDEX].cmp(
        a.amounts[UTXO_ASSET_SOL_INDEX],
      );
    }
    return bAmount.cmp(aAmount);
  });
}

/**
 *
 * @param mintToFind mint
 * @param tokenRegistry TOKEN_REGISTRY
 * @returns TokenData of the mint. Throws an error if mint not registered.
 */
export function getTokenDataByMint(
  mintToFind: PublicKey,
  tokenRegistry: Map<string, TokenData>,
): TokenData {
  for (const value of tokenRegistry.values()) {
    if (value.mint.equals(mintToFind)) {
      return value;
    }
  }
  throw new ProgramUtxoBalanceError(
    ProgramUtxoBalanceErrorCode.TOKEN_DATA_NOT_FOUND,
    "getTokenDataByMint",
    `Tokendata not found when trying to get tokenData for mint ${mintToFind.toBase58()}`,
  );
}

/**
 *
 * initializes TokenBalance for mint of TokenData and Utxos
 * Throws if Utxos do not match TokenData
 * If Utxos are not provided, initializes empty TokenBalance for the mint specified in TokenData
 * @param tokenData TokenData of the mint
 * @param utxos Utxos to initialize TokenBalance with
 * @returns TokenBalance
 *
 */
export function initTokenBalance(
  tokenData: TokenData,
  utxos?: Utxo[],
): TokenBalance {
  let splAmount = BN_0;
  let lamports = BN_0;

  if (utxos) {
    utxos.forEach((utxo) => {
      if (!utxo.assets[UTXO_ASSET_SPL_INDEX].equals(tokenData.mint)) {
        throw new ProgramUtxoBalanceError(
          ProgramUtxoBalanceErrorCode.INVALID_UTXO_MINT,
          "initTokenBalance",
          `UTXO mint does not match provided Tokendata ${tokenData.mint}`,
        );
      }
      splAmount = splAmount.add(utxo.amounts[UTXO_ASSET_SPL_INDEX] ?? BN_0);
      lamports = lamports.add(utxo.amounts[UTXO_ASSET_SOL_INDEX]);
    });
  }

  return {
    splAmount,
    lamports,
    tokenData,
    utxos: utxos ? sortUtxos(utxos) : [],
  } as TokenBalance;
}

/**
 * Updates TokenBalance with Utxo
 * Throws if Utxo does not match TokenBalance
 * Returns false if Utxo already part of TokenBalance.
 * @param utxo utxo to add to tokenBalance
 * @param tokenBalance tokenBalance to add utxo to
 * @param poseidon poseidon instance
 * @returns boolean indicating if the utxo was added to the tokenBalance
 */
export function updateTokenBalanceWithUtxo(
  utxo: Utxo,
  tokenBalance: TokenBalance,
  poseidon: Poseidon,
): boolean {
  // TODO: check if assigning commitments here is the right move.
  // note that getPoseidon will be loaded in memory once, so this is not a performance issue.
  // but if we need commitments for other purposes, we should consider moving it out.
  const utxoExists = tokenBalance.utxos.some(
    (existingUtxo) =>
      existingUtxo.getCommitment(poseidon) === utxo.getCommitment(poseidon),
  );
  if (utxoExists) return false;

  tokenBalance.utxos.push(utxo);
  tokenBalance.utxos = sortUtxos(tokenBalance.utxos);

  tokenBalance.lamports = tokenBalance.lamports.add(
    utxo.amounts[UTXO_ASSET_SOL_INDEX],
  );

  tokenBalance.splAmount = tokenBalance.splAmount.add(
    utxo.amounts[UTXO_ASSET_SPL_INDEX] ?? BN_0,
  );

  return true;
}

/**
 *
 * Given a balance and a utxo, adds the utxo to the balance.
 * skips if utxo already exists in balance.
 * initializes a new TokenBalance if the utxo is the first of its mint.
 * @param utxo utxo to add to balance
 * @param balance balance to add utxo to
 * @param poseidon poseidon instance
 * @returns
 */
export function addUtxoToBalance(
  utxo: Utxo,
  balance: Balance,
  poseidon: Poseidon,
): boolean {
  const ASSET_INDEX = isSPLUtxo(utxo)
    ? UTXO_ASSET_SPL_INDEX
    : UTXO_ASSET_SOL_INDEX;

  const assetKey = utxo.assets[ASSET_INDEX].toString();
  let tokenBalance = balance.tokenBalances.get(assetKey);

  if (!tokenBalance) {
    const tokenData = getTokenDataByMint(
      utxo.assets[ASSET_INDEX],
      TOKEN_REGISTRY,
    );

    tokenBalance = initTokenBalance(tokenData, [utxo]);
    balance.tokenBalances.set(assetKey, tokenBalance);
    return true;
  }

  return updateTokenBalanceWithUtxo(utxo, tokenBalance, poseidon);
}

/// TODO: after we implement history, extend this function to move the spentUtxo to history
/**
 * removes the specified utxo from balance
 * @param balance balance to remove utxo from
 * @param commitment commitment of the utxo to be removed
 * @returns boolean indicating if the utxo was removed from the balance
 */
export function spendUtxo(balance: Balance, commitment: string): boolean {
  for (const [_assetKey, tokenBalance] of balance.tokenBalances) {
    const utxoIndex = tokenBalance.utxos.findIndex(
      (utxo) => utxo._commitment === commitment,
    );
    if (utxoIndex !== -1) {
      const [spentUtxo] = tokenBalance.utxos.splice(utxoIndex, 1);
      tokenBalance.lamports = tokenBalance.lamports.sub(
        spentUtxo.amounts[UTXO_ASSET_SOL_INDEX],
      );
      tokenBalance.splAmount = tokenBalance.splAmount.sub(
        spentUtxo.amounts[UTXO_ASSET_SPL_INDEX] ?? BN_0,
      );
      return true;
    }
  }
  return false;
}

/**
 * serializes TokenBalance into a SerializedTokenBalance
 * @param tokenBalance
 * @returns serializedTokenBalance
 */
/// keeping track of index separately because it's not part of the UTXO IDL
async function serializeTokenBalance(
  tokenBalance: TokenBalance,
): Promise<SerializedTokenBalance> {
  const utxos = await Promise.all(
    tokenBalance.utxos.map(async (utxo) => ({
      utxo: await utxo.toString(),
      index: utxo.index,
    })),
  );

  const serializedTokenBalance: SerializedTokenBalance = {
    mint: tokenBalance.tokenData.mint.toString(),
    utxos: utxos,
  };

  return serializedTokenBalance;
}

export type AssetLookupTable = string[];
/**
 * deserializes SerializedTokenBalance into a TokenBalance
 */
function deserializeTokenBalance(
  serializedTokenBalance: SerializedTokenBalance,
  tokenRegistry: Map<string, TokenData>,
  assetLookupTable: AssetLookupTable,
  hasher: Hasher,
): TokenBalance {
  const tokenData = getTokenDataByMint(
    new PublicKey(serializedTokenBalance.mint),
    tokenRegistry,
  );

  const utxos = sortUtxos(
    serializedTokenBalance.utxos.map((serializedUtxo) => {
      const utxo = Utxo.fromString(
        serializedUtxo.utxo,
        hasher,
        assetLookupTable,
      );

      const index = serializedUtxo.index;
      utxo.index = index;
      return utxo;
    }),
  );

  return initTokenBalance(tokenData, utxos);
}

/**
 * serializes Balance into a stringified array of SerializedTokenBalances
 * @param balance balance
 * @returns serializedBalance
 */
export async function serializeBalance(balance: Balance): Promise<string> {
  const serializedBalance: SerializedBalance = {
    tokenBalances: [],
    lastSyncedSlot: balance.lastSyncedSlot,
  };

  for (const tokenBalance of balance.tokenBalances.values()) {
    serializedBalance.tokenBalances.push(
      await serializeTokenBalance(tokenBalance),
    );
  }

  return JSON.stringify(serializedBalance);
}

export function initBalance() {
  const balance: Balance = {
    tokenBalances: new Map<string, TokenBalance>(),
    lastSyncedSlot: 0,
  };
  return balance;
}

/**
 * deserializes stringified array of SerializedTokenBalances and reconstructs into a Balance
 * @param serializedBalance serializedBalance
 * @param tokenRegistry
 * @param provider lightProvider
 * @returns balance
 */
export function deserializeBalance(
  serializedBalance: string,
  tokenRegistry: Map<string, TokenData>,
  assetLookupTable: AssetLookupTable,
  hasher: Hasher,
): Balance {
  const cachedBalance: SerializedBalance = JSON.parse(serializedBalance);
  const balance = initBalance();
  balance.lastSyncedSlot = cachedBalance.lastSyncedSlot;

  for (const serializedTokenBalance of cachedBalance.tokenBalances) {
    const tokenBalance = deserializeTokenBalance(
      serializedTokenBalance,
      tokenRegistry,
      assetLookupTable,
      hasher,
    );
    balance.tokenBalances.set(serializedTokenBalance.mint, tokenBalance);
  }

  return balance;
}

/**
 * syncs balance with the blockchain. currently fetches all events. creates a new balance if none is provided.
 */
// until is either empty, accountcreationslot, or lastsyncedslot
// later: make it a prefix/signature
export async function syncBalance({
  connection,
  relayer,
  account,
  hasher,
  assetLookupTable,
  balance,
  _until, // keep it if we extract the event fetching into its own function,
}: {
  connection: Connection;
  relayer: Relayer;
  account: Account;
  hasher: Hasher;
  assetLookupTable: string[];
  balance?: Balance;
  _until?: number;
}) {
  if (!balance) balance = initBalance();
  // loops backwards in time starting at most recent event
  // TODO: until 'until' is reached
  const _balance = await findSpentUtxos(balance, connection, account, hasher);

  /// TODO: refactor this after the indexer refactor
  /// main goals: performant merkleproofs, and only fetch new events in syncbalance
  /// use balance.lastSyncedSlot for it as well
  const indexedTransactions = await relayer.getIndexedTransactions(connection);

  /// TODO: adapt to new index refactor
  // await provider.latestMerkleTree(indexedTransactions);

  // mutates _balance
  await tryDecryptNewUtxos({
    balance: _balance,
    indexedTransactions,
    connection,
    assetLookupTable,
    hasher,
    account,
    aes: true, // aes
    merkleTreePdaPublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
  });

  return _balance;
}

/// Ideally we'd want to reduce these types of calls as much as possible. This should only be needed if we actually append new utxos.
/// Are we not refetching all of em anyways? => if we use until sig/slot we can just fetch the new ones and then this would make sense
/// and also maybe only if we DO find new utxos (else the old ones wouldnt be spent)
/// the "updating balance" is specific to action to done so we should allow filtering by token/program as well.  to reduce latency.
// We should also have a client sidef subscription that runs in the background, to these such that the "lastsyncedslot" is as fresh as possible.
/// there should be fns that allows latency control and those that ensure safety for devs. (one that ensures full balance up2date).
/// will change once we refactor indexing by prefixes. so keep it easy for now.
export async function findSpentUtxos(
  balance: Balance,
  connection: Connection,
  account: Account,
  hasher: Hasher,
) {
  const tokenBalancesPromises = Array.from(balance.tokenBalances.values()).map(
    async (tokenBalance) => {
      const utxoPromises = tokenBalance.utxos.map(async (utxo) => {
        const nullifierAccountInfo = await fetchNullifierAccountInfo(
          utxo.getNullifier({
            hasher,
            account,
          })!,
          connection,
        );
        if (nullifierAccountInfo)
          spendUtxo(balance, utxo.getCommitment(hasher));
      });
      await Promise.all(utxoPromises);
    },
  );
  await Promise.all(tokenBalancesPromises);
  return balance;
}

/// TODO: adapt to work with wallet-adapter, batched decryption calls.
/**
 * for each event, decrypts the utxos belonging to the specified account and adds them to the balance
 * @param balance
 * @param indexedTransactions
 * @param provider
 * @param hasher
 * @param account
 * @param aes - whether to use aes (symmetric) decryption or not. default true for inbox
 * @param merkleTreePdaPublicKey
 */
export async function tryDecryptNewUtxos({
  balance,
  indexedTransactions,
  connection,
  hasher,
  account,
  aes,
  merkleTreePdaPublicKey,
  assetLookupTable, /// TODO: make optional, provide DEFAULT_ASSET_LOOKUP_TABLE
}: {
  balance: Balance;
  indexedTransactions: ParsedIndexedTransaction[];
  connection: Connection;
  hasher: Hasher;
  account: Account;
  aes: boolean;
  merkleTreePdaPublicKey: PublicKey;
  assetLookupTable: string[];
}): Promise<void> {
  /**
   * provider.solMerkleTree!.merkleTree.path(leftLeafIndex + 1)
   *       .pathElements
   */
  const merkleProofs = []; /// TODO: adapt to secure index refactor
  for (const trx of indexedTransactions) {
    const leftLeafIndex = new BN(trx.firstLeafIndex).toNumber();

    for (let index = 0; index < trx.leaves.length; index += 2) {
      /// @ts-ignore
      const leafLeft = trx.leaves[index];
      const leafRight = trx.leaves[index + 1];

      const encUtxoSize =
        NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH + UTXO_PREFIX_LENGTH;
      // transaction nonce is the same for all utxos in one transaction
      await decryptAddUtxoToBalance_new({
        encBytes: Buffer.from(
          trx.encryptedUtxos.slice(
            index * encUtxoSize,
            index * encUtxoSize + encUtxoSize,
          ),
        ),
        index: leftLeafIndex,
        commitment: Buffer.from([...leafLeft]),
        account,
        hasher,
        connection,
        balance,
        merkleTreePdaPublicKey,
        leftLeaf: Uint8Array.from([...leafLeft]),
        aes,
        assetLookupTable,
        merkleProof: merkleProofs[index],
      });
      await decryptAddUtxoToBalance_new({
        encBytes: Buffer.from(
          trx.encryptedUtxos.slice(
            index * encUtxoSize + encUtxoSize,
            index * encUtxoSize + encUtxoSize * 2,
          ),
        ),
        index: leftLeafIndex + 1,
        commitment: Buffer.from([...leafRight]),
        account,
        hasher,
        connection,
        balance,
        merkleTreePdaPublicKey,
        leftLeaf: Uint8Array.from([...leafLeft]),
        aes,
        assetLookupTable,
        merkleProof: merkleProofs[index + 1],
      });
    }
  }
}

/// This is temporary. adapt after indexer refactor.
/// uses new Balance type
/// TODO: replace with batchdecryption
/// Remove this when removing the user class
/// Ideally we don't want to mutate balance in place
export async function decryptAddUtxoToBalance_new({
  account,
  encBytes,
  index,
  commitment,
  hasher,
  connection,
  balance,
  merkleTreePdaPublicKey,
  leftLeaf,
  aes,
  assetLookupTable,
  merkleProof,
}: {
  encBytes: Uint8Array;
  index: number;
  commitment: Uint8Array;
  account: Account;
  merkleTreePdaPublicKey: PublicKey;
  hasher: Hasher;
  connection: Connection;
  balance: Balance;
  leftLeaf: Uint8Array;
  aes: boolean;
  assetLookupTable: string[];
  merkleProof: string[];
}): Promise<void> {
  const decryptedUtxo = aes
    ? await Utxo.decrypt({
        hasher,
        encBytes: encBytes,
        account: account,
        index: index,
        commitment,
        aes,
        merkleTreePdaPublicKey,
        assetLookupTable,
        merkleProof,
      })
    : await Utxo.decryptUnchecked({
        hasher,
        encBytes: encBytes,
        account: account,
        index: index,
        commitment,
        aes,
        merkleTreePdaPublicKey,
        assetLookupTable,
        merkleProof,
      });

  // null if utxo did not decrypt -> return nothing and continue
  if (!decryptedUtxo.value || decryptedUtxo.error) return;
  const utxo = decryptedUtxo.value;
  const nullifier = utxo.getNullifier({ hasher, account });
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

  // valid amounts and is not app utxo
  if (
    amountsValid &&
    utxo.verifierAddress.toBase58() === new PublicKey(0).toBase58() &&
    utxo.appDataHash.toString() === "0"
  ) {
    if (!queuedLeavesPdaExists && !nullifierExists) {
      addUtxoToBalance(utxo, balance, hasher);
    }
  }
  return;
}
