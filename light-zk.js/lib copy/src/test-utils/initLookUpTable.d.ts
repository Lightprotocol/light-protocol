/// <reference types="node" />
import { Provider } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { PathLike } from "fs";
export declare function initLookUpTableFromFile(provider: anchor.AnchorProvider, path?: PathLike, extraAccounts?: Array<PublicKey>): Promise<PublicKey>;
export declare function initLookUpTableTest(provider: Provider, lookupTableAddress: PublicKey, recentSlot: number, extraAccounts?: Array<PublicKey>): Promise<PublicKey>;
