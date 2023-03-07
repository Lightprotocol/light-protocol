import { TransactionSignature } from "@solana/web3.js";
import { Provider } from "light-sdk";
export declare function sendTransaction(ix: any, provider: Provider): Promise<TransactionSignature | undefined>;
