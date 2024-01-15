import {
  UserIndexedTransaction,
  TOKEN_REGISTRY,
  TokenUtxoBalance,
} from "@lightprotocol/zk.js";

import { BN } from "@coral-xyz/anchor";

export function parseAmount(amount: BN, tokenCtx: any, decimals = 4) {
  let { div: quotient, mod: remainder } = amount.divmod(tokenCtx.decimals);

  // We're using BN to prevent overflowing Number.MAX_SAFE_INTEGER
  let remainderDecimal = remainder
    .mul(new BN(10).pow(new BN(decimals)))
    .div(tokenCtx.decimals);

  // Convert to string and pad with zeros if necessary
  let remainderString = remainderDecimal.toString(10).padStart(decimals, "0");

  // If the first decimals place is a trailing zero just return the integer
  if (remainderString === "0".repeat(decimals)) {
    return `${quotient.toString()}`;
  } else {
    return `${quotient.toString()}.${remainderString}`;
  }
}

export const parseTxAmount = (tx: UserIndexedTransaction) => {
  const amountSpl = new BN(tx.publicAmountSpl, "hex");
  const amountSol = new BN(tx.publicAmountSol, "hex");
  const isSpl = amountSpl.toNumber() > 0;
  const isTransfer = amountSpl.toNumber() === 0 && amountSol.toNumber() === 0;
  const tokenCtx = isSpl
    ? TOKEN_REGISTRY.get("USDC")!
    : TOKEN_REGISTRY.get("SOL")!;

  let val = isTransfer
    ? "encrypted"
    : isSpl
    ? parseAmount(amountSpl, tokenCtx)
    : parseAmount(amountSol, tokenCtx);

  return val;
};

export function parseCompressedBalance(tokenBalance: TokenUtxoBalance) {
  let _token = tokenBalance.tokenData.symbol;
  let tokenCtx = TOKEN_REGISTRY.get(_token)!;
  let balance =
    _token === "SOL"
      ? parseAmount(tokenBalance.totalBalanceSol, tokenCtx, 9)
      : parseAmount(tokenBalance.totalBalanceSpl, tokenCtx, 6);
  let utxoNumber = tokenBalance.utxos.size;

  return {
    token: _token,
    balance: balance,
    utxos: utxoNumber,
  };
}
