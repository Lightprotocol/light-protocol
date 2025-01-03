import { Connection, ConnectionConfig, PublicKey } from '@solana/web3.js';
import BN from 'bn.js';
import {
    getCompressedAccountByHashTest,
    getCompressedAccountsByOwnerTest,
    getMultipleCompressedAccountsByHashTest,
} from './get-compressed-accounts';
import {
    getCompressedTokenAccountByHashTest,
    getCompressedTokenAccountsByDelegateTest,
    getCompressedTokenAccountsByOwnerTest,
} from './get-compressed-token-accounts';

import { MerkleTree } from '../merkle-tree/merkle-tree';
import { getParsedEvents } from './get-parsed-events';
import { defaultTestStateTreeAccounts } from '../../constants';
import {
    AddressWithTree,
    CompressedMintTokenHolders,
    CompressedTransaction,
    GetCompressedAccountsByOwnerConfig,
    PaginatedOptions,
    HashWithTree,
    LatestNonVotingSignatures,
    LatestNonVotingSignaturesPaginated,
    SignatureWithMetadata,
    WithContext,
    WithCursor,
    BaseRpc,
} from '../../rpc-interface';
import {
    CompressedProofWithContext,
    CompressionApiInterface,
    GetCompressedTokenAccountsByOwnerOrDelegateOptions,
    ParsedTokenAccount,
    TokenBalance,
} from '../../rpc-interface';
import {
    BN254,
    CompressedAccountWithMerkleContext,
    MerkleContextWithMerkleProof,
    PublicTransactionEvent,
    bn,
} from '../../state';
import { IndexedArray } from '../merkle-tree';
import {
    MerkleContextWithNewAddressProof,
    Rpc,
    convertMerkleProofsWithContextToHex,
    convertNonInclusionMerkleProofInputsToHex,
    proverRequest,
} from '../../rpc';
// import { ConnectionInterface } from '../../connection-interface';

export interface TestRpcConfig {
    /**
     * Address of the state tree to index. Default: public default test state
     * tree.
     */
    merkleTreeAddress?: PublicKey;
    /**
     * Nullifier queue associated with merkleTreeAddress
     */
    nullifierQueueAddress?: PublicKey;
    /**
     * Depth of state tree. Defaults to the public default test state tree depth
     */
    depth?: number;
    /**
     * Log proof generation time
     */
    log?: boolean;
    /**
     * Address of the address tree to index. Default: public default test
     * address tree.
     */
    addressTreeAddress?: PublicKey;
    /**
     * Address queue associated with addressTreeAddress
     */
    addressQueueAddress?: PublicKey;
}

export type ClientSubscriptionId = number;
export interface LightWasm {
    blakeHash(input: string | Uint8Array, hashLength: number): Uint8Array;
    poseidonHash(input: string[] | BN[]): Uint8Array;
    poseidonHashString(input: string[] | BN[]): string;
    poseidonHashBN(input: string[] | BN[]): BN;
}

/**
 * Returns a mock RPC instance for use in unit tests.
 *
 * @param lightWasm               Wasm hasher instance.
 * @param endpoint                RPC endpoint URL. Defaults to
 *                                'http://127.0.0.1:8899'.
 * @param proverEndpoint          Prover server endpoint URL. Defaults to
 *                                'http://localhost:3001'.
 * @param merkleTreeAddress       Address of the merkle tree to index. Defaults
 *                                to the public default test state tree.
 * @param nullifierQueueAddress   Optional address of the associated nullifier
 *                                queue.
 * @param depth                   Depth of the merkle tree.
 * @param log                     Log proof generation time.
 */
export async function getTestRpc(
    lightWasm: LightWasm,
    endpoint: string = 'http://127.0.0.1:8899',
    compressionApiEndpoint: string = 'http://127.0.0.1:8784',
    proverEndpoint: string = 'http://127.0.0.1:3001',
    merkleTreeAddress?: PublicKey,
    nullifierQueueAddress?: PublicKey,
    depth?: number,
    log = false,
) {
    const defaultAccounts = defaultTestStateTreeAccounts();

    return new TestRpc(
        endpoint,
        lightWasm,
        compressionApiEndpoint,
        proverEndpoint,
        undefined,
        {
            merkleTreeAddress: merkleTreeAddress || defaultAccounts.merkleTree,
            nullifierQueueAddress:
                nullifierQueueAddress || defaultAccounts.nullifierQueue,
            depth: depth || defaultAccounts.merkleTreeHeight,
            log,
        },
    );
}
/**
 * Simple mock rpc for unit tests that simulates the compression rpc interface.
 * Fetches, parses events and builds merkletree on-demand, i.e. it does not persist state.
 * Constraints:
 * - Can only index 1 merkletree
 * - Can only index up to 1000 transactions
 *
 * For advanced testing use photon: https://github.com/helius-labs/photon
 */
export class TestRpc extends Connection implements CompressionApiInterface {
    // connection: Connection;
    compressionApiEndpoint: string;
    proverEndpoint: string;
    merkleTreeAddress: PublicKey;
    nullifierQueueAddress: PublicKey;
    addressTreeAddress: PublicKey;
    addressQueueAddress: PublicKey;
    lightWasm: LightWasm;
    depth: number;
    log = false;

    /**
     * Establish a Compression-compatible JSON RPC mock-connection
     *
     * @param endpoint                  endpoint to the solana cluster (use for
     *                                  localnet only)
     * @param hasher                    light wasm hasher instance
     * @param compressionApiEndpoint    Endpoint to the compression server.
     * @param proverEndpoint            Endpoint to the prover server. defaults
     *                                  to endpoint
     * @param connectionConfig          Optional connection config
     * @param testRpcConfig             Config for the mock rpc
     */
    constructor(
        endpoint: string,
        hasher: LightWasm,
        compressionApiEndpoint: string,
        proverEndpoint: string,
        connectionConfig?: ConnectionConfig,
        testRpcConfig?: TestRpcConfig,
    ) {
        super(endpoint, connectionConfig || 'confirmed');

        // this.connection = new Connection(
        //     endpoint,
        //     connectionConfig || 'confirmed',
        // );
        this.compressionApiEndpoint = compressionApiEndpoint;
        this.proverEndpoint = proverEndpoint;

        const {
            merkleTreeAddress,
            nullifierQueueAddress,
            depth,
            log,
            addressTreeAddress,
            addressQueueAddress,
        } = testRpcConfig ?? {};

        const {
            merkleTree,
            nullifierQueue,
            merkleTreeHeight,
            addressQueue,
            addressTree,
        } = defaultTestStateTreeAccounts();

        this.lightWasm = hasher;
        this.merkleTreeAddress = merkleTreeAddress ?? merkleTree;
        this.nullifierQueueAddress = nullifierQueueAddress ?? nullifierQueue;
        this.addressTreeAddress = addressTreeAddress ?? addressTree;
        this.addressQueueAddress = addressQueueAddress ?? addressQueue;
        this.depth = depth ?? merkleTreeHeight;
        this.log = log ?? false;
    }

