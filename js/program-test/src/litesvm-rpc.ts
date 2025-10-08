import { LiteSVM } from 'litesvm';
import {
    PublicKey,
    Transaction,
    VersionedTransaction,
    ConfirmedSignatureInfo,
    ParsedTransactionWithMeta,
} from '@solana/web3.js';
import {
    TestRpc,
    defaultTestStateTreeAccounts,
} from '@lightprotocol/stateless.js';
import { LiteSVMConfig } from './types';
import * as path from 'path';
import * as fs from 'fs';
import bs58 from 'bs58';

/**
 * LiteSVM-based RPC implementation for testing Light Protocol programs
 * Extends TestRpc and overrides only the blockchain interaction methods
 * All proof generation and indexing logic is inherited from TestRpc
 */
export class LiteSVMRpc extends TestRpc {
    private litesvm: LiteSVM;
    private storedTransactions: Map<string, any>;
    private storedRawTransactions: Map<string, Transaction | VersionedTransaction>;

    constructor(
        lightWasm: any,
        config?: LiteSVMConfig,
        proverEndpoint: string = 'http://127.0.0.1:3001'
    ) {
        // Initialize TestRpc with dummy endpoints
        super(
            'http://127.0.0.1:8899',
            lightWasm,
            'http://127.0.0.1:8784',
            proverEndpoint,
            { commitment: 'confirmed' },
            { depth: defaultTestStateTreeAccounts().merkleTreeHeight }
        );

        this.storedTransactions = new Map();
        this.storedRawTransactions = new Map();

        // Initialize LiteSVM with configuration
        this.litesvm = new LiteSVM()
            .withSysvars()
            .withBuiltins()
            .withDefaultPrograms()
            .withPrecompiles();

        if (config?.sigverify !== undefined) {
            this.litesvm = this.litesvm.withSigverify(config.sigverify);
        }
        if (config?.blockhashCheck !== undefined) {
            this.litesvm = this.litesvm.withBlockhashCheck(config.blockhashCheck);
        }
        if (config?.initialLamports !== undefined) {
            this.litesvm = this.litesvm.withLamports(config.initialLamports);
        }
        if (config?.transactionHistorySize !== undefined) {
            this.litesvm = this.litesvm.withTransactionHistory(config.transactionHistorySize);
        }

        // Load Light Protocol programs
        this.loadLightPrograms();

        // Load state tree account fixtures
        this.loadAccountFixtures();
    }

    /**
     * Load Light Protocol program binaries from target/deploy
     */
    private loadLightPrograms(): void {
        // When running from dist/, we need to go up to repo root
        // dist/ -> ../ -> js/program-test/ -> ../../ -> repo root
        const repoRoot = path.join(__dirname, '../../..');
        const deployPath = path.join(repoRoot, 'target/deploy');

        // Load Light Protocol programs
        const LIGHT_SYSTEM_PROGRAM_ID = new PublicKey(
            'SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7'
        );
        const ACCOUNT_COMPRESSION_PROGRAM_ID = new PublicKey(
            'compr6CUsB5m2jS4Y3831ztGSTnDpnKJTKS95d64XVq'
        );

        this.litesvm.addProgramFromFile(
            LIGHT_SYSTEM_PROGRAM_ID,
            path.join(deployPath, 'light_system_program_pinocchio.so')
        );
        this.litesvm.addProgramFromFile(
            ACCOUNT_COMPRESSION_PROGRAM_ID,
            path.join(deployPath, 'account_compression.so')
        );
    }

    /**
     * Load account fixtures from cli/accounts
     */
    private loadAccountFixtures(): void {
        const repoRoot = path.join(__dirname, '../../..');
        const accountsPath = path.join(repoRoot, 'cli/accounts');

        // Load all account JSON files from cli/accounts
        const files = fs.readdirSync(accountsPath);

        for (const filename of files) {
            if (!filename.endsWith('.json')) continue;

            const filepath = path.join(accountsPath, filename);
            const accountData = JSON.parse(fs.readFileSync(filepath, 'utf-8'));
            const pubkey = new PublicKey(accountData.pubkey);

            // Handle rentEpoch: if it exceeds JavaScript's MAX_SAFE_INTEGER or approaches u64::MAX,
            // set it to 0 to avoid overflow issues (same approach as litesvm's copyAccounts test)
            let rentEpoch = accountData.account.rentEpoch || 0;
            if (rentEpoch > Number.MAX_SAFE_INTEGER) {
                rentEpoch = 0;
            }

            const account = {
                lamports: accountData.account.lamports,
                data: Buffer.from(accountData.account.data[0], 'base64'),
                owner: new PublicKey(accountData.account.owner),
                executable: accountData.account.executable,
                rentEpoch,
            };
            this.litesvm.setAccount(pubkey, account);
        }
    }

