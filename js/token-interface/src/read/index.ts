import type { PublicKey } from '@solana/web3.js';
import { getAta as getTokenInterfaceAta } from '../account';
import type {
    AtaOwnerInput,
    GetAtaInput,
    TokenInterfaceAccount,
} from '../types';
import { getAssociatedTokenAddress } from './associated-token-address';

export { getAssociatedTokenAddress } from './associated-token-address';
export * from './ata-utils';
export { getMint } from './get-mint';
export type { MintInfo } from './get-mint';
export * from './get-account';

export function getAtaAddress({
    mint,
    owner,
    programId,
}: AtaOwnerInput): PublicKey {
    return getAssociatedTokenAddress(mint, owner, false, programId);
}

export async function getAta({
    rpc,
    owner,
    mint,
    commitment,
}: GetAtaInput): Promise<TokenInterfaceAccount> {
    return getTokenInterfaceAta({
        rpc,
        owner,
        mint,
        commitment,
    });
}
