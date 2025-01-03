// import {
//     PublicKey,
//     Commitment,
//     ConnectionConfig,
//     GetBalanceConfig,
//     RpcResponseAndContext,
//     Supply,
//     TokenAmount,
//     GetSupplyConfig,
//     TokenAccountsFilter,
//     GetProgramAccountsResponse,
//     GetTokenAccountsByOwnerConfig,
//     AccountInfo,
//     ParsedAccountData,
//     GetLargestAccountsConfig,
//     AccountBalancePair,
//     TokenAccountBalancePair,
//     GetAccountInfoConfig,
//     GetMultipleAccountsConfig,
//     StakeActivationData,
//     GetBlockHeightConfig,
//     GetBlockProductionConfig,
//     BlockProduction,
//     GetLatestBlockhashConfig,
//     BlockhashWithExpiryBlockHeight,
//     SimulatedTransactionResponse,
//     SimulateTransactionConfig,
//     SendOptions,
//     TransactionSignature,
//     AddressLookupTableAccount,
//     GetParsedProgramAccountsConfig,
//     Finality,
//     GetVersionedBlockConfig,
//     VersionedBlockResponse,
//     VersionedAccountsModeBlockResponse,
//     VersionedNoneModeBlockResponse,
//     ParsedAccountsModeBlockResponse,
//     ParsedNoneModeBlockResponse,
//     ParsedTransactionWithMeta,
//     VersionedTransaction,
//     VersionedTransactionResponse,
//     ConfirmedBlock,
//     ConfirmedSignatureInfo,
//     ConfirmedSignaturesForAddress2Options,
//     SignatureStatusConfig,
//     SignatureStatus,
//     Version,
//     VoteAccountStatus,
//     GetSlotConfig,
//     GetSlotLeaderConfig,
//     GetProgramAccountsConfig,
//     SignatureResult,
//     TransactionConfirmationStrategy,
//     AccountChangeCallback,
//     ProgramAccountChangeCallback,
//     LogsCallback,
//     SlotChangeCallback,
//     SlotUpdateCallback,
//     SignatureResultCallback,
//     SignatureSubscriptionCallback,
//     SignatureSubscriptionOptions,
//     RootChangeCallback,
//     GetStakeActivationConfig,
//     InflationGovernor,
//     GetTransactionCountConfig,
//     GetInflationRewardConfig,
//     InflationReward,
//     InflationRate,
//     GetEpochInfoConfig,
//     EpochInfo,
//     EpochSchedule,
//     LeaderSchedule,
//     Blockhash,
//     FeeCalculator,
//     PerfSample,
//     VersionedMessage,
//     GetRecentPrioritizationFeesConfig,
//     RecentPrioritizationFees,
//     IsBlockhashValidConfig,
//     GetVersionedTransactionConfig,
//     ParsedConfirmedTransaction,
//     ConfirmedTransaction,
//     BlockSignatures,
//     SignaturesForAddressOptions,
//     GetNonceAndContextConfig,
//     NonceAccount,
//     GetNonceConfig,
//     GetStakeMinimumDelegationConfig,
//     AccountSubscriptionConfig,
//     ProgramAccountSubscriptionConfig,
//     LogsFilter,
//     ContactInfo,
// } from '@solana/web3.js';

// export type ClientSubscriptionId = number;

// export interface ConnectionInterface {
//     readonly commitment?: Commitment;
//     readonly rpcEndpoint: string;

//     getBalanceAndContext(
//         publicKey: PublicKey,
//         commitmentOrConfig?: Commitment | GetBalanceConfig,
//     ): Promise<RpcResponseAndContext<number>>;

//     getBalance(
//         publicKey: PublicKey,
//         commitmentOrConfig?: Commitment | GetBalanceConfig,
//     ): Promise<number>;

//     getBlockTime(slot: number): Promise<number | null>;

//     getMinimumLedgerSlot(): Promise<number>;