    /**
     * Send and execute a transaction using LiteSVM
     */
    override async sendTransaction(
        transaction: Transaction | VersionedTransaction,
        ...args: any[]
    ): Promise<string> {
        const result = this.litesvm.sendTransaction(transaction);

        // Check if transaction succeeded or failed
        if ('err' in result && typeof result.err === 'function') {
            const error = result.err();
            throw new Error(`Transaction failed: ${error}`);
        }

        const successResult = result as any;
        const logs = successResult.logs();
        const signatureBytes = successResult.signature();
        const signature = bs58.encode(signatureBytes);

        // Extract inner instructions from LiteSVM result
        const innerInstructionsRaw = successResult.innerInstructions();
        const innerInstructions = innerInstructionsRaw.map((group: any[], index: number) => ({
            index,
            instructions: group.map((inner: any) => {
                const compiledIx = inner.instruction();
                return {
                    programIdIndex: compiledIx.programIdIndex(),
                    accounts: Array.from(compiledIx.accounts()),
                    data: bs58.encode(compiledIx.data()),
                };
            }),
        }));

        // Store transaction metadata for TestRpc to query later
        this.storedTransactions.set(signature, {
            signature,
            logs,
            slot: 1,
            blockTime: Math.floor(Date.now() / 1000),
            confirmationStatus: 'confirmed',
            innerInstructions,
        });

        // Store raw transaction for getParsedTransactions
        this.storedRawTransactions.set(signature, transaction);

        return signature;
    }

    /**
     * Override getSignaturesForAddress to return our stored LiteSVM transactions
     * This allows TestRpc's proof generation to work with LiteSVM transactions
     */
    override async getSignaturesForAddress(
        address: PublicKey,
        options?: any
    ): Promise<ConfirmedSignatureInfo[]> {
        // Return all stored transactions
        // TestRpc will parse these to build proofs
        return Array.from(this.storedTransactions.values()).map(tx => ({
            signature: tx.signature,
            slot: tx.slot,
            err: null,
            memo: null,
            blockTime: tx.blockTime,
            confirmationStatus: tx.confirmationStatus,
        }));
    }

    /**
     * Override getTransaction to return stored LiteSVM transaction
     */
    override async getTransaction(signature: string | Uint8Array, options?: any): Promise<any> {
        // Convert Uint8Array signature to base58 string if needed
        const sigString = typeof signature === 'string' ? signature : bs58.encode(signature);

        const tx = this.storedTransactions.get(sigString);
        const rawTx = this.storedRawTransactions.get(sigString);

        if (!tx || !rawTx) {
            return null;
        }

        // Extract message and account keys from transaction
        let message: any;
        let accountKeys: PublicKey[];
        let compiledInstructions: any[];

        if ('message' in rawTx) {
            // VersionedTransaction
            message = rawTx.message;
            // For VersionedTransaction, accountKeys are in staticAccountKeys property
            if ('staticAccountKeys' in message) {
                accountKeys = message.staticAccountKeys;
            } else if ('accountKeys' in message) {
                accountKeys = message.accountKeys;
            } else if (typeof message.getAccountKeys === 'function') {
                accountKeys = message.getAccountKeys().staticAccountKeys;
            } else {
                accountKeys = [];
            }
            compiledInstructions = message.compiledInstructions || [];
        } else {
            // Legacy Transaction - need to compile to get accountKeys
            const compiledMessage = (rawTx as Transaction).compileMessage();
            message = compiledMessage;
            accountKeys = compiledMessage.accountKeys;
            compiledInstructions = compiledMessage.instructions || [];
        }

        return {
            slot: tx.slot,
            blockTime: tx.blockTime,
            transaction: {
                message: {
                    accountKeys,
                    compiledInstructions,
                    recentBlockhash: message.recentBlockhash || message.header?.recentBlockhash || '',
                    addressTableLookups: message.addressTableLookups || [],
                },
                signatures: 'signatures' in rawTx ? rawTx.signatures : [(rawTx as any).signature],
            },
            meta: {
                err: null,
                logMessages: tx.logs,
                innerInstructions: tx.innerInstructions || [],
                preBalances: [],
                postBalances: [],
                preTokenBalances: [],
                postTokenBalances: [],
                rewards: [],
                fee: 5000,
            },
        };
    }

