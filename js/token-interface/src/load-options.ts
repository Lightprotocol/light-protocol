import type { PublicKey } from '@solana/web3.js';
import type { SplPoolInfo } from './spl-interface';

export interface LoadOptions {
    splPoolInfos?: SplPoolInfo[];
    wrap?: boolean;
    delegatePubkey?: PublicKey;
}