//     getFirstAvailableBlock(): Promise<number>;

//     getSupply(
//         config?: GetSupplyConfig | Commitment,
//     ): Promise<RpcResponseAndContext<Supply>>;

//     getTokenSupply(
//         tokenMintAddress: PublicKey,
//         commitment?: Commitment,
//     ): Promise<RpcResponseAndContext<TokenAmount>>;

//     getTokenAccountBalance(
//         tokenAddress: PublicKey,
//         commitment?: Commitment,
//     ): Promise<RpcResponseAndContext<TokenAmount>>;

//     getTokenAccountsByOwner(
//         ownerAddress: PublicKey,
//         filter: TokenAccountsFilter,
//         commitmentOrConfig?: Commitment | GetTokenAccountsByOwnerConfig,
//     ): Promise<RpcResponseAndContext<GetProgramAccountsResponse>>;

//     getParsedTokenAccountsByOwner(
//         ownerAddress: PublicKey,
//         filter: TokenAccountsFilter,
//         commitment?: Commitment,
//     ): Promise<
//         RpcResponseAndContext<
//             Array<{
//                 pubkey: PublicKey;
//                 account: AccountInfo<ParsedAccountData>;
//             }>
//         >
//     >;

//     getLargestAccounts(
//         config?: GetLargestAccountsConfig,
//     ): Promise<RpcResponseAndContext<Array<AccountBalancePair>>>;

//     getTokenLargestAccounts(
//         mintAddress: PublicKey,
//         commitment?: Commitment,
//     ): Promise<RpcResponseAndContext<Array<TokenAccountBalancePair>>>;

//     getAccountInfoAndContext(
//         publicKey: PublicKey,
//         commitmentOrConfig?: Commitment | GetAccountInfoConfig,
//     ): Promise<RpcResponseAndContext<AccountInfo<Buffer> | null>>;

//     getParsedAccountInfo(
//         publicKey: PublicKey,
//         commitmentOrConfig?: Commitment | GetAccountInfoConfig,
//     ): Promise<
//         RpcResponseAndContext<AccountInfo<Buffer | ParsedAccountData> | null>
//     >;

//     getAccountInfo(
//         publicKey: PublicKey,
//         commitmentOrConfig?: Commitment | GetAccountInfoConfig,
//     ): Promise<AccountInfo<Buffer> | null>;

//     getMultipleParsedAccounts(
//         publicKeys: PublicKey[],
//         rawConfig?: GetMultipleAccountsConfig,
//     ): Promise<
//         RpcResponseAndContext<
//             (AccountInfo<Buffer | ParsedAccountData> | null)[]
//         >
//     >;

//     getMultipleAccountsInfoAndContext(
//         publicKeys: PublicKey[],
//         commitmentOrConfig?: Commitment | GetMultipleAccountsConfig,
//     ): Promise<RpcResponseAndContext<(AccountInfo<Buffer> | null)[]>>;

//     getMultipleAccountsInfo(
//         publicKeys: PublicKey[],
//         commitmentOrConfig?: Commitment | GetMultipleAccountsConfig,
//     ): Promise<(AccountInfo<Buffer> | null)[]>;

//     getStakeActivation(
//         publicKey: PublicKey,
//         commitmentOrConfig?: Commitment | GetStakeActivationConfig,
//         epoch?: number,
//     ): Promise<StakeActivationData>;

//     getProgramAccounts(
//         programId: PublicKey,
//         configOrCommitment?: GetProgramAccountsConfig | Commitment,
//     ): Promise<GetProgramAccountsResponse>;

//     getProgramAccounts(
//         programId: PublicKey,
//         configOrCommitment: GetProgramAccountsConfig & { withContext: true },
//     ): Promise<RpcResponseAndContext<GetProgramAccountsResponse>>;

