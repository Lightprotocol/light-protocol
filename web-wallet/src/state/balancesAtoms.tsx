import { atom } from "jotai";
import { DECIMALS_SOL, TOKEN_REGISTRY, Token } from "../constants";
import { priceOracleAtom } from "./priceOracleAtoms";
import { userUtxosAtom } from "./userUtxoAtoms";
import { PublicKey, Connection } from "@solana/web3.js";
import { getPublicBalances } from "../util/getPublicBalances";
import { activePublicBalanceAtom, activeFeeConfigAtom } from "./activeAtoms";
import { connectWalletAtom, userKeypairsAtom } from "./userAtoms";

export type UserShieldedBalance = {
  token: string;
  amount: number;
  decimals: number;
  uiAmount: number;
  usdValue: number;
};

export const userShieldedBalanceAtom = atom((get) => {
  let userUtxos = get(userUtxosAtom);
  let userShieldedBalance: UserShieldedBalance[] = TOKEN_REGISTRY.map((t) => ({
    token: t.symbol,
    amount: 0,
    decimals: t.decimals,
    uiAmount: 0,
    usdValue: 0,
  }));

  const { usdPerSol, usdPerUsdc } = get(priceOracleAtom);

  userUtxos
    .filter((userUtxo) => !userUtxo.spent)
    .forEach((userUtxo) => {
      if (userUtxo.spent) console.error("filter dont work!"); // TODO: rm
      let token = userUtxo.token;
      let amount = Number(userUtxo.utxo.amount);
      let decimals = TOKEN_REGISTRY.find((t) => t.symbol === token)?.decimals;
      if (!decimals) return;
      let uiAmount = amount / decimals; //Math.pow(10, decimals);
      let usdValue = uiAmount * (token === Token.SOL ? usdPerSol : usdPerUsdc);
      let existingToken = userShieldedBalance.find((b) => b.token === token);
      if (existingToken) {
        existingToken.amount += amount;
        existingToken.uiAmount += uiAmount;
        existingToken.usdValue += usdValue;
      } else {
        throw new Error(`token ${token} is not registered`);
      }
    });

  return userShieldedBalance;
});

// utxos have been decrypted and dedicated atom is updated then
export const shieldedBalanceIsFetchedAtom = atom((get) => {
  let userShieldedBalance = get(userShieldedBalanceAtom);
  return userShieldedBalance.length > 0;
});

export const shieldedSolBalanceAtom = atom((get) => {
  let userShieldedBalance = get(userShieldedBalanceAtom);
  let solBalance = userShieldedBalance.find((b) => b.token === Token.SOL);
  return solBalance;
});

export const shieldedUsdcBalanceAtom = atom((get) => {
  let userShieldedBalance = get(userShieldedBalanceAtom);
  let usdcBalance = userShieldedBalance.find((b) => b.token === Token.USDC);
  return usdcBalance;
});

export const publicSolBalanceAtom = atom((get) => {
  let userPublicBalance = get(publicBalancesAtom);
  let solBalance = userPublicBalance.find((b) => b.token === Token.SOL);
  return solBalance;
});
// derive totalUserShieldedBalance from userShieldedBalanceAtom
export const totalUserShieldedBalanceAtom = atom((get) => {
  let userShieldedBalance = get(userShieldedBalanceAtom);
  let totalUserShieldedBalance = userShieldedBalance.reduce(
    (acc, b) => acc + b.usdValue,
    0,
  );
  return totalUserShieldedBalance;
});

/**
 *
 *
 *
 *
 *
 */

export type PublicBalance = {
  token: string;
  amount: number;
  decimals: number;
  uiAmount: number;
  publicKey: PublicKey;
  usdValue: number;
};

export const publicBalancesAtom = atom<PublicBalance[]>([]);

export const fetchedPublicBalancesAtom = atom(
  (get) => get(publicBalancesAtom),
  async (get, set, connection: Connection) => {
    let { usdPerSol, usdPerUsdc } = get(priceOracleAtom);
    let publicKey: PublicKey = get(connectWalletAtom);
    let newBalances = await getPublicBalances({
      connection,
      publicKey,
      usdPerSol,
      usdPerUsdc,
    });
    set(publicBalancesAtom, newBalances);
  },
);

export const BURNER_TOKEN = "BURNER_SOL";
// TODO: could refactor to have it's own publickey atom... (makes things easier!)
export const addPublicBurnerBalanceAtom = atom(
  (get) => get(publicBalancesAtom),
  async (get, set, connection: Connection) => {
    let currentBalances = get(publicBalancesAtom);
    let burnerKeypair = get(userKeypairsAtom).burnerKeypair;
    if (!burnerKeypair) return;
    let burnerPublicKey = burnerKeypair.publicKey;
    let burnerBalance = await connection.getBalance(burnerPublicKey);
    let burnerPublicBalance = {
      token: BURNER_TOKEN,
      publicKey: burnerPublicKey,
      amount: burnerBalance,
      decimals: DECIMALS_SOL,
      uiAmount: burnerBalance / DECIMALS_SOL,
      usdValue: (burnerBalance / DECIMALS_SOL) * get(priceOracleAtom).usdPerSol,
    };
    // if cant find BURNER_TOKEN, add it
    if (!currentBalances.find((b) => b.token === BURNER_TOKEN)) {
      set(publicBalancesAtom, [...currentBalances, burnerPublicBalance]);
    }
    // else, update it
    else {
      set(
        publicBalancesAtom,
        currentBalances.map((b) =>
          b.token === BURNER_TOKEN ? burnerPublicBalance : b,
        ),
      );
    }
  },
);

/** custom for deposit form */
export const shieldBalanceDisplayAtom = atom((get) => {
  let activePublicBalance = get(activePublicBalanceAtom);

  if (activePublicBalance.token === Token.SOL) {
    let activeFeeConfig = get(activeFeeConfigAtom);
    let { DECIMALS, TOTAL_FEES_DEPOSIT, DEPOSIT_COLLATERAL } = activeFeeConfig;
    let shieldBalanceDisplay =
      (activePublicBalance.amount - TOTAL_FEES_DEPOSIT - DEPOSIT_COLLATERAL) /
      DECIMALS;
    return shieldBalanceDisplay;
  } else {
    return activePublicBalance.uiAmount;
  }
});

export const burnerBalanceAtom = atom((get) => {
  let publicBalances = get(publicBalancesAtom);
  let burnerBalance: PublicBalance | undefined = publicBalances.find(
    (b) => b.token === BURNER_TOKEN,
  );
  if (!burnerBalance)
    return {
      token: BURNER_TOKEN,
      publicKey: null,
      amount: 0,
      decimals: DECIMALS_SOL,
      uiAmount: 0,
    };
  return burnerBalance;
});
