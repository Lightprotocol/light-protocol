import { Signer, PublicKey } from '@solana/web3.js';

/** @internal */
export function getSigners(
    signerOrMultisig: Signer | PublicKey,
    multiSigners: Signer[],
): [PublicKey, Signer[]] {
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
