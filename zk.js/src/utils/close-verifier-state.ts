// TODO: deal with this: set own payer just for that? where is this used?

import {
  TransactionSignature,
  Transaction as SolanaTransaction,
  PublicKey,
} from "@solana/web3.js";
import { Idl } from "@coral-xyz/anchor";
import { TransactionError, TransactionErrorCode } from "../errors";
import { Provider } from "../provider";
import { getVerifierProgram } from "../transaction/psp-util";

// TODO: add test
// This is used by applications not the rpc
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
    await getVerifierProgram(verifierIdl, provider.provider)
      .methods.closeVerifierState()
      .accounts({
        signingAddress: provider.wallet.publicKey,
        verifierState,
      })
      .instruction(),
  );

  return await provider.wallet!.sendAndConfirmTransaction(transaction);
}