    /**
     * Override getParsedTransactions to return stored LiteSVM transactions in parsed format
     */
    override async getParsedTransactions(
        signatures: string[],
        options?: any
    ): Promise<(ParsedTransactionWithMeta | null)[]> {
        return signatures.map(signature => {
            const tx = this.storedTransactions.get(signature);
            const rawTx = this.storedRawTransactions.get(signature);

            if (!tx || !rawTx) {
                return null;
            }

            // Extract message and account keys from transaction
            let message: any;
            let accountKeys: PublicKey[];

            if ('message' in rawTx) {
                // VersionedTransaction
                message = rawTx.message;
                if ('staticAccountKeys' in message) {
                    accountKeys = message.staticAccountKeys;
                } else if ('accountKeys' in message) {
                    accountKeys = message.accountKeys;
                } else if (typeof message.getAccountKeys === 'function') {
                    accountKeys = message.getAccountKeys().staticAccountKeys;
                } else {
                    accountKeys = [];
                }
            } else {
                // Legacy Transaction - need to compile to get accountKeys
                const compiledMessage = (rawTx as Transaction).compileMessage();
                message = compiledMessage;
                accountKeys = compiledMessage.accountKeys;
            }

            // Convert signatures to base58 strings
            const rawSignatures = 'signatures' in rawTx ? rawTx.signatures : [(rawTx as any).signature];
            const signatures = rawSignatures.map((sig: string | Uint8Array) =>
                typeof sig === 'string' ? sig : bs58.encode(sig)
            );

            return {
                slot: tx.slot,
                blockTime: tx.blockTime,
                transaction: {
                    message: {
                        accountKeys: accountKeys.map((key: PublicKey) => ({
                            pubkey: key,
                            signer: false,
                            writable: false,
                            source: 'transaction' as const,
                        })),
                        instructions: [],
                        recentBlockhash: message.recentBlockhash || message.header?.recentBlockhash || '',
                        addressTableLookups: message.addressTableLookups || undefined,
                    },
                    signatures,
                },
                meta: {
                    err: null,
                    fee: 5000,
                    preBalances: [],
                    postBalances: [],
                    innerInstructions: tx.innerInstructions || [],
                    preTokenBalances: [],
                    postTokenBalances: [],
                    logMessages: tx.logs,
                    rewards: [],
                    loadedAddresses: undefined,
                    computeUnitsConsumed: undefined,
                },
                version: options?.maxSupportedTransactionVersion || 0,
            } as ParsedTransactionWithMeta;
        });
    }

    /**
     * Airdrop SOL to an account using LiteSVM
     */
    override async requestAirdrop(
        pubkey: PublicKey,
        lamports: number
    ): Promise<string> {
        this.litesvm.airdrop(pubkey, BigInt(lamports));
        return 'mock-airdrop-signature';
    }

    /**
     * Get account info using LiteSVM
     */
    override async getAccountInfo(
        publicKey: PublicKey,
        commitmentOrConfig?: any
    ): Promise<any> {
        const account = this.litesvm.getAccount(publicKey);
        if (!account) {
            return null;
        }
        return {
            executable: account.executable,
            owner: account.owner,
            lamports: Number(account.lamports),
            data: account.data,
            rentEpoch: account.rentEpoch,
        };
    }

    /**
     * Get multiple account infos using LiteSVM
     */
    override async getMultipleAccountsInfo(
        publicKeys: PublicKey[],
        commitmentOrConfig?: any
    ): Promise<(any | null)[]> {
        return publicKeys.map(publicKey => {
            const account = this.litesvm.getAccount(publicKey);
            if (!account) {
                return null;
            }
            return {
                executable: account.executable,
                owner: account.owner,
                lamports: Number(account.lamports),
                data: account.data,
                rentEpoch: account.rentEpoch,
            };
        });
    }

    /**
     * Get balance using LiteSVM
     */
    override async getBalance(publicKey: PublicKey): Promise<number> {
        return Number(this.litesvm.getBalance(publicKey));
    }

    /**
     * Get minimum balance for rent exemption
     */
    override async getMinimumBalanceForRentExemption(
        dataLength: number,
        commitment?: any
    ): Promise<number> {
        return Number(this.litesvm.minimumBalanceForRentExemption(BigInt(dataLength)));
    }

