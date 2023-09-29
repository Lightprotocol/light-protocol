// TODO: deal with this: set own payer just for that? where is this used?

import {
  TransactionSignature,
  Transaction as SolanaTransaction,
  PublicKey,
} from "@solana/web3.js";
import { TransactionError, TransactionErrorCode } from "./errors";
import { Provider, TransactionParameters } from "./index";
import { Idl } from "@coral-xyz/anchor";

// TODO: add test
// This is used by applications not the relayer
export async function closeVerifierState(
  provider: Provider,
  verifierIdl: Idl,
  verifierState: PublicKey,
): Promise<TransactionSignature> {
  if (!provider.wallet)
    throw new TransactionError(
      TransactionErrorCode.WALLET_UNDEFINED,
      "closeVerifierState",
      "Cannot use closeVerifierState without wallet",
    );

  const transaction = new SolanaTransaction().add(
    await TransactionParameters.getVerifierProgram(
      verifierIdl,
      provider.provider,
    )
      .methods.closeVerifierState()
      .accounts({
        signingAddress: provider.wallet.publicKey,
        verifierState,
      })
      .instruction(),
  );

  return await provider.wallet!.sendAndConfirmTransaction(transaction);
}