    // get commitment(): Commitment | undefined {
    //     return this.connection.commitment;
    // }

    // get rpcEndpoint(): string {
    //     return this.connection.rpcEndpoint;
    // }

    // // === Connection Methods Delegated ===

    // async getBalanceAndContext(
    //     publicKey: PublicKey,
    //     commitmentOrConfig?: Commitment | GetBalanceConfig,
    // ): Promise<RpcResponseAndContext<number>> {
    //     return this.connection.getBalanceAndContext(
    //         publicKey,
    //         commitmentOrConfig,
    //     );
    // }

    // async getBalance(
    //     publicKey: PublicKey,
    //     commitmentOrConfig?: Commitment | GetBalanceConfig,
    // ): Promise<number> {
    //     return this.connection.getBalance(publicKey, commitmentOrConfig);
    // }

    // async getBlockTime(slot: number): Promise<number | null> {
    //     return this.connection.getBlockTime(slot);
    // }

    // async getMinimumLedgerSlot(): Promise<number> {
    //     return this.connection.getMinimumLedgerSlot();
    // }

    // async getFirstAvailableBlock(): Promise<number> {
    //     return this.connection.getFirstAvailableBlock();
    // }

    // async getSupply(
    //     config?: GetSupplyConfig | Commitment,
    // ): Promise<RpcResponseAndContext<Supply>> {
    //     return this.connection.getSupply(config);
    // }

    // async getTokenSupply(
    //     tokenMintAddress: PublicKey,
    //     commitment?: Commitment,
    // ): Promise<RpcResponseAndContext<TokenAmount>> {
    //     return this.connection.getTokenSupply(tokenMintAddress, commitment);
    // }

    // async getTokenAccountBalance(
    //     tokenAddress: PublicKey,
    //     commitment?: Commitment,
    // ): Promise<RpcResponseAndContext<TokenAmount>> {
    //     return this.connection.getTokenAccountBalance(tokenAddress, commitment);
    // }

    // async getTokenAccountsByOwner(
    //     ownerAddress: PublicKey,
    //     filter: TokenAccountsFilter,
    //     commitmentOrConfig?: Commitment | GetTokenAccountsByOwnerConfig,
    // ): Promise<RpcResponseAndContext<GetProgramAccountsResponse>> {
    //     return this.connection.getTokenAccountsByOwner(
    //         ownerAddress,
    //         filter,
    //         commitmentOrConfig,
    //     );
    // }

    // async getParsedTokenAccountsByOwner(
    //     ownerAddress: PublicKey,
    //     filter: TokenAccountsFilter,
    //     commitment?: Commitment,
    // ): Promise<
    //     RpcResponseAndContext<
    //         Array<{
    //             pubkey: PublicKey;
    //             account: AccountInfo<ParsedAccountData>;
    //         }>
    //     >
    // > {
    //     return this.connection.getParsedTokenAccountsByOwner(
    //         ownerAddress,
    //         filter,
    //         commitment,
    //     );
    // }

    // async getLargestAccounts(
    //     config?: GetLargestAccountsConfig,
    // ): Promise<RpcResponseAndContext<Array<AccountBalancePair>>> {
    //     return this.connection.getLargestAccounts(config);
    // }

    // async getTokenLargestAccounts(
    //     mintAddress: PublicKey,
    //     commitment?: Commitment,
    // ): Promise<RpcResponseAndContext<Array<TokenAccountBalancePair>>> {
    //     return this.connection.getTokenLargestAccounts(mintAddress, commitment);
    // }

    // async getAccountInfoAndContext(
    //     publicKey: PublicKey,
    //     commitmentOrConfig?: Commitment | GetAccountInfoConfig,
    // ): Promise<RpcResponseAndContext<AccountInfo<Buffer> | null>> {
    //     return this.connection.getAccountInfoAndContext(
    //         publicKey,
    //         commitmentOrConfig,
    //     );
    // }

    // async getParsedAccountInfo(
    //     publicKey: PublicKey,
    //     commitmentOrConfig?: Commitment | GetAccountInfoConfig,
    // ): Promise<
    //     RpcResponseAndContext<AccountInfo<Buffer | ParsedAccountData> | null>
    // > {
    //     return this.connection.getParsedAccountInfo(
    //         publicKey,
    //         commitmentOrConfig,
    //     );
    // }

    // async getAccountInfo(
    //     publicKey: PublicKey,
    //     commitmentOrConfig?: Commitment | GetAccountInfoConfig,
    // ): Promise<AccountInfo<Buffer> | null> {
    //     return this.connection.getAccountInfo(publicKey, commitmentOrConfig);
    // }

    // async getMultipleParsedAccounts(
    //     publicKeys: PublicKey[],
    //     rawConfig?: GetMultipleAccountsConfig,
    // ): Promise<
    //     RpcResponseAndContext<
    //         (AccountInfo<Buffer | ParsedAccountData> | null)[]
    //     >
    // > {
    //     return this.connection.getMultipleParsedAccounts(publicKeys, rawConfig);
    // }

    // async getMultipleAccountsInfoAndContext(
    //     publicKeys: PublicKey[],
    //     commitmentOrConfig?: Commitment | GetMultipleAccountsConfig,
    // ): Promise<RpcResponseAndContext<(AccountInfo<Buffer> | null)[]>> {
    //     return this.connection.getMultipleAccountsInfoAndContext(
    //         publicKeys,
    //         commitmentOrConfig,
    //     );
    // }

    // async getMultipleAccountsInfo(
    //     publicKeys: PublicKey[],
    //     commitmentOrConfig?: Commitment | GetMultipleAccountsConfig,
    // ): Promise<(AccountInfo<Buffer> | null)[]> {
    //     return this.connection.getMultipleAccountsInfo(
    //         publicKeys,
    //         commitmentOrConfig,
    //     );
    // }

    // async getStakeActivation(
    //     publicKey: PublicKey,
    //     commitmentOrConfig?: Commitment | GetStakeActivationConfig,
    //     epoch?: number,
    // ): Promise<StakeActivationData> {
    //     return this.connection.getStakeActivation(
    //         publicKey,
    //         commitmentOrConfig,
    //         epoch,
    //     );
    // }

    // async getProgramAccounts(
    //     programId: PublicKey,
    //     configOrCommitment?: GetProgramAccountsConfig | Commitment,
    // ): Promise<GetProgramAccountsResponse>;

    // async getProgramAccounts(
    //     programId: PublicKey,
    //     configOrCommitment: GetProgramAccountsConfig & { withContext: true },
    // ): Promise<RpcResponseAndContext<GetProgramAccountsResponse>>;

    // async getProgramAccounts(
    //     programId: PublicKey,
    //     configOrCommitment?: GetProgramAccountsConfig | Commitment,
    // ): Promise<
    //     | GetProgramAccountsResponse
    //     | RpcResponseAndContext<GetProgramAccountsResponse>
    // > {
    //     return this.connection.getProgramAccounts(
    //         programId,
    //         configOrCommitment,
    //     );
    // }

