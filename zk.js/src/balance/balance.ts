import { Utxo } from "../utxo";
import { PublicKey } from "@solana/web3.js";
import {
  BN_0,
  TOKEN_REGISTRY,
  UTXO_ASSET_SOL_INDEX,
  UTXO_ASSET_SPL_INDEX,
} from "../constants";
import { Provider } from "../wallet";
import { Poseidon } from "../types/poseidon";
import { TokenData, SerializedTokenBalance } from "../types";
import { Balance, TokenBalance } from "../types/balance";

export const isSPLUtxo = (utxo: Utxo): boolean => {
  return !utxo.amounts[UTXO_ASSET_SPL_INDEX].eqn(0);
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
  tokenBalance.lamports = tokenBalance.lamports.add(
    utxo.amounts[UTXO_ASSET_SOL_INDEX],
  );

  tokenBalance.amount = tokenBalance.amount.add(
    utxo.amounts[UTXO_ASSET_SPL_INDEX] ?? BN_0,
  );

  return true;
}

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
export function spendUtxo(balance: Balance[], commitment: string): boolean {
  for (let i = 0; i < balance.length; i++) {
    for (const [_assetKey, tokenBalance] of balance[i].tokenBalances) {
      const utxoIndex = tokenBalance.utxos.findIndex(
        (utxo) => utxo._commitment === commitment,
      );
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
    mint: tokenBalance.data.mint.toString(),
    utxos: utxos,
  };

  return serializedTokenBalance;
}

function deserializeTokenBalance(
  serializedTokenBalance: SerializedTokenBalance,
  tokenRegistry: Map<string, TokenData>,
  provider: Provider,
): TokenBalance {
  const tokenData = getTokenDataByMint(
    new PublicKey(serializedTokenBalance.mint),
    tokenRegistry,
  );

  const utxos = serializedTokenBalance.utxos.map((serializedUtxo) => {
    const utxo = Utxo.fromString(
      serializedUtxo.utxo,
      provider.poseidon,
      provider.lookUpTables.assetLookupTable,
    );

    const index = serializedUtxo.index;
    utxo.index = index;
    return utxo;
  });

  return initTokenBalance(tokenData, utxos);
}

export async function serializeBalance(balance: Balance): Promise<string> {
  const serializedBalance: SerializedTokenBalance[] = [];

  for (const tokenBalance of balance.tokenBalances.values()) {
    serializedBalance.push(await serializeTokenBalance(tokenBalance));
  }

  return JSON.stringify(serializedBalance);
}

export function deserializeBalance(
  serializedBalance: string,
  tokenRegistry: Map<string, TokenData>,
  provider: Provider,
): Balance {
  const balance: Balance = {
    tokenBalances: new Map<string, TokenBalance>(),
  };

  const serializedTokenBalances: SerializedTokenBalance[] =
    JSON.parse(serializedBalance);

  for (const serializedTokenBalance of serializedTokenBalances) {
    const tokenBalance = deserializeTokenBalance(
      serializedTokenBalance,
      tokenRegistry,
      provider,
    );
    balance.tokenBalances.set(serializedTokenBalance.mint, tokenBalance);
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