    /**
     * Simulate a transaction without executing it
     */
    override async simulateTransaction(
        transactionOrMessage: any,
        configOrSigners?: any,
        includeAccounts?: any
    ): Promise<any> {
        // Extract transaction from possible message wrapper
        const transaction = 'message' in transactionOrMessage
            ? transactionOrMessage
            : transactionOrMessage;

        const result = this.litesvm.simulateTransaction(transaction);

        // Check if simulation failed
        if ('err' in result && typeof result.err === 'function') {
            const error = result.err();
            return {
                context: { slot: 1 },
                value: {
                    err: error,
                    logs: [],
                    accounts: null,
                    unitsConsumed: 0,
                    returnData: null,
                },
            };
        }

        const simResult = result as any;
        const meta = simResult.meta();

        return {
            context: { slot: 1 },
            value: {
                err: null,
                logs: meta.logs(),
                accounts: null,
                unitsConsumed: Number(meta.computeUnitsConsumed()),
                returnData: meta.returnData() ? {
                    programId: new PublicKey(meta.returnData().programId()).toBase58(),
                    data: [Buffer.from(meta.returnData().data()).toString('base64'), 'base64'],
                } : null,
            },
        };
    }

    /**
     * Get epoch schedule
     */
    override async getEpochSchedule(): Promise<any> {
        const schedule = this.litesvm.getEpochSchedule();
        return {
            slotsPerEpoch: Number(schedule.slotsPerEpoch),
            leaderScheduleSlotOffset: Number(schedule.leaderScheduleSlotOffset),
            warmup: schedule.warmup,
            firstNormalEpoch: Number(schedule.firstNormalEpoch),
            firstNormalSlot: Number(schedule.firstNormalSlot),
        };
    }

    /**
     * Get latest blockhash from LiteSVM
     */
    override async getRecentBlockhash(): Promise<any> {
        const blockhash = this.litesvm.latestBlockhash();
        return {
            blockhash,
            feeCalculator: {
                lamportsPerSignature: 5000,
            },
        };
    }

    /**
     * Get latest blockhash (modern API)
     */
    override async getLatestBlockhash(commitment?: any): Promise<any> {
        const blockhash = this.litesvm.latestBlockhash();
        return {
            blockhash,
            lastValidBlockHeight: 1000000,
        };
    }

    /**
     * Confirm transaction (instant for LiteSVM)
     */
    override async confirmTransaction(
        signature: string | any,
        commitment?: any
    ): Promise<any> {
        return {
            context: { slot: 1 },
            value: { err: null },
        };
    }

    /**
     * Get signature statuses (return instant confirmation for LiteSVM)
     */
    override async getSignatureStatuses(
        signatures: string[],
        config?: any
    ): Promise<any> {
        return {
            context: { slot: 1 },
            value: signatures.map(() => ({
                slot: 1,
                confirmations: null,
                err: null,
                confirmationStatus: 'confirmed',
            })),
        };
    }

    /**
     * Get current slot from LiteSVM
     */
    override async getSlot(commitment?: any): Promise<number> {
        return Number(this.litesvm.getClock().slot);
    }

    /**
     * Confirm transaction is indexed (instant for LiteSVM as no indexer)
     */
    async confirmTransactionIndexed(_slot: number): Promise<boolean> {
        return true;
    }

    // All other methods (getValidityProof, getMultipleCompressedAccountProofs, etc.)
    // are inherited from TestRpc and work automatically!

    /**
     * Get the underlying LiteSVM instance for advanced operations
     */
    getLiteSVM(): LiteSVM {
        return this.litesvm;
    }

    /**
     * Warp to a specific slot (useful for testing time-dependent logic)
     */
    warpToSlot(slot: bigint): void {
        this.litesvm.warpToSlot(slot);
    }

    /**
     * Expire the current blockhash (forces new blockhash generation)
     */
    expireBlockhash(): void {
        this.litesvm.expireBlockhash();
    }
}

/**
 * Create a new LiteSVMRpc instance
 */
export async function createLiteSVMRpc(
    lightWasm: any,
    config?: LiteSVMConfig,
    proverEndpoint: string = 'http://127.0.0.1:3001'
): Promise<LiteSVMRpc> {
    return new LiteSVMRpc(lightWasm, config, proverEndpoint);
}