    // async getParsedProgramAccounts(
    //     programId: PublicKey,
    //     configOrCommitment?: GetParsedProgramAccountsConfig | Commitment,
    // ): Promise<
    //     Array<{
    //         pubkey: PublicKey;
    //         account: AccountInfo<Buffer | ParsedAccountData>;
    //     }>
    // > {
    //     return this.connection.getParsedProgramAccounts(
    //         programId,
    //         configOrCommitment,
    //     );
    // }

    // // === Subscription Methods ===

    // onAccountChange(
    //     publicKey: PublicKey,
    //     callback: AccountChangeCallback,
    //     config?: AccountSubscriptionConfig,
    // ): ClientSubscriptionId {
    //     return this.connection.onAccountChange(publicKey, callback, config);
    // }

    // async removeAccountChangeListener(
    //     clientSubscriptionId: ClientSubscriptionId,
    // ): Promise<void> {
    //     return this.connection.removeAccountChangeListener(
    //         clientSubscriptionId,
    //     );
    // }

    // onProgramAccountChange(
    //     programId: PublicKey,
    //     callback: ProgramAccountChangeCallback,
    //     config?: ProgramAccountSubscriptionConfig,
    // ): ClientSubscriptionId {
    //     return this.connection.onProgramAccountChange(
    //         programId,
    //         callback,
    //         config,
    //     );
    // }

    // async removeProgramAccountChangeListener(
    //     clientSubscriptionId: ClientSubscriptionId,
    // ): Promise<void> {
    //     return this.connection.removeProgramAccountChangeListener(
    //         clientSubscriptionId,
    //     );
    // }

    // onLogs(
    //     filter: LogsFilter,
    //     callback: LogsCallback,
    //     commitment?: Commitment,
    // ): ClientSubscriptionId {
    //     return this.connection.onLogs(filter, callback, commitment);
    // }

    // async removeOnLogsListener(
    //     clientSubscriptionId: ClientSubscriptionId,
    // ): Promise<void> {
    //     return this.connection.removeOnLogsListener(clientSubscriptionId);
    // }

    // onSlotChange(callback: SlotChangeCallback): ClientSubscriptionId {
    //     return this.connection.onSlotChange(callback);
    // }

    // async removeSlotChangeListener(
    //     clientSubscriptionId: ClientSubscriptionId,
    // ): Promise<void> {
    //     return this.connection.removeSlotChangeListener(clientSubscriptionId);
    // }

    // onSlotUpdate(callback: SlotUpdateCallback): ClientSubscriptionId {
    //     return this.connection.onSlotUpdate(callback);
    // }

    // async removeSlotUpdateListener(
    //     clientSubscriptionId: ClientSubscriptionId,
    // ): Promise<void> {
    //     return this.connection.removeSlotUpdateListener(clientSubscriptionId);
    // }

    // onSignature(
    //     signature: TransactionSignature,
    //     callback: SignatureResultCallback,
    //     commitment?: Commitment,
    // ): ClientSubscriptionId {
    //     return this.connection.onSignature(signature, callback, commitment);
    // }

    // onSignatureWithOptions(
    //     signature: TransactionSignature,
    //     callback: SignatureSubscriptionCallback,
    //     options?: SignatureSubscriptionOptions,
    // ): ClientSubscriptionId {
    //     return this.connection.onSignatureWithOptions(
    //         signature,
    //         callback,
    //         options,
    //     );
    // }

    // async removeSignatureListener(
    //     clientSubscriptionId: ClientSubscriptionId,
    // ): Promise<void> {
    //     return this.connection.removeSignatureListener(clientSubscriptionId);
    // }

    // onRootChange(callback: RootChangeCallback): ClientSubscriptionId {
    //     return this.connection.onRootChange(callback);
    // }

    // async removeRootChangeListener(
    //     clientSubscriptionId: ClientSubscriptionId,
    // ): Promise<void> {
    //     return this.connection.removeRootChangeListener(clientSubscriptionId);
    // }

    // // === Transaction Methods ===

    // async sendTransaction(
    //     transaction: VersionedTransaction,
    //     options?: SendOptions,
    // ): Promise<TransactionSignature> {
    //     return this.connection.sendTransaction(transaction, options);
    // }

    // async sendRawTransaction(
    //     rawTransaction: Buffer | Uint8Array | Array<number>,
    //     options?: SendOptions,
    // ): Promise<TransactionSignature> {
    //     return this.connection.sendRawTransaction(rawTransaction, options);
    // }

    // async sendEncodedTransaction(
    //     encodedTransaction: string,
    //     options?: SendOptions,
    // ): Promise<TransactionSignature> {
    //     return this.connection.sendEncodedTransaction(
    //         encodedTransaction,
    //         options,
    //     );
    // }

    // async simulateTransaction(
    //     transaction: VersionedTransaction,
    //     config?: SimulateTransactionConfig,
    // ): Promise<RpcResponseAndContext<SimulatedTransactionResponse>> {
    //     return this.connection.simulateTransaction(transaction, config);
    // }

    // async requestAirdrop(
    //     to: PublicKey,
    //     lamports: number,
    // ): Promise<TransactionSignature> {
    //     return this.connection.requestAirdrop(to, lamports);
    // }

    // async getStakeMinimumDelegation(
    //     config?: GetStakeMinimumDelegationConfig,
    // ): Promise<RpcResponseAndContext<number>> {
    //     return this.connection.getStakeMinimumDelegation(config);
    // }

    // async getTransactions(
    //     signatures: TransactionSignature[],
    //     commitmentOrConfig?: GetTransactionConfig | Finality,
    // ): Promise<(VersionedTransactionResponse | null)[]> {
    //     return this.connection.getTransactions(signatures, commitmentOrConfig);
    // }

    // async getTransaction(
    //     signature: string,
    //     rawConfig?: GetTransactionConfig,
    // ): Promise<VersionedTransactionResponse | null> {
    //     return this.connection.getTransaction(signature, rawConfig);
    // }

    // async getParsedTransaction(
    //     signature: TransactionSignature,
    //     commitmentOrConfig?: GetVersionedTransactionConfig | Finality,
    // ): Promise<ParsedTransactionWithMeta | null> {
    //     return this.connection.getParsedTransaction(
    //         signature,
    //         commitmentOrConfig,
    //     );
    // }

    // async getParsedTransactions(
    //     signatures: TransactionSignature[],
    //     commitmentOrConfig?: GetVersionedTransactionConfig | Finality,
    // ): Promise<(ParsedTransactionWithMeta | null)[]> {
    //     return this.connection.getParsedTransactions(
    //         signatures,
    //         commitmentOrConfig,
    //     );
    // }

    // async getConfirmedBlock(
    //     slot: number,
    //     commitment?: Finality,
    // ): Promise<ConfirmedBlock> {
    //     return this.connection.getConfirmedBlock(slot, commitment);
    // }

