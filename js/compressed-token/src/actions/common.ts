import { Signer, PublicKey } from '@solana/web3.js';

/** @internal */
export function getSigners(
  signerOrMultisig: Signer | PublicKey,
  multiSigners: Signer[],
): [PublicKey, Signer[]] {
  // TODO: add multisig support
  if (multiSigners.length > 0) throw new Error('Multisig not supported yet.');

  if (signerOrMultisig instanceof PublicKey)
    throw new Error('Multisig not supported yet.');

  return signerOrMultisig instanceof PublicKey
    ? [signerOrMultisig, multiSigners]
    : [signerOrMultisig.publicKey, [signerOrMultisig]];
}

/** @internal remove signer from signers if part of signers */
export function dedupeSigner(signer: Signer, signers: Signer[]): Signer[] {
  if (signers.includes(signer)) {
    return signers.filter(
      s => s.publicKey.toString() !== signer.publicKey.toString(),
    );
  }
  return signers;
}
