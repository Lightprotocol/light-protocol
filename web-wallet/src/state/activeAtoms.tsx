import { atom } from "jotai";
import { FEE_USDC_PLUS_ATA, Token, selectConst } from "../constants";
import { Utxo } from "../sdk/src/utxo";
import {
  publicBalancesAtom,
  userShieldedBalanceAtom,
  PublicBalance,
} from "./balancesAtoms";
import { priceOracleAtom } from "./priceOracleAtoms";
import { userUtxosAtom, ActiveFeeConfig } from "./userUtxoAtoms";
import { recipientAtaIsInitializedAtom } from "./utilAtoms";

export const activeTokenAtom = atom<string>(Token.SOL);

export const activeShieldedBalanceAtom = atom((get) => {
  let activeToken = get(activeTokenAtom);
  let userShieldedBalance = get(userShieldedBalanceAtom);
  let activeShieldBalance = userShieldedBalance.find(
    (b) => b.token === activeToken,
  );
  return activeShieldBalance;
});

export const activeUserUtxosAtom = atom((get) => {
  let activeToken = get(activeTokenAtom);
  let userUtxos = get(userUtxosAtom);
  let activeUserUtxos = userUtxos.filter((u) => u.token === activeToken);
  return activeUserUtxos;
});
export const activeUnspentUtxosAtom = atom((get) => {
  let activeUserUtxos = get(activeUserUtxosAtom);
  let activeUnspentUserUtxos = activeUserUtxos.filter((u) => !u.spent);
  let activeUnspentUtxos: Utxo[] = activeUnspentUserUtxos.map((u) => u.utxo);
  return activeUnspentUtxos;
});

export const activeFeeConfigAtom = atom((get) => {
  let activeToken = get(activeTokenAtom);
  let activeFeeConfig: ActiveFeeConfig = selectConst(activeToken);
  let ataIsInitialized = get(recipientAtaIsInitializedAtom);
  if (!ataIsInitialized) {
    activeFeeConfig = {
      ...activeFeeConfig,
      TOTAL_FEES_WITHDRAWAL: FEE_USDC_PLUS_ATA,
    };
    return activeFeeConfig;
  }
  return activeFeeConfig;
});

export const activeConversionRateAtom = atom((get) => {
  let activeToken = get(activeTokenAtom);
  let activeConversionRate =
    get(priceOracleAtom)[
      activeToken === Token.SOL ? "usdPerSol" : "usdPerUsdc"
    ];
  return activeConversionRate;
});

export const activePublicBalanceAtom = atom((get) => {
  let activeToken = get(activeTokenAtom);
  let userPublicBalances = get(publicBalancesAtom);
  let activePublicBalance: PublicBalance | undefined = userPublicBalances.find(
    (b) => b.token === activeToken,
  );
  if (!activePublicBalance)
    return {
      token: activeToken,
      amount: 0,
      decimals: 9,
      uiAmount: 0,
      usdValue: 0,
      publicKey: null,
    };
  return activePublicBalance;
});