    // async getBlocks(
    //     startSlot: number,
    //     endSlot?: number,
    //     commitment?: Finality,
    // ): Promise<Array<number>> {
    //     return this.connection.getBlocks(startSlot, endSlot, commitment);
    // }

    // async getBlockSignatures(
    //     slot: number,
    //     commitment?: Finality,
    // ): Promise<BlockSignatures> {
    //     return this.connection.getBlockSignatures(slot, commitment);
    // }

    // async getConfirmedBlockSignatures(
    //     slot: number,
    //     commitment?: Finality,
    // ): Promise<BlockSignatures> {
    //     return this.connection.getConfirmedBlockSignatures(slot, commitment);
    // }

    // confirmTransaction(
    //     strategy: TransactionConfirmationStrategy,
    //     commitment?: Commitment,
    // ): Promise<RpcResponseAndContext<SignatureResult>>;

    // /** @deprecated Instead, call `confirmTransaction` and pass in {@link TransactionConfirmationStrategy} */
    // // eslint-disable-next-line no-dupe-class-members
    // confirmTransaction(
    //     strategy: TransactionSignature,
    //     commitment?: Commitment,
    // ): Promise<RpcResponseAndContext<SignatureResult>>;

    // async confirmTransaction(
    //     strategy: TransactionConfirmationStrategy | TransactionSignature,
    //     commitment?: Commitment,
    // ): Promise<RpcResponseAndContext<SignatureResult>> {
    //     // @ts-ignore
    //     return this.connection.confirmTransaction(strategy, commitment);
    // }

    // async getClusterNodes(): Promise<Array<ContactInfo>> {
    //     return this.connection.getClusterNodes();
    // }

    // async getVoteAccounts(commitment?: Commitment): Promise<VoteAccountStatus> {
    //     return this.connection.getVoteAccounts(commitment);
    // }

    // async getSlot(
    //     commitmentOrConfig?: Commitment | GetSlotConfig,
    // ): Promise<number> {
    //     return this.connection.getSlot(commitmentOrConfig);
    // }

    // async getSlotLeader(
    //     commitmentOrConfig?: Commitment | GetSlotLeaderConfig,
    // ): Promise<string> {
    //     return this.connection.getSlotLeader(commitmentOrConfig);
    // }

    // async getSlotLeaders(
    //     startSlot: number,
    //     limit: number,
    // ): Promise<Array<PublicKey>> {
    //     return this.connection.getSlotLeaders(startSlot, limit);
    // }

    // async getSignatureStatus(
    //     signature: TransactionSignature,
    //     config?: SignatureStatusConfig,
    // ): Promise<RpcResponseAndContext<SignatureStatus | null>> {
    //     return this.connection.getSignatureStatus(signature, config);
    // }

    // async getSignatureStatuses(
    //     signatures: Array<TransactionSignature>,
    //     config?: SignatureStatusConfig,
    // ): Promise<RpcResponseAndContext<Array<SignatureStatus | null>>> {
    //     return this.connection.getSignatureStatuses(signatures, config);
    // }

    // async getTotalSupply(commitment?: Commitment): Promise<number> {
    //     return this.connection.getTotalSupply(commitment);
    // }

    // async getBlockHeight(config?: GetBlockHeightConfig): Promise<number> {
    //     return this.connection.getBlockHeight(config);
    // }

    // async getBlockProduction(
    //     configOrCommitment?: GetBlockProductionConfig | Commitment,
    // ): Promise<RpcResponseAndContext<BlockProduction>> {
    //     return this.connection.getBlockProduction(configOrCommitment);
    // }

    // async getTransactionCount(
    //     config?: GetTransactionCountConfig,
    // ): Promise<number> {
    //     return this.connection.getTransactionCount(config);
    // }

    // async getInflationGovernor(): Promise<InflationGovernor> {
    //     return this.connection.getInflationGovernor();
    // }

    // async getInflationReward(
    //     addresses: PublicKey[],
    //     epochs?: number,
    //     config?: GetInflationRewardConfig,
    // ): Promise<Array<InflationReward | null>> {
    //     return this.connection.getInflationReward(addresses, epochs, config);
    // }

    // async getInflationRate(): Promise<InflationRate> {
    //     return this.connection.getInflationRate();
    // }

    // async getEpochInfo(config?: GetEpochInfoConfig): Promise<EpochInfo> {
    //     return this.connection.getEpochInfo(config);
    // }

    // async getEpochSchedule(): Promise<EpochSchedule> {
    //     return this.connection.getEpochSchedule();
    // }

    // async getLeaderSchedule(): Promise<LeaderSchedule> {
    //     return this.connection.getLeaderSchedule();
    // }

    // async getRecentBlockhashAndContext(commitment?: Commitment): Promise<
    //     RpcResponseAndContext<{
    //         blockhash: Blockhash;
    //         feeCalculator: FeeCalculator;
    //     }>
    // > {
    //     return this.connection.getRecentBlockhashAndContext(commitment);
    // }

    // async getRecentPerformanceSamples(
    //     limit?: number,
    // ): Promise<Array<PerfSample>> {
    //     return this.connection.getRecentPerformanceSamples(limit);
    // }

    // async getFeeCalculatorForBlockhash(
    //     blockhash: Blockhash,
    //     commitment?: Commitment,
    // ): Promise<RpcResponseAndContext<FeeCalculator | null>> {
    //     return this.connection.getFeeCalculatorForBlockhash(
    //         blockhash,
    //         commitment,
    //     );
    // }

    // async getFeeForMessage(
    //     message: VersionedMessage,
    //     commitment?: Commitment,
    // ): Promise<RpcResponseAndContext<number | null>> {
    //     return this.connection.getFeeForMessage(message, commitment);
    // }

    // async getMinimumBalanceForRentExemption(
    //     dataLength: number,
    //     commitment?: Commitment,
    // ): Promise<number> {
    //     return this.connection.getMinimumBalanceForRentExemption(
    //         dataLength,
    //         commitment,
    //     );
    // }

    // async getRecentBlockhash(
    //     commitment?: Commitment,
    // ): Promise<{ blockhash: Blockhash; feeCalculator: FeeCalculator }> {
    //     return this.connection.getRecentBlockhash(commitment);
    // }

    // async getGenesisHash(): Promise<string> {
    //     return this.connection.getGenesisHash();
    // }
    // async getBlock(
    //     slot: number,
    //     rawConfig?: GetVersionedBlockConfig,
    // ): Promise<VersionedBlockResponse | null>;
    // async getBlock(
    //     slot: number,
    //     rawConfig: GetVersionedBlockConfig & { transactionDetails: 'accounts' },
    // ): Promise<VersionedAccountsModeBlockResponse | null>;
    // async getBlock(
    //     slot: number,
    //     rawConfig: GetVersionedBlockConfig & { transactionDetails: 'none' },
    // ): Promise<VersionedNoneModeBlockResponse | null>;
    // async getBlock(
    //     slot: number,
    //     rawConfig?: GetVersionedBlockConfig,
    // ): Promise<
    //     | VersionedBlockResponse
    //     | VersionedAccountsModeBlockResponse
    //     | VersionedNoneModeBlockResponse
    //     | null
    // > {
    //     return this.connection.getBlock(slot, rawConfig);
    // }

