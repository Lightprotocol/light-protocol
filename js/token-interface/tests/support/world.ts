import { World, IWorldOptions, setWorldConstructor } from '@cucumber/cucumber';
import type { Keypair, PublicKey, TransactionInstruction } from '@solana/web3.js';
import type { MintFixture } from '../e2e/helpers.js';

export class TokenInterfaceWorld extends World {
    // ---- Fixture state (e2e) ----
    fixture?: MintFixture;
    owner?: Keypair;
    sender?: Keypair;
    recipient?: Keypair;
    delegate?: Keypair;

    // ---- Instruction results ----
    instructions: TransactionInstruction[] = [];
    lastApproveInstructions: TransactionInstruction[] = [];
    lastRevokeInstructions: TransactionInstruction[] = [];
    transactionSignature?: string;

    // ---- Assertion targets ----
    resultBalance?: bigint;
    resultAmounts?: bigint[];
    resultError?: Error;
    resultAccount?: any;
    resultDelegateInfo?: {
        delegate: PublicKey | null;
        delegatedAmount: bigint;
    };
    resultState?: number;

    // ---- Unit test state ----
    keypairs: Record<string, PublicKey> = {};
    instruction?: TransactionInstruction;
    builtInstructions: Record<string, TransactionInstruction> = {};
    kitInstructions?: any[];
    errorInstance?: Error;

    constructor(options: IWorldOptions) {
        super(options);
    }
}

setWorldConstructor(TokenInterfaceWorld);
