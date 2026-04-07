import type { LoadOptions } from './load-options';
import { getMint } from './read';
import { PublicKey } from '@solana/web3.js';
import type { Rpc } from '@lightprotocol/stateless.js';

export async function getMintDecimals(
    rpc: Rpc,
    mint: PublicKey,
): Promise<number> {
    const mintInfo = await getMint(rpc, mint);
    return mintInfo.mint.decimals;
}

export function toLoadOptions(
    owner: PublicKey,
    authority?: PublicKey,
    wrap = false,
): LoadOptions | undefined {
    if ((!authority || authority.equals(owner)) && !wrap) {
        return undefined;
    }

    const options: LoadOptions = {};
    if (wrap) {
        options.wrap = true;
    }
    if (authority && !authority.equals(owner)) {
        options.delegatePubkey = authority;
    }

    return options;
}

export function toBigIntAmount(amount: number | bigint): bigint {
    return BigInt(amount.toString());
}