    // async getParsedBlock(
    //     slot: number,
    //     rawConfig?: GetVersionedBlockConfig,
    // ): Promise<ParsedAccountsModeBlockResponse>;
    // async getParsedBlock(
    //     slot: number,
    //     rawConfig: GetVersionedBlockConfig & { transactionDetails: 'accounts' },
    // ): Promise<ParsedAccountsModeBlockResponse>;
    // async getParsedBlock(
    //     slot: number,
    //     rawConfig: GetVersionedBlockConfig & { transactionDetails: 'none' },
    // ): Promise<ParsedNoneModeBlockResponse>;
    // async getParsedBlock(
    //     slot: number,
    //     rawConfig?: GetVersionedBlockConfig,
    // ): Promise<
    //     ParsedAccountsModeBlockResponse | ParsedNoneModeBlockResponse | null
    // > {
    //     return this.connection.getParsedBlock(slot, rawConfig);
    // }

    // async getConfirmedTransaction(
    //     signature: TransactionSignature,
    //     commitment?: Finality,
    // ): Promise<ConfirmedTransaction | null> {
    //     return this.connection.getConfirmedTransaction(signature, commitment);
    // }

    // async getParsedConfirmedTransaction(
    //     signature: TransactionSignature,
    //     commitment?: Finality,
    // ): Promise<ParsedConfirmedTransaction | null> {
    //     return this.connection.getParsedConfirmedTransaction(
    //         signature,
    //         commitment,
    //     );
    // }

    // async getParsedConfirmedTransactions(
    //     signatures: TransactionSignature[],
    //     commitment?: Finality,
    // ): Promise<(ParsedConfirmedTransaction | null)[]> {
    //     return this.connection.getParsedConfirmedTransactions(
    //         signatures,
    //         commitment,
    //     );
    // }

    // async getConfirmedSignaturesForAddress(
    //     address: PublicKey,
    //     startSlot: number,
    //     endSlot: number,
    // ): Promise<Array<TransactionSignature>> {
    //     return this.connection.getConfirmedSignaturesForAddress(
    //         address,
    //         startSlot,
    //         endSlot,
    //     );
    // }

    // async getConfirmedSignaturesForAddress2(
    //     address: PublicKey,
    //     options?: ConfirmedSignaturesForAddress2Options,
    //     commitment?: Finality,
    // ): Promise<Array<ConfirmedSignatureInfo>> {
    //     return this.connection.getConfirmedSignaturesForAddress2(
    //         address,
    //         options,
    //         commitment,
    //     );
    // }

    // async getSignaturesForAddress(
    //     address: PublicKey,
    //     options?: SignaturesForAddressOptions,
    //     commitment?: Finality,
    // ): Promise<Array<ConfirmedSignatureInfo>> {
    //     return this.connection.getSignaturesForAddress(
    //         address,
    //         options,
    //         commitment,
    //     );
    // }

    // async getRecentPrioritizationFees(
    //     config?: GetRecentPrioritizationFeesConfig,
    // ): Promise<RecentPrioritizationFees[]> {
    //     return this.connection.getRecentPrioritizationFees(config);
    // }

    // async getLatestBlockhash(
    //     config?: GetLatestBlockhashConfig,
    // ): Promise<BlockhashWithExpiryBlockHeight> {
    //     return this.connection.getLatestBlockhash(config);
    // }
    // async getLatestBlockhashAndContext(
    //     commitmentOrConfig?: Commitment | GetLatestBlockhashConfig,
    // ): Promise<RpcResponseAndContext<BlockhashWithExpiryBlockHeight>> {
    //     return this.connection.getLatestBlockhashAndContext(commitmentOrConfig);
    // }

    // async isBlockhashValid(
    //     blockhash: Blockhash,
    //     config?: IsBlockhashValidConfig,
    // ): Promise<RpcResponseAndContext<boolean>> {
    //     return this.connection.isBlockhashValid(blockhash, config);
    // }

    // async getVersion(): Promise<Version> {
    //     return this.connection.getVersion();
    // }

    // async getAddressLookupTable(
    //     accountKey: PublicKey,
    //     config?: GetAccountInfoConfig,
    // ): Promise<RpcResponseAndContext<AddressLookupTableAccount | null>> {
    //     return this.connection.getAddressLookupTable(accountKey, config);
    // }

    // async getNonceAndContext(
    //     nonceAccount: PublicKey,
    //     commitmentOrConfig?: Commitment | GetNonceAndContextConfig,
    // ): Promise<RpcResponseAndContext<NonceAccount | null>> {
    //     return this.connection.getNonceAndContext(
    //         nonceAccount,
    //         commitmentOrConfig,
    //     );
    // }

    // async getNonce(
    //     nonceAccount: PublicKey,
    //     commitmentOrConfig?: Commitment | GetNonceConfig,
    // ): Promise<NonceAccount | null> {
    //     return this.connection.getNonce(nonceAccount, commitmentOrConfig);
    // }

    // _buildArgs(
    //     args: Array<any>,
    //     override?: Commitment,
    //     encoding?: 'jsonParsed' | 'base64',
    //     extra?: any,
    // ): Array<any> {
    //     const commitment = override || this.connection.commitment;
    //     if (commitment || encoding || extra) {
    //         let options: any = {};
    //         if (encoding) {
    //             options.encoding = encoding;
    //         }
    //         if (commitment) {
    //             options.commitment = commitment;
    //         }
    //         if (extra) {
    //             options = Object.assign(options, extra);
    //         }
    //         args.push(options);
    //     }
    //     return args;
    // }

    // private async getCancellationPromise() {
    //     throw new Error(
    //         'getCancellationPromise not supported in rpc. it is a stub that is marked as private in web3.js Connection',
    //     );
    // }
    // private async getTransactionConfirmationPromise() {
    //     throw new Error(
    //         'getTransactionConfirmationPromise not supported in rpc. it is a stub that is marked as private in web3.js Connection',
    //     );
    // }
    // private async confirmTransactionUsingBlockHeightExceedanceStrategy() {
    //     throw new Error(
    //         'confirmTransactionUsingBlockHeightExceedanceStrategy not supported in rpc. it is a stub that is marked as private in web3.js Connection',
    //     );
    // }
    // async confirmTransactionUsingDurableNonceStrategy() {
    //     throw new Error(
    //         'confirmTransactionUsingDurableNonceStrategy not supported in rpc. it is a stub that is marked as private in web3.js Connection',
    //     );
    // }
    // private async confirmTransactionUsingLegacyTimeoutStrategy({
    //     commitment,
    //     signature,
    // }: {
    //     commitment?: Commitment;
    //     signature: string;
    // }): Promise<RpcResponseAndContext<SignatureResult>> {
    //     throw new Error(
    //         'confirmTransactionUsingLegacyTimeoutStrategy not supported in rpc. it is a stub that is marked as private in web3.js Connection',
    //     );
    // }

