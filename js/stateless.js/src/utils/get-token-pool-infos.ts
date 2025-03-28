import {
    AddressLookupTableProgram,
    Connection,
    Keypair,
    PublicKey,
    Signer,
} from '@solana/web3.js';
import { buildAndSignTx, sendAndConfirmTx } from './send-and-confirm';
import { dedupeSigner } from '../actions';
import { StateTreeInfo, TreeType } from '../state/types';
import { Rpc } from '../rpc';
import { TokenPoolInfo } from '../rpc-interface';
import BN from 'bn.js';

/**
 * Get a random token pool info from the token pool infos.
 * Filters out token pool infos that are not initialized.
 * Filters out token pools with insufficient balance.
 * Returns multiple token pool infos if multiple will be required for the required amount.
 *
 * @param infos The token pool infos
 * @returns A random token pool info
 */
export function pickTokenPoolInfos(
    infos: TokenPoolInfo[],
    amount: number,
): TokenPoolInfo[] {
    // Shuffle the infos array
    for (let i = infos.length - 1; i > 0; i--) {
        const j = Math.floor(Math.random() * (i + 1));
        [infos[i], infos[j]] = [infos[j], infos[i]];
    }

    // Find the first info where balance is 10x the requested amount
    const sufficientBalanceInfo = infos.find(info =>
        info.balance.gte(new BN(amount).mul(new BN(10))),
    );

    // If none found, return all infos
    return sufficientBalanceInfo ? [sufficientBalanceInfo] : infos;
}