//     getParsedProgramAccounts(
//         programId: PublicKey,
//         configOrCommitment?: GetParsedProgramAccountsConfig | Commitment,
//     ): Promise<
//         Array<{
//             pubkey: PublicKey;
//             account: AccountInfo<Buffer | ParsedAccountData>;
//         }>
//     >;

//     confirmTransaction(
//         strategy: TransactionConfirmationStrategy,
//         commitment?: Commitment,
//     ): Promise<RpcResponseAndContext<SignatureResult>>;

//     confirmTransaction(
//         strategy: TransactionSignature,
//         commitment?: Commitment,
//     ): Promise<RpcResponseAndContext<SignatureResult>>;

//     getClusterNodes(): Promise<Array<ContactInfo>>;

//     getVoteAccounts(commitment?: Commitment): Promise<VoteAccountStatus>;

//     getSlot(commitmentOrConfig?: Commitment | GetSlotConfig): Promise<number>;

//     getSlotLeader(
//         commitmentOrConfig?: Commitment | GetSlotLeaderConfig,
//     ): Promise<string>;

//     getSlotLeaders(startSlot: number, limit: number): Promise<Array<PublicKey>>;

//     getSignatureStatus(
//         signature: TransactionSignature,
//         config?: SignatureStatusConfig,
//     ): Promise<RpcResponseAndContext<SignatureStatus | null>>;

//     getSignatureStatuses(
//         signatures: Array<TransactionSignature>,
//         config?: SignatureStatusConfig,
//     ): Promise<RpcResponseAndContext<Array<SignatureStatus | null>>>;

//     getTransactionCount(
//         commitmentOrConfig?: Commitment | GetTransactionCountConfig,
//     ): Promise<number>;

//     getTotalSupply(commitment?: Commitment): Promise<number>;

//     getInflationGovernor(commitment?: Commitment): Promise<InflationGovernor>;

//     getInflationReward(
//         addresses: PublicKey[],
//         epoch?: number,
//         commitmentOrConfig?: Commitment | GetInflationRewardConfig,
//     ): Promise<(InflationReward | null)[]>;

//     getInflationRate(): Promise<InflationRate>;

//     getEpochInfo(
//         commitmentOrConfig?: Commitment | GetEpochInfoConfig,
//     ): Promise<EpochInfo>;

//     getEpochSchedule(): Promise<EpochSchedule>;

//     getLeaderSchedule(): Promise<LeaderSchedule>;

//     getMinimumBalanceForRentExemption(
//         dataLength: number,
//         commitment?: Commitment,
//     ): Promise<number>;

//     getRecentBlockhashAndContext(commitment?: Commitment): Promise<
//         RpcResponseAndContext<{
//             blockhash: Blockhash;
//             feeCalculator: FeeCalculator;
//         }>
//     >;

//     getRecentPerformanceSamples(limit?: number): Promise<Array<PerfSample>>;

//     getFeeCalculatorForBlockhash(
//         blockhash: Blockhash,
//         commitment?: Commitment,
//     ): Promise<RpcResponseAndContext<FeeCalculator | null>>;

//     getFeeForMessage(
//         message: VersionedMessage,
//         commitment?: Commitment,
//     ): Promise<RpcResponseAndContext<number | null>>;

//     getRecentPrioritizationFees(
//         config?: GetRecentPrioritizationFeesConfig,
//     ): Promise<RecentPrioritizationFees[]>;

//     getRecentBlockhash(
//         commitment?: Commitment,
//     ): Promise<{ blockhash: Blockhash; feeCalculator: FeeCalculator }>;

//     getLatestBlockhash(
//         commitmentOrConfig?: Commitment | GetLatestBlockhashConfig,
//     ): Promise<BlockhashWithExpiryBlockHeight>;

//     getLatestBlockhashAndContext(
//         commitmentOrConfig?: Commitment | GetLatestBlockhashConfig,
//     ): Promise<RpcResponseAndContext<BlockhashWithExpiryBlockHeight>>;

