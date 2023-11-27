import { BN } from "@coral-xyz/anchor";
import { Utxo } from "../utxo";
import { PublicKey } from "@solana/web3.js";
import { BN_0, TOKEN_REGISTRY } from "../constants";
import { Provider } from "../wallet";
/**
 * the SOL asset is always the 0th index in the UTXO
 * the SPL asset is an optional 1st index in the UTXO
 */
const UTXO_ASSET_SPL_INDEX = 1;
const UTXO_ASSET_SOL_INDEX = 0;

// TODO: add history (spentutxos)

/**
 * We keep spent UTXOs in a separate type,
 * because we need to keep Balance up2date at
 * any time, and syncing spent UTXOs is expensive.
 */
export type Balance = {
  // key is token
  // includes only unspent UTXOs
  tokenBalances: Map<string, TokenBalance>;
  // TODO: add programBalances
};

export type TokenBalance = {
  amount: BN;
  lamports: BN; // rent
  data: TokenData;
  utxos: Utxo[];
};

// from ctx
export type TokenData = {
  symbol: string;
  decimals: BN;
  isNft: boolean;
  isNative: boolean;
  mint: PublicKey;
};
export type SerializedTokenBalance = Omit<TokenBalance, "utxos"> & {
  utxos: string[];
};

export function getTokenDataByMint(
  mintToFind: PublicKey,
  tokenRegistry: Map<string, TokenData>,
): TokenData {
  for (let value of tokenRegistry.values()) {
    if (value.mint.equals(mintToFind)) {
      return value;
    }
  }
  throw new Error(`Token with mint ${mintToFind} not found in token registry.`);
}

export function initTokenBalance(
  tokenData: TokenData,
  utxos?: Utxo[],
): TokenBalance {
  let amount = BN_0;
  let lamports = BN_0;

  if (utxos) {
    utxos.forEach((utxo) => {
      if (!utxo.assets[UTXO_ASSET_SPL_INDEX].equals(tokenData.mint)) {
        throw new Error(`UTXO does not match mint ${tokenData.mint}`);
      }
      amount = amount.add(utxo.amounts[UTXO_ASSET_SPL_INDEX] ?? BN_0);
      lamports = lamports.add(utxo.amounts[UTXO_ASSET_SOL_INDEX]);
    });
  }

  return {
    amount: amount,
    lamports: lamports,
    data: tokenData,
    utxos: utxos || [],
  };
}

export function isSPLUtxo(utxo: Utxo): boolean {
  return !utxo.amounts[UTXO_ASSET_SPL_INDEX].eqn(0);
}

export function updateTokenBalanceWithUtxo(
  utxo: Utxo,
  tokenBalance: TokenBalance,
): boolean {
  const utxoExists = tokenBalance.utxos.some(
    (existingUtxo) => existingUtxo._commitment === utxo._commitment,
  );
  if (utxoExists) return false;

  tokenBalance.utxos.push(utxo);
  tokenBalance.lamports = tokenBalance.lamports.add(
    utxo.amounts[UTXO_ASSET_SOL_INDEX],
  );

  tokenBalance.amount = tokenBalance.amount.add(
    utxo.amounts[UTXO_ASSET_SPL_INDEX] ?? BN_0,
  );

  return true;
}

export function addUtxoToBalance(utxo: Utxo, balance: Balance): boolean {
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

  return updateTokenBalanceWithUtxo(utxo, tokenBalance);
}

export function spendUtxo(balance: Balance[], commitment: string): boolean {
  for (let i = 0; i < balance.length; i++) {
    for (const [_assetKey, tokenBalance] of balance[i].tokenBalances) {
      // Find the utxo with the given commitment
      const utxoIndex = tokenBalance.utxos.findIndex(
        (utxo) => utxo._commitment === commitment,
      );
      // If found, remove it from the utxos array
      if (utxoIndex !== -1) {
        const [spentUtxo] = tokenBalance.utxos.splice(utxoIndex, 1);
        tokenBalance.lamports = tokenBalance.lamports.sub(
          spentUtxo.amounts[UTXO_ASSET_SOL_INDEX],
        );
        tokenBalance.amount = tokenBalance.amount.sub(
          spentUtxo.amounts[UTXO_ASSET_SPL_INDEX] ?? BN_0,
        );
        return true;
      }
    }
  }
  return false;
}

/**
 *
 * @param balance
 * @returns
 * TODO: we can consider using a more efficient serialization format.
 * however, this won't be a bottleneck since the balance will consist of a
 * few dozen UTXOs at most.
 */
export async function serializeBalance(balance: Balance): Promise<string> {
  const clonedBalance: Balance = JSON.parse(JSON.stringify(balance));
  const serializedBalance: {
    tokenBalances: Map<string, SerializedTokenBalance>;
  } = { tokenBalances: new Map() };

  for (const [key, tokenBalance] of clonedBalance.tokenBalances.entries()) {
    const utxosAsString: string[] = [];
    for (let i = 0; i < tokenBalance.utxos.length; i++) {
      utxosAsString[i] = await tokenBalance.utxos[i].toString();
    }
    const serializedTokenBalance: SerializedTokenBalance = {
      ...tokenBalance,
      utxos: utxosAsString,
    };
    serializedBalance.tokenBalances.set(key, serializedTokenBalance);
  }
  return JSON.stringify(serializedBalance);
}

// ideally we would not pass provider, but poseidon and assetLookupTable separately
// TODO: consider passing params explicitly, after we dealt we the provider class
export async function deserializeBalance(
  serializedBalance: string,
  provider: Provider,
): Promise<Balance> {
  const parsedBalance: {
    tokenBalances: Map<string, SerializedTokenBalance>;
  } = JSON.parse(serializedBalance);

  const balance: Balance = { tokenBalances: new Map() };

  for (const [key, serializedTokenBalance] of Object.entries(
    parsedBalance.tokenBalances,
  )) {
    const utxos: Utxo[] = [];
    for (let i = 0; i < serializedTokenBalance.utxos.length; i++) {
      // Assuming Utxo has a static method fromString to convert a string back into a Utxo object
      utxos[i] = Utxo.fromString(
        serializedTokenBalance.utxos[i],
        provider.poseidon,
        provider.lookUpTables.assetLookupTable,
      );
    }
    const tokenBalance: TokenBalance = {
      ...serializedTokenBalance,
      utxos: utxos,
    };
    balance.tokenBalances.set(key, tokenBalance);
  }

  return balance;
}

// export async function syncBalance(balance: Balance) {
//   // identify spent utxos
//   for (const [, tokenBalance] of balance.tokenBalances) {
//     for (const [key, utxo] of tokenBalance.utxos) {
//       const nullifierAccountInfo = await fetchNullifierAccountInfo(
//         utxo.getNullifier({
//           poseidon: this.provider.poseidon,
//           account: this.account,
//         })!,
//         this.provider.provider.connection,
//       );
//       if (nullifierAccountInfo !== null) {
//         // tokenBalance.utxos.delete(key)
//         tokenBalance.moveToSpentUtxos(key);
//       }
//     }
//   }
// }

// const lastSyncedBlock = 0;
// const accountCreationBlock = 0;

// export type SyncConfig = {
//   lastSyncedBlock: number;
//   accountCreationBlock: number;
//   shouldLazyFetchInbox: boolean;
// };
