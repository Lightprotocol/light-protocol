import {
    getAssociatedTokenAddressInterface,
} from '@lightprotocol/compressed-token';
import type { PublicKey } from '@solana/web3.js';
import { getAta as getTokenInterfaceAta } from './account';
import type { AtaOwnerInput, GetAtaInput, TokenInterfaceAccount } from './types';

export function getAtaAddress({ mint, owner, programId }: AtaOwnerInput): PublicKey {
    return getAssociatedTokenAddressInterface(mint, owner, false, programId);
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