    /**
     * Fetch the compressed account for the specified account hash
     */
    async getCompressedAccount(
        address?: BN254,
        hash?: BN254,
    ): Promise<CompressedAccountWithMerkleContext | null> {
        if (address) {
            throw new Error('address is not supported in test-rpc');
        }
        if (!hash) {
            throw new Error('hash is required');
        }
        // @ts-ignore
        const account = await getCompressedAccountByHashTest(this, hash);
        return account ?? null;
    }

    /**
     * Fetch the compressed balance for the specified account hash
     */
    async getCompressedBalance(address?: BN254, hash?: BN254): Promise<BN> {
        if (address) {
            throw new Error('address is not supported in test-rpc');
        }
        if (!hash) {
            throw new Error('hash is required');
        }
        // @ts-ignore
        const account = await getCompressedAccountByHashTest(this, hash);
        if (!account) {
            throw new Error('Account not found');
        }
        return bn(account.lamports);
    }

    /**
     * Fetch the total compressed balance for the specified owner public key
     */
    async getCompressedBalanceByOwner(owner: PublicKey): Promise<BN> {
        const accounts = await this.getCompressedAccountsByOwner(owner);
        return accounts.items.reduce(
            (acc, account) => acc.add(account.lamports),
            bn(0),
        );
    }

    /**
     * Fetch the latest merkle proof for the specified account hash from the
     * cluster
     */
    async getCompressedAccountProof(
        hash: BN254,
    ): Promise<MerkleContextWithMerkleProof> {
        const proofs = await this.getMultipleCompressedAccountProofs([hash]);
        return proofs[0];
    }

    /**
     * Fetch all the account info for multiple compressed accounts specified by
     * an array of account hashes
     */
    async getMultipleCompressedAccounts(
        hashes: BN254[],
    ): Promise<CompressedAccountWithMerkleContext[]> {
        // @ts-ignore
        return await getMultipleCompressedAccountsByHashTest(this, hashes);
    }
    /**
     * Ensure that the Compression Indexer has already indexed the transaction
     */
    async confirmTransactionIndexed(_slot: number): Promise<boolean> {
        return true;
    }
    /**
     * Fetch the latest merkle proofs for multiple compressed accounts specified
     * by an array account hashes
     */
    async getMultipleCompressedAccountProofs(
        hashes: BN254[],
    ): Promise<MerkleContextWithMerkleProof[]> {
        /// Build tree
        const events: PublicTransactionEvent[] = await getParsedEvents(
            // @ts-ignore
            this,
        ).then(events => events.reverse());
        const allLeaves: number[][] = [];
        const allLeafIndices: number[] = [];
        for (const event of events) {
            for (
                let index = 0;
                index < event.outputCompressedAccounts.length;
                index++
            ) {
                const hash = event.outputCompressedAccountHashes[index];

                allLeaves.push(hash);
                allLeafIndices.push(event.outputLeafIndices[index]);
            }
        }
        const tree = new MerkleTree(
            this.depth,
            this.lightWasm,
            allLeaves.map(leaf => bn(leaf).toString()),
        );

        /// create merkle proofs and assemble return type
        const merkleProofs: MerkleContextWithMerkleProof[] = [];

        for (let i = 0; i < hashes.length; i++) {
            const leafIndex = tree.indexOf(hashes[i].toString());
            const pathElements = tree.path(leafIndex).pathElements;
            const bnPathElements = pathElements.map(value => bn(value));
            const root = bn(tree.root());
            const merkleProof: MerkleContextWithMerkleProof = {
                hash: hashes[i].toArray('be', 32),
                merkleTree: this.merkleTreeAddress,
                leafIndex: leafIndex,
                merkleProof: bnPathElements,
                nullifierQueue: this.nullifierQueueAddress,
                rootIndex: allLeaves.length,
                root: root,
            };
            merkleProofs.push(merkleProof);
        }

        /// Validate
        merkleProofs.forEach((proof, index) => {
            const leafIndex = proof.leafIndex;
            const computedHash = tree.elements()[leafIndex];
            const hashArr = bn(computedHash).toArray('be', 32);
            if (!hashArr.every((val, index) => val === proof.hash[index])) {
                throw new Error(
                    `Mismatch at index ${index}: expected ${proof.hash.toString()}, got ${hashArr.toString()}`,
                );
            }
        });

        return merkleProofs;
    }

    /**
     * Fetch all the compressed accounts owned by the specified public key.
     * Owner can be a program or user account
     */
    async getCompressedAccountsByOwner(
        owner: PublicKey,
        _config?: GetCompressedAccountsByOwnerConfig,
    ): Promise<WithCursor<CompressedAccountWithMerkleContext[]>> {
        // @ts-ignore
        const accounts = await getCompressedAccountsByOwnerTest(this, owner);
        return {
            items: accounts,
            cursor: null,
        };
    }

    /**
     * Fetch the latest compression signatures on the cluster. Results are
     * paginated.
     */
    async getLatestCompressionSignatures(
        _cursor?: string,
        _limit?: number,
    ): Promise<LatestNonVotingSignaturesPaginated> {
        throw new Error(
            'getLatestNonVotingSignaturesWithContext not supported in test-rpc',
        );
    }
    /**
     * Fetch the latest non-voting signatures on the cluster. Results are
     * not paginated.
     */
    async getLatestNonVotingSignatures(
        _limit?: number,
    ): Promise<LatestNonVotingSignatures> {
        throw new Error(
            'getLatestNonVotingSignaturesWithContext not supported in test-rpc',
        );
    }
    /**
     * Fetch all the compressed token accounts owned by the specified public
     * key. Owner can be a program or user account
     */
    async getCompressedTokenAccountsByOwner(
        owner: PublicKey,
        options: GetCompressedTokenAccountsByOwnerOrDelegateOptions,
    ): Promise<WithCursor<ParsedTokenAccount[]>> {
        return await getCompressedTokenAccountsByOwnerTest(
            // @ts-ignore
            this,
            owner,
            options!.mint!,
        );
    }

    /**
     * Fetch all the compressed accounts delegated to the specified public key.
     */
    async getCompressedTokenAccountsByDelegate(
        delegate: PublicKey,
        options: GetCompressedTokenAccountsByOwnerOrDelegateOptions,
    ): Promise<WithCursor<ParsedTokenAccount[]>> {
        return await getCompressedTokenAccountsByDelegateTest(
            // @ts-ignore
            this,
            delegate,
            options.mint!,
        );
    }

