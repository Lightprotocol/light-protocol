/// <reference types="node" />
import { PublicKey } from "@solana/web3.js";
import { Action } from "../transaction";
import { TokenData } from "../types";
import { Balance, Provider, User } from "../wallet";
export type TestInputs = {
    amountSpl?: number;
    amountSol?: number;
    token: string;
    type: Action;
    recipient?: PublicKey;
    expectedUtxoHistoryLength: number;
    expectedSpentUtxosLength?: number;
    recipientSeed?: string;
    expectedRecipientUtxoLength?: number;
    mergedUtxo?: boolean;
    shieldToRecipient?: boolean;
    utxoCommitments?: string[];
    storage?: boolean;
    message?: Buffer;
    isMerge?: boolean;
};
export type TestUserBalances = {
    user: User;
    preShieldedBalance?: Balance;
    preShieldedInboxBalance?: Balance;
    preTokenBalance?: number | null;
    preSolBalance?: number;
    isSender: boolean;
    recipientSplAccount?: PublicKey;
    senderSplAccount?: PublicKey;
};
export declare class UserTestAssertHelper {
    private recentTransaction?;
    provider: Provider;
    sender: TestUserBalances;
    recipient: TestUserBalances;
    testInputs: TestInputs;
    tokenCtx: TokenData;
    relayerPreSolBalance?: number;
    recipientPreSolBalance?: number;
    constructor({ userSender, userRecipient, provider, testInputs, }: {
        userSender: User;
        userRecipient: User;
        provider: Provider;
        testInputs: TestInputs;
    });
    fetchAndSaveState(): Promise<void>;
    assertRecentTransactionIsIndexedCorrectly(): Promise<void>;
    /**
     * Checks:
     * - every utxo in utxos is inserted and is not spent
     * - every utxo in spent utxos is spent
     * - if an utxo has an spl asset it is categorized in that spl asset
     * - every utxo in an spl TokenBalance has a balance in this token
     * - for every TokenUtxoBalance total amounts are correct
     */
    assertBalance(user: User): Promise<void>;
    assertInboxBalance(user: User): Promise<void>;
    /**
     * - check that utxos with an aggregate amount greater or equal than the spl and sol transfer amounts were spent
     */
    assertUserUtxoSpent(): Promise<void>;
    assertShieldedSplBalance(amount: number, userBalances: TestUserBalances, shieldToRecipient?: boolean): Promise<void>;
    assertSplBalance(amount: number, userBalances: TestUserBalances): Promise<void>;
    assertSolBalance(lamports: number, transactionCost: number, preSolBalance: number, recipient: PublicKey): Promise<void>;
    assertShieldedSolBalance(lamports: number, userBalances: TestUserBalances, shieldToRecipient?: boolean): Promise<void>;
    assertNullifierAccountDoesNotExist(nullifier: string): Promise<void>;
    assertNullifierAccountExists(nullifier: string): Promise<void>;
    checkShieldedTransferReceived(transferAmountSpl: number, transferAmountSol: number, mint: PublicKey): Promise<void>;
    standardAsserts(): Promise<void>;
    assertRelayerFee(): Promise<void>;
    /**
     * Asynchronously checks if token shielding has been performed correctly for a user.
     * This method performs the following checks:
     *
     * 1. Asserts that the user's shielded token balance has increased by the amount shielded.
     * 2. Asserts that the user's token balance has decreased by the amount shielded.
     * 3. Asserts that the user's sol shielded balance has increased by the additional sol amount.
     * 4. Asserts that the length of spent UTXOs matches the expected spent UTXOs length.
     * 5. Asserts that the nullifier account exists for the user's first UTXO.
     * 6. Asserts that the recent indexed transaction is of type SHIELD and has the correct values.
     *
     * @returns {Promise<void>} Resolves when all checks are successful, otherwise throws an error.
     */
    checkSplShielded(): Promise<void>;
    /**
     * Asynchronously checks if SOL shielding has been performed correctly for a user.
     * This method performs the following checks:
     *
     * 1. Asserts recipient user balance increased by shielded amount.
     * 2. Asserts sender users sol balance decreased by shielded amount.
     * 3. Asserts that user UTXOs are spent and updated correctly.
     * 4. Asserts that the recent indexed transaction is of type SHIELD and has the correct values.
     *
     * Note: The temporary account cost calculation is not deterministic and may vary depending on whether the user has
     * shielded SPL tokens before. This needs to be handled carefully.
     *
     * @returns {Promise<void>} Resolves when all checks are successful, otherwise throws an error.
     */
    checkSolShielded(): Promise<void>;
    checkSolUnshielded(): Promise<void>;
    checkSolTransferred(): Promise<void>;
    /**
     * Asynchronously checks if token unshielding has been performed correctly for a user.
     * This method performs the following checks:
     *
     * 1. Asserts that the user's shielded token balance has decreased by the amount unshielded.
     * 2. Asserts that the recipient's token balance has increased by the amount unshielded.
     * 3. Asserts that the user's shielded SOL balance has decreased by the fee.
     * 4. Asserts that user UTXOs are spent and updated correctly.
     * 5. Asserts that the recent indexed transaction is of type UNSHIELD and has the correct values.
     *
     * @returns {Promise<void>} Resolves when all checks are successful, otherwise throws an error.
     */
    checkSplUnshielded(): Promise<void>;
    /**
     * Asynchronously checks if a shielded token transfer has been performed correctly for a user.
     * This method performs the following checks:
     *
     * 1. Asserts that the user's shielded token balance has decreased by the amount transferred.
     * 2. Asserts that the user's shielded SOL balance has decreased by the relayer fee.
     * 3. Asserts that user UTXOs are spent and updated correctly.
     * 4. Asserts that the recent indexed transaction is of type SHIELD and has the correct values.
     * 5. Assert that the transfer has been received correctly by the shielded recipient's account.
     *
     * @returns {Promise<void>} Resolves when all checks are successful, otherwise throws an error.
     */
    checkSplTransferred(): Promise<void>;
    checkMergedAll(): Promise<void>;
    checkMerged(): Promise<void>;
    checkMessageStored(): Promise<void>;
    assertStoredWithTransfer(): Promise<void>;
    assertStoredWithShield(): Promise<void>;
    checkCommittedBalanceSpl(): Promise<void>;
}
