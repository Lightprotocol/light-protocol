import { Signer } from '@solana/web3.js';

/** @internal remove signer from signers if part of signers */
export function dedupeSigner(signer: Signer, signers: Signer[]): Signer[] {
    if (signers.includes(signer)) {
        return signers.filter(
            s => s.publicKey.toString() !== signer.publicKey.toString(),
        );
    }
    return signers;
}
