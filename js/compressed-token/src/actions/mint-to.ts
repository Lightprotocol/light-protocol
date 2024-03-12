import {
  ConfirmOptions,
  Connection,
  PublicKey,
  Signer,
  TransactionSignature,
} from '@solana/web3.js';
import { CompressedTokenProgram } from '../program';
import {
  defaultTestStateTreeAccounts,
  sendAndConfirmTx,
} from '@lightprotocol/stateless.js';
import { buildAndSignTx } from '@lightprotocol/stateless.js';
import { BN } from '@coral-xyz/anchor';

/**
 * Mint compressed tokens to a solana address
 *
 * @param connection     Connection to use
 * @param payer          Payer of the transaction fees
 * @param mint           Mint for the account
 * @param destination    Address of the account to mint to
 * @param authority      Minting authority
 * @param amount         Amount to mint
 * @param multiSigners   Signing accounts if `authority` is a multisig
 * @param merkleTree     State tree account that the compressed tokens should be
 *                       part of. Defaults to the default state tree account.
 * @param confirmOptions Options for confirming the transaction
 *
 * @return Signature of the confirmed transaction
 */
export async function mintTo(
  connection: Connection,
  payer: Signer,
  mint: PublicKey,
  destination: PublicKey,
  authority: Signer | PublicKey,
  amount: number | BN,
  multiSigners: Signer[] = [],
  merkleTree: PublicKey = defaultTestStateTreeAccounts().merkleTree, // DEFAULT IF NOT PROVIDED
  confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
  const [authorityPubkey, additionalSigners] = getSigners(
    authority,
    multiSigners,
  );
  const ix = await CompressedTokenProgram.mintTo({
    feePayer: payer.publicKey,
    mint: mint,
    authority: authorityPubkey,
    amount: amount,
    toPubkey: destination,
    merkleTree,
  });

  const { blockhash } = await connection.getLatestBlockhash();

  const tx = buildAndSignTx([ix], payer, blockhash, additionalSigners);

  const txId = await sendAndConfirmTx(connection, tx, confirmOptions);

  return txId;
}

/** @internal */
export function getSigners(
  signerOrMultisig: Signer | PublicKey,
  multiSigners: Signer[],
): [PublicKey, Signer[]] {
  return signerOrMultisig instanceof PublicKey
    ? [signerOrMultisig, multiSigners]
    : [signerOrMultisig.publicKey, [signerOrMultisig]];
}