    /**
     * Fetch the compressed token balance for the specified account hash
     */
    async getCompressedTokenAccountBalance(
        hash: BN254,
    ): Promise<{ amount: BN }> {
        // @ts-ignore
        const account = await getCompressedTokenAccountByHashTest(this, hash);
        return { amount: bn(account.parsed.amount) };
    }

    /**
     * @deprecated use {@link getCompressedTokenBalancesByOwnerV2}.
     * Fetch all the compressed token balances owned by the specified public
     * key. Can filter by mint.
     */
    async getCompressedTokenBalancesByOwner(
        publicKey: PublicKey,
        options: GetCompressedTokenAccountsByOwnerOrDelegateOptions,
    ): Promise<WithCursor<{ balance: BN; mint: PublicKey }[]>> {
        const accounts = await getCompressedTokenAccountsByOwnerTest(
            // @ts-ignore
            this,
            publicKey,
            options.mint!,
        );
        return {
            items: accounts.items.map(account => ({
                balance: bn(account.parsed.amount),
                mint: account.parsed.mint,
            })),
            cursor: null,
        };
    }

    /**
     * Fetch all the compressed token balances owned by the specified public
     * key. Can filter by mint. Uses context.
     */
    async getCompressedTokenBalancesByOwnerV2(
        publicKey: PublicKey,
        options: GetCompressedTokenAccountsByOwnerOrDelegateOptions,
    ): Promise<WithContext<WithCursor<TokenBalance[]>>> {
        const accounts = await getCompressedTokenAccountsByOwnerTest(
            // @ts-ignore
            this,
            publicKey,
            options.mint!,
        );
        return {
            context: { slot: 1 },
            value: {
                items: accounts.items.map(account => ({
                    balance: bn(account.parsed.amount),
                    mint: account.parsed.mint,
                })),
                cursor: null,
            },
        };
    }

    /**
     * Returns confirmed signatures for transactions involving the specified
     * account hash forward in time from genesis to the most recent confirmed
     * block
     *
     * @param hash queried account hash
     */
    async getCompressionSignaturesForAccount(
        _hash: BN254,
    ): Promise<SignatureWithMetadata[]> {
        throw new Error(
            'getCompressionSignaturesForAccount not implemented in test-rpc',
        );
    }

    /**
     * Fetch a confirmed or finalized transaction from the cluster. Return with
     * CompressionInfo
     */
    async getTransactionWithCompressionInfo(
        _signature: string,
    ): Promise<CompressedTransaction | null> {
        throw new Error('getCompressedTransaction not implemented in test-rpc');
    }

    /**
     * Returns confirmed signatures for transactions involving the specified
     * address forward in time from genesis to the most recent confirmed
     * block
     *
     * @param address queried compressed account address
     */
    async getCompressionSignaturesForAddress(
        _address: PublicKey,
        _options?: PaginatedOptions,
    ): Promise<WithCursor<SignatureWithMetadata[]>> {
        throw new Error('getSignaturesForAddress3 not implemented');
    }

    /**
     * Returns confirmed signatures for compression transactions involving the
     * specified account owner forward in time from genesis to the
     * most recent confirmed block
     *
     * @param owner queried owner public key
     */
    async getCompressionSignaturesForOwner(
        _owner: PublicKey,
        _options?: PaginatedOptions,
    ): Promise<WithCursor<SignatureWithMetadata[]>> {
        throw new Error('getSignaturesForOwner not implemented');
    }

    /**
     * Returns confirmed signatures for compression transactions involving the
     * specified token account owner forward in time from genesis to the most
     * recent confirmed block
     */
    async getCompressionSignaturesForTokenOwner(
        _owner: PublicKey,
        _options?: PaginatedOptions,
    ): Promise<WithCursor<SignatureWithMetadata[]>> {
        throw new Error('getSignaturesForTokenOwner not implemented');
    }

    /**
     * Fetch the current indexer health status
     */
    async getIndexerHealth(): Promise<string> {
        return 'ok';
    }

    /**
     * Fetch the current slot that the node is processing
     */
    async getIndexerSlot(): Promise<number> {
        return 1;
    }

    /**
     * Fetch the latest address proofs for new unique addresses specified by an
     * array of addresses.
     *
     * the proof states that said address have not yet been created in respective address tree.
     * @param addresses Array of BN254 new addresses
     * @returns Array of validity proofs for new addresses
     */
    async getMultipleNewAddressProofs(addresses: BN254[]) {
        /// Build tree
        const indexedArray = IndexedArray.default();
        const allAddresses: BN[] = [];
        indexedArray.init();
        const hashes: BN[] = [];
        // TODO(crank): add support for cranked address tree in 'allAddresses'.
        // The Merkle tree root doesnt actually advance beyond init() unless we
        // start emptying the address queue.
        for (let i = 0; i < allAddresses.length; i++) {
            indexedArray.append(bn(allAddresses[i]));
        }
        for (let i = 0; i < indexedArray.elements.length; i++) {
            const hash = indexedArray.hashElement(this.lightWasm, i);
            hashes.push(bn(hash!));
        }
        const tree = new MerkleTree(
            this.depth,
            this.lightWasm,
            hashes.map(hash => bn(hash).toString()),
        );

        /// Creates proof for each address
        const newAddressProofs: MerkleContextWithNewAddressProof[] = [];

        for (let i = 0; i < addresses.length; i++) {
            const [lowElement] = indexedArray.findLowElement(addresses[i]);
            if (!lowElement) throw new Error('Address not found');

            const leafIndex = lowElement.index;

            const pathElements: string[] = tree.path(leafIndex).pathElements;
            const bnPathElements = pathElements.map(value => bn(value));

            const higherRangeValue = indexedArray.get(
                lowElement.nextIndex,
            )!.value;
            const root = bn(tree.root());

            const proof: MerkleContextWithNewAddressProof = {
                root,
                rootIndex: 3,
                value: addresses[i],
                leafLowerRangeValue: lowElement.value,
                leafHigherRangeValue: higherRangeValue,
                nextIndex: bn(lowElement.nextIndex),
                merkleProofHashedIndexedElementLeaf: bnPathElements,
                indexHashedIndexedElementLeaf: bn(lowElement.index),
                merkleTree: this.addressTreeAddress,
                nullifierQueue: this.addressQueueAddress,
            };
            newAddressProofs.push(proof);
        }
        return newAddressProofs;
    }

    async getCompressedMintTokenHolders(
        _mint: PublicKey,
        _options?: PaginatedOptions,
    ): Promise<WithContext<WithCursor<CompressedMintTokenHolders[]>>> {
        throw new Error(
            'getCompressedMintTokenHolders not implemented in test-rpc',
        );
    }

