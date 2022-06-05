import { Connection } from '@solana/web3.js';
import MerkelTree from './merkelTree';
export declare const buildMerkelTree: (connection: Connection) => Promise<MerkelTree>;
