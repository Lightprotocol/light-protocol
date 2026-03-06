/**
 * Shared E2E test helpers for reading light-token account state via parseLightTokenHot.
 * Use these instead of magic byte offsets (e.g. info.data[108], readBigUInt64LE(64)).
 */
import { PublicKey } from '@solana/web3.js';
import type { Rpc } from '@lightprotocol/stateless.js';
import { parseLightTokenHot } from '../../src/v3/get-account-interface';
import { AccountState } from '@solana/spl-token';

export async function getLightTokenBalance(
    rpc: Rpc,
    address: PublicKey,
): Promise<bigint> {
    const info = await rpc.getAccountInfo(address);
    if (!info) return BigInt(0);
    const { parsed } = parseLightTokenHot(address, info);
    return parsed.amount;
}

export async function getLightTokenState(
    rpc: Rpc,
    address: PublicKey,
): Promise<AccountState> {
    const info = await rpc.getAccountInfo(address);
    if (!info) throw new Error(`Account not found: ${address.toBase58()}`);
    const { parsed } = parseLightTokenHot(address, info);
    return parsed.isFrozen
        ? AccountState.Frozen
        : parsed.isInitialized
          ? AccountState.Initialized
          : AccountState.Uninitialized;
}