    /**
     * Advanced usage of getValidityProof: fetches ZKP directly from a custom
     * non-rpcprover. Note: This uses the proverEndpoint specified in the
     * constructor. For normal usage, please use {@link getValidityProof}
     * instead.
     *
     * Note: Use RPC class for forested trees. TestRpc is only for custom
     * testing purposes.
     */
    async getValidityProofDirect(
        hashes: BN254[] = [],
        newAddresses: BN254[] = [],
    ): Promise<CompressedProofWithContext> {
        return this.getValidityProof(hashes, newAddresses);
    }
    /**
     * @deprecated This method is not available for TestRpc. Please use
     * {@link getValidityProof} instead.
     */
    async getValidityProofAndRpcContext(
        hashes: HashWithTree[] = [],
        newAddresses: AddressWithTree[] = [],
    ): Promise<WithContext<CompressedProofWithContext>> {
        if (newAddresses.some(address => !(address instanceof BN))) {
            throw new Error('AddressWithTree is not supported in test-rpc');
        }
        return {
            value: await this.getValidityProofV0(hashes, newAddresses),
            context: { slot: 1 },
        };
    }
    /**
     * Fetch the latest validity proof for (1) compressed accounts specified by
     * an array of account hashes. (2) new unique addresses specified by an
     * array of addresses.
     *
     * Validity proofs prove the presence of compressed accounts in state trees
     * and the non-existence of addresses in address trees, respectively. They
     * enable verification without recomputing the merkle proof path, thus
     * lowering verification and data costs.
     *
     * @param hashes        Array of BN254 hashes.
     * @param newAddresses  Array of BN254 new addresses.
     * @returns             validity proof with context
     */
    async getValidityProof(
        hashes: BN254[] = [],
        newAddresses: BN254[] = [],
    ): Promise<CompressedProofWithContext> {
        if (newAddresses.some(address => !(address instanceof BN))) {
            throw new Error('AddressWithTree is not supported in test-rpc');
        }
        let validityProof: CompressedProofWithContext;

        if (hashes.length === 0 && newAddresses.length === 0) {
            throw new Error(
                'Empty input. Provide hashes and/or new addresses.',
            );
        } else if (hashes.length > 0 && newAddresses.length === 0) {
            /// inclusion
            const merkleProofsWithContext =
                await this.getMultipleCompressedAccountProofs(hashes);
            const inputs = convertMerkleProofsWithContextToHex(
                merkleProofsWithContext,
            );

            // TODO: reactivate to handle proofs of height 32
            // const publicInputHash = getPublicInputHash(
            //     merkleProofsWithContext,
            //     hashes,
            //     [],
            //     this.lightWasm,
            // );

            const compressedProof = await proverRequest(
                this.proverEndpoint,
                'inclusion',
                inputs,
                this.log,
                // publicInputHash,
            );
            validityProof = {
                compressedProof,
                roots: merkleProofsWithContext.map(proof => proof.root),
                rootIndices: merkleProofsWithContext.map(
                    proof => proof.rootIndex,
                ),
                leafIndices: merkleProofsWithContext.map(
                    proof => proof.leafIndex,
                ),
                leaves: merkleProofsWithContext.map(proof => bn(proof.hash)),
                merkleTrees: merkleProofsWithContext.map(
                    proof => proof.merkleTree,
                ),
                nullifierQueues: merkleProofsWithContext.map(
                    proof => proof.nullifierQueue,
                ),
            };
        } else if (hashes.length === 0 && newAddresses.length > 0) {
            /// new-address
            const newAddressProofs: MerkleContextWithNewAddressProof[] =
                await this.getMultipleNewAddressProofs(newAddresses);

            const inputs =
                convertNonInclusionMerkleProofInputsToHex(newAddressProofs);
            // const publicInputHash = getPublicInputHash(
            //     [],
            //     [],
            //     newAddressProofs,
            //     this.lightWasm,
            // );
            const compressedProof = await proverRequest(
                this.proverEndpoint,
                'new-address',
                inputs,
                this.log,
                // publicInputHash,
            );

            validityProof = {
                compressedProof,
                roots: newAddressProofs.map(proof => proof.root),
                // TODO(crank): make dynamic to enable forester support in
                // test-rpc.ts. Currently this is a static root because the
                // address tree doesn't advance.
                rootIndices: newAddressProofs.map(_ => 3),
                leafIndices: newAddressProofs.map(
                    proof => proof.indexHashedIndexedElementLeaf.toNumber(), // TODO: support >32bit
                ),
                leaves: newAddressProofs.map(proof => bn(proof.value)),
                merkleTrees: newAddressProofs.map(proof => proof.merkleTree),
                nullifierQueues: newAddressProofs.map(
                    proof => proof.nullifierQueue,
                ),
            };
        } else if (hashes.length > 0 && newAddresses.length > 0) {
            /// combined
            const merkleProofsWithContext =
                await this.getMultipleCompressedAccountProofs(hashes);
            const inputs = convertMerkleProofsWithContextToHex(
                merkleProofsWithContext,
            );
            const newAddressProofs: MerkleContextWithNewAddressProof[] =
                await this.getMultipleNewAddressProofs(newAddresses);

            const newAddressInputs =
                convertNonInclusionMerkleProofInputsToHex(newAddressProofs);
            // const publicInputHash = getPublicInputHash(
            //     merkleProofsWithContext,
            //     hashes,
            //     newAddressProofs,
            //     this.lightWasm,
            // );
            const compressedProof = await proverRequest(
                this.proverEndpoint,
                'combined',
                [inputs, newAddressInputs],
                this.log,
                // publicInputHash,
            );

            validityProof = {
                compressedProof,
                roots: merkleProofsWithContext
                    .map(proof => proof.root)
                    .concat(newAddressProofs.map(proof => proof.root)),
                rootIndices: merkleProofsWithContext
                    .map(proof => proof.rootIndex)
                    // TODO(crank): make dynamic to enable forester support in
                    // test-rpc.ts. Currently this is a static root because the
                    // address tree doesn't advance.
                    .concat(newAddressProofs.map(_ => 3)),
                leafIndices: merkleProofsWithContext
                    .map(proof => proof.leafIndex)
                    .concat(
                        newAddressProofs.map(
                            proof =>
                                proof.indexHashedIndexedElementLeaf.toNumber(), // TODO: support >32bit
                        ),
                    ),
                leaves: merkleProofsWithContext
                    .map(proof => bn(proof.hash))
                    .concat(newAddressProofs.map(proof => bn(proof.value))),
                merkleTrees: merkleProofsWithContext
                    .map(proof => proof.merkleTree)
                    .concat(newAddressProofs.map(proof => proof.merkleTree)),
                nullifierQueues: merkleProofsWithContext
                    .map(proof => proof.nullifierQueue)
                    .concat(
                        newAddressProofs.map(proof => proof.nullifierQueue),
                    ),
            };
        } else throw new Error('Invalid input');

        return validityProof;
    }

    async getValidityProofV0(
        hashes: HashWithTree[] = [],
        newAddresses: AddressWithTree[] = [],
    ): Promise<CompressedProofWithContext> {
        /// TODO(swen): add support for custom trees
        return this.getValidityProof(
            hashes.map(hash => hash.hash),
            newAddresses.map(address => address.address),
        );
    }
}
