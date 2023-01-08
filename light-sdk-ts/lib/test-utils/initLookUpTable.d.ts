/// <reference types="node" />
import { Provider } from "@project-serum/anchor";
import { PublicKey } from "@solana/web3.js";
import * as anchor from "@project-serum/anchor";
import { PathOrFileDescriptor } from "fs";
export declare function initLookUpTableFromFile(provider: anchor.Provider, path?: PathOrFileDescriptor, extraAccounts?: Array<PublicKey>): Promise<PublicKey>;
export declare function initLookUpTable(provider: Provider, lookupTableAddress: PublicKey, recentSlot: number, extraAccounts?: Array<PublicKey>): Promise<PublicKey>;