//     isBlockhashValid(
//         blockhash: Blockhash,
//         rawConfig?: IsBlockhashValidConfig,
//     ): Promise<RpcResponseAndContext<boolean>>;

//     getVersion(): Promise<Version>;

//     getGenesisHash(): Promise<string>;

//     getBlock(
//         slot: number,
//         rawConfig?: GetVersionedBlockConfig,
//     ): Promise<VersionedBlockResponse | null>;

//     getBlock(
//         slot: number,
//         rawConfig: GetVersionedBlockConfig & { transactionDetails: 'accounts' },
//     ): Promise<VersionedAccountsModeBlockResponse | null>;

//     getBlock(
//         slot: number,
//         rawConfig: GetVersionedBlockConfig & { transactionDetails: 'none' },
//     ): Promise<VersionedNoneModeBlockResponse | null>;

//     getParsedBlock(
//         slot: number,
//         rawConfig?: GetVersionedBlockConfig,
//     ): Promise<ParsedAccountsModeBlockResponse>;

//     getParsedBlock(
//         slot: number,
//         rawConfig: GetVersionedBlockConfig & { transactionDetails: 'accounts' },
//     ): Promise<ParsedAccountsModeBlockResponse>;

//     getParsedBlock(
//         slot: number,
//         rawConfig: GetVersionedBlockConfig & { transactionDetails: 'none' },
//     ): Promise<ParsedNoneModeBlockResponse>;

//     getBlockHeight(
//         commitmentOrConfig?: Commitment | GetBlockHeightConfig,
//     ): Promise<number>;

//     getBlockProduction(
//         configOrCommitment?: GetBlockProductionConfig | Commitment,
//     ): Promise<RpcResponseAndContext<BlockProduction>>;

//     getTransaction(
//         signature: string,
//         rawConfig?: GetVersionedTransactionConfig,
//     ): Promise<VersionedTransactionResponse | null>;

//     getParsedTransaction(
//         signature: TransactionSignature,
//         commitmentOrConfig?: GetVersionedTransactionConfig | Finality,
//     ): Promise<ParsedTransactionWithMeta | null>;

//     getParsedTransactions(
//         signatures: TransactionSignature[],
//         commitmentOrConfig?: GetVersionedTransactionConfig | Finality,
//     ): Promise<(ParsedTransactionWithMeta | null)[]>;

//     getTransactions(
//         signatures: TransactionSignature[],
//         commitmentOrConfig: GetVersionedTransactionConfig | Finality,
//     ): Promise<(VersionedTransactionResponse | null)[]>;

//     getConfirmedBlock(
//         slot: number,
//         commitment?: Finality,
//     ): Promise<ConfirmedBlock>;

//     getBlocks(
//         startSlot: number,
//         endSlot?: number,
//         commitment?: Finality,
//     ): Promise<Array<number>>;

//     getBlockSignatures(
//         slot: number,
//         commitment?: Finality,
//     ): Promise<BlockSignatures>;

//     getConfirmedBlockSignatures(
//         slot: number,
//         commitment?: Finality,
//     ): Promise<BlockSignatures>;

//     getConfirmedTransaction(
//         signature: TransactionSignature,
//         commitment?: Finality,
//     ): Promise<ConfirmedTransaction | null>;

//     getParsedConfirmedTransaction(
//         signature: TransactionSignature,
//         commitment?: Finality,
//     ): Promise<ParsedConfirmedTransaction | null>;

//     getParsedConfirmedTransactions(
//         signatures: TransactionSignature[],
//         commitment?: Finality,
//     ): Promise<(ParsedConfirmedTransaction | null)[]>;

//     getConfirmedSignaturesForAddress(
//         address: PublicKey,
//         startSlot: number,
//         endSlot: number,
//     ): Promise<Array<TransactionSignature>>;

//     getConfirmedSignaturesForAddress2(
//         address: PublicKey,
//         options?: ConfirmedSignaturesForAddress2Options,
//         commitment?: Finality,
//     ): Promise<Array<ConfirmedSignatureInfo>>;

//     getSignaturesForAddress(
//         address: PublicKey,
//         options?: SignaturesForAddressOptions,
//         commitment?: Finality,
//     ): Promise<Array<ConfirmedSignatureInfo>>;

//     getAddressLookupTable(
//         accountKey: PublicKey,
//         config?: GetAccountInfoConfig,
//     ): Promise<RpcResponseAndContext<AddressLookupTableAccount | null>>;

//     getNonceAndContext(
//         nonceAccount: PublicKey,
//         commitmentOrConfig?: Commitment | GetNonceAndContextConfig,
//     ): Promise<RpcResponseAndContext<NonceAccount | null>>;

//     getNonce(
//         nonceAccount: PublicKey,
//         commitmentOrConfig?: Commitment | GetNonceConfig,
//     ): Promise<NonceAccount | null>;

//     requestAirdrop(
//         to: PublicKey,
//         lamports: number,
//     ): Promise<TransactionSignature>;

//     getStakeMinimumDelegation(
//         config?: GetStakeMinimumDelegationConfig,
//     ): Promise<RpcResponseAndContext<number>>;

//     simulateTransaction(
//         transaction: VersionedTransaction,
//         config?: SimulateTransactionConfig,
//     ): Promise<RpcResponseAndContext<SimulatedTransactionResponse>>;

//     sendTransaction(
//         transaction: VersionedTransaction,
//         options?: SendOptions,
//     ): Promise<TransactionSignature>;

//     sendRawTransaction(
//         rawTransaction: Buffer | Uint8Array | Array<number>,
//         options?: SendOptions,
//     ): Promise<TransactionSignature>;

//     sendEncodedTransaction(
//         encodedTransaction: string,
//         options?: SendOptions,
//     ): Promise<TransactionSignature>;

//     onAccountChange(
//         publicKey: PublicKey,
//         callback: AccountChangeCallback,
//         config?: AccountSubscriptionConfig,
//     ): ClientSubscriptionId;

//     onProgramAccountChange(
//         programId: PublicKey,
//         callback: ProgramAccountChangeCallback,
//         config?: ProgramAccountSubscriptionConfig,
//     ): ClientSubscriptionId;

//     onLogs(
//         filter: LogsFilter,
//         callback: LogsCallback,
//         commitment?: Commitment,
//     ): ClientSubscriptionId;

//     onSlotChange(callback: SlotChangeCallback): ClientSubscriptionId;

//     onSlotUpdate(callback: SlotUpdateCallback): ClientSubscriptionId;

//     onSignature(
//         signature: TransactionSignature,
//         callback: SignatureResultCallback,
//         commitment?: Commitment,
//     ): ClientSubscriptionId;

//     onSignatureWithOptions(
//         signature: TransactionSignature,
//         callback: SignatureSubscriptionCallback,
//         options?: SignatureSubscriptionOptions,
//     ): ClientSubscriptionId;

//     onRootChange(callback: RootChangeCallback): ClientSubscriptionId;

//     removeAccountChangeListener(
//         clientSubscriptionId: ClientSubscriptionId,
//     ): Promise<void>;

//     removeProgramAccountChangeListener(
//         clientSubscriptionId: ClientSubscriptionId,
//     ): Promise<void>;

//     removeOnLogsListener(
//         clientSubscriptionId: ClientSubscriptionId,
//     ): Promise<void>;

//     removeSlotChangeListener(
//         clientSubscriptionId: ClientSubscriptionId,
//     ): Promise<void>;

//     removeSlotUpdateListener(
//         clientSubscriptionId: ClientSubscriptionId,
//     ): Promise<void>;

//     removeSignatureListener(
//         clientSubscriptionId: ClientSubscriptionId,
//     ): Promise<void>;

//     removeRootChangeListener(
//         clientSubscriptionId: ClientSubscriptionId,
//     ): Promise<void>;
// }
