import { LiteSVM } from "litesvm";
import {
  PublicKey,
  Transaction,
  VersionedTransaction,
  ConfirmedSignatureInfo,
  ParsedTransactionWithMeta,
  AddressLookupTableAccount,
} from "@solana/web3.js";
import { defaultTestStateTreeAccounts } from "@lightprotocol/stateless.js";
import { TestRpc } from "./test-rpc/test-rpc";
import { LiteSVMConfig } from "./types";
import * as path from "path";
import * as fs from "fs";
import bs58 from "bs58";

/**
 * LiteSVM-based RPC implementation for testing Light Protocol programs
 * Extends TestRpc and overrides only the blockchain interaction methods
 * All proof generation and indexing logic is inherited from TestRpc
 */
export class LiteSVMRpc extends TestRpc {
  private litesvm: LiteSVM;
  private storedTransactions: Map<string, any>;
  private storedRawTransactions: Map<
    string,
    Transaction | VersionedTransaction
  >;

  constructor(
    lightWasm: any,
    config?: LiteSVMConfig,
    proverEndpoint: string = "http://127.0.0.1:3001",
  ) {
    // Initialize TestRpc with dummy endpoints
    super(
      "http://127.0.0.1:8899",
      lightWasm,
      "http://127.0.0.1:8784",
      proverEndpoint,
      { commitment: "confirmed" },
      { depth: defaultTestStateTreeAccounts().merkleTreeHeight },
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
      this.litesvm = this.litesvm.withTransactionHistory(
        config.transactionHistorySize,
      );
    }

    // Load Light Protocol programs
    this.loadLightPrograms();

    // Load custom programs if provided
    if (config?.customPrograms) {
      for (const { programId, programPath } of config.customPrograms) {
        this.litesvm.addProgramFromFile(programId, programPath);
      }
    }

    // Load state tree account fixtures
    this.loadAccountFixtures();
  }

  /**
   * Load Light Protocol program binaries from target/deploy
   */
  private loadLightPrograms(): void {
    // Find repo root by looking for target/deploy
    // Works whether running from source (src/) or built (dist/cjs/)
    let repoRoot = __dirname;
    while (!fs.existsSync(path.join(repoRoot, "target/deploy"))) {
      const parent = path.dirname(repoRoot);
      if (parent === repoRoot) {
        throw new Error("Could not find target/deploy directory");
      }
      repoRoot = parent;
    }
    const deployPath = path.join(repoRoot, "target/deploy");

    // Load Light Protocol programs
    const LIGHT_SYSTEM_PROGRAM_ID = new PublicKey(
      "SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7",
    );
    const ACCOUNT_COMPRESSION_PROGRAM_ID = new PublicKey(
      "compr6CUsB5m2jS4Y3831ztGSTnDpnKJTKS95d64XVq",
    );
    const COMPRESSED_TOKEN_PROGRAM_ID = new PublicKey(
      "cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m",
    );

    this.litesvm.addProgramFromFile(
      LIGHT_SYSTEM_PROGRAM_ID,
      path.join(deployPath, "light_system_program_pinocchio.so"),
    );
    this.litesvm.addProgramFromFile(
      ACCOUNT_COMPRESSION_PROGRAM_ID,
      path.join(deployPath, "account_compression.so"),
    );
    this.litesvm.addProgramFromFile(
      COMPRESSED_TOKEN_PROGRAM_ID,
      path.join(deployPath, "light_compressed_token.so"),
    );
  }

  /**
   * Load account fixtures from cli/accounts
   *
   * Note: State merkle trees and nullifier queues are loaded with their existing state.
   * TestRpc builds merkle trees in-memory from transaction events, so there will be
   * a mismatch between on-chain tree indices and TestRpc's in-memory indices until
   * transactions are processed.
   */
  private loadAccountFixtures(): void {
    // Find repo root by looking for cli/accounts
    // Works whether running from source (src/) or built (dist/cjs/)
    let repoRoot = __dirname;
    while (!fs.existsSync(path.join(repoRoot, "cli/accounts"))) {
      const parent = path.dirname(repoRoot);
      if (parent === repoRoot) {
        throw new Error("Could not find cli/accounts directory");
      }
      repoRoot = parent;
    }
    const accountsPath = path.join(repoRoot, "cli/accounts");

    // Load all account JSON files from cli/accounts
    const files = fs.readdirSync(accountsPath);

    for (const filename of files) {
      if (!filename.endsWith(".json")) continue;

      const filepath = path.join(accountsPath, filename);
      const accountData = JSON.parse(fs.readFileSync(filepath, "utf-8"));
      const pubkey = new PublicKey(accountData.pubkey);

      // Handle rentEpoch: if it exceeds JavaScript's MAX_SAFE_INTEGER or approaches u64::MAX,
      // set it to 0 to avoid overflow issues (same approach as litesvm's copyAccounts test)
      let rentEpoch = accountData.account.rentEpoch || 0;
      if (rentEpoch > Number.MAX_SAFE_INTEGER) {
        rentEpoch = 0;
      }

      const account = {
        lamports: accountData.account.lamports,
        data: Buffer.from(accountData.account.data[0], "base64"),
        owner: new PublicKey(accountData.account.owner),
        executable: accountData.account.executable,
        rentEpoch,
      };
      this.litesvm.setAccount(pubkey, account);
    }
  }

  /**
   * Send raw transaction (for compatibility)
   */
  override async sendRawTransaction(
    rawTransaction: Buffer | Uint8Array | Array<number>,
    options?: any,
  ): Promise<string> {
    // Deserialize and send
    const tx = Transaction.from(Buffer.from(rawTransaction));
    return this.sendTransaction(tx);
  }

  /**
   * Send and confirm a transaction (wrapper for compatibility with SPL token)
   * Just calls sendTransaction since LiteSVM executes synchronously
   */
  async sendAndConfirmTransaction(
    transaction: Transaction | VersionedTransaction,
    signers?: any[],
    options?: any,
  ): Promise<string> {
    // Sign the transaction if signers are provided
    if (signers && signers.length > 0 && "sign" in transaction) {
      (transaction as Transaction).sign(...(signers as any));
    }

    // Just call sendTransaction - LiteSVM executes synchronously
    return this.sendTransaction(transaction);
  }

  /**
   * Send and execute a transaction using LiteSVM
   */
  override async sendTransaction(
    transaction: Transaction | VersionedTransaction,
    ...args: any[]
  ): Promise<string> {
    // If it's a legacy transaction without recentBlockhash, add one
    if ("recentBlockhash" in transaction && !transaction.recentBlockhash) {
      transaction.recentBlockhash = this.litesvm.latestBlockhash();
    }

    // If it's a legacy transaction without fee payer, try to get it from signatures or signers
    if ("feePayer" in transaction && !transaction.feePayer) {
      // Try to get fee payer from signers in args
      const signers = args[0];
      if (
        Array.isArray(signers) &&
        signers.length > 0 &&
        signers[0].publicKey
      ) {
        transaction.feePayer = signers[0].publicKey;
      } else if (transaction.signatures && transaction.signatures.length > 0) {
        transaction.feePayer = transaction.signatures[0].publicKey;
      }
    }

    // Check transaction size before sending
    const serialized = transaction.serialize({
      requireAllSignatures: false,
      verifySignatures: false,
    });
    const txSize = serialized.length;
    const MAX_TRANSACTION_SIZE = 1232; // Solana's practical max transaction size

    // Detailed logging for transaction size analysis
    if ("message" in transaction) {
      // VersionedTransaction
      const msg = transaction.message;

      if (msg.addressTableLookups?.length > 0) {
        msg.addressTableLookups.forEach((lookup, i) => {
          console.log(
            `    - Lookup ${i}: ${lookup.writableIndexes.length} writable, ${lookup.readonlyIndexes.length} readonly`,
          );
        });
      }
    }

    if (txSize > MAX_TRANSACTION_SIZE) {
      console.error(
        "[LiteSVM] Transaction too large:",
        txSize,
        "bytes exceeds",
        MAX_TRANSACTION_SIZE,
        "bytes",
      );

      // Check if it's a versioned transaction with lookup tables
      if (
        "message" in transaction &&
        transaction.message.addressTableLookups?.length > 0
      ) {
        console.error(
          "[LiteSVM] Transaction uses",
          transaction.message.addressTableLookups.length,
          "lookup tables but still exceeds size limit",
        );
        console.error(
          "[LiteSVM] This suggests the transaction is too complex even with LUT optimization",
        );
      }

      throw new Error(
        `Transaction size ${txSize} bytes exceeds maximum of ${MAX_TRANSACTION_SIZE} bytes. Consider using fewer recipients or optimizing with address lookup tables.`,
      );
    }

    const result = this.litesvm.sendTransaction(transaction);

    // Check if transaction succeeded or failed
    if ("err" in result && typeof result.err === "function") {
      const error = result.err();
      const sim_result = this.litesvm.simulateTransaction(transaction);
      const logs = sim_result.meta().prettyLogs();

      console.error("[LiteSVM] Transaction error:", error);
      console.error("[LiteSVM] Transaction logs:", logs);

      const errorMessage =
        logs.length > 0
          ? `Transaction failed (error ${error}):\n${logs}`
          : `Transaction failed: ${error}`;
      throw new Error(errorMessage);
    }

    const successResult = result as any;
    const logs = successResult.logs();
    const signatureBytes = successResult.signature();
    const signature = bs58.encode(signatureBytes);

    // Extract inner instructions from LiteSVM result
    const innerInstructionsRaw = successResult.innerInstructions();
    const innerInstructions = innerInstructionsRaw.map(
      (group: any[], index: number) => ({
        index,
        instructions: group.map((inner: any) => {
          const compiledIx = inner.instruction();
          return {
            programIdIndex: compiledIx.programIdIndex(),
            accounts: Array.from(compiledIx.accounts()),
            data: bs58.encode(compiledIx.data()),
          };
        }),
      }),
    );

    // Store transaction metadata for TestRpc to query later
    this.storedTransactions.set(signature, {
      signature,
      logs,
      slot: 1,
      blockTime: Math.floor(Date.now() / 1000),
      confirmationStatus: "confirmed",
      innerInstructions,
    });

    // Store raw transaction for getParsedTransactions
    this.storedRawTransactions.set(signature, transaction);

    // Expire blockhash to force new blockhash for next transaction
    // This prevents transaction replay errors when creating similar transactions
    this.litesvm.expireBlockhash();

    return signature;
  }

  /**
   * Override getSignaturesForAddress to return our stored LiteSVM transactions
   * This allows TestRpc's proof generation to work with LiteSVM transactions
   *
   * Note: Returns in reverse order because getParsedEvents will reverse them again
   */
  override async getSignaturesForAddress(
    address: PublicKey,
    options?: any,
  ): Promise<ConfirmedSignatureInfo[]> {
    // Return all stored transactions in reverse order
    // TestRpc's getParsedEvents will reverse them again, resulting in correct order
    return Array.from(this.storedTransactions.values())
      .reverse()
      .map((tx) => ({
        signature: tx.signature,
        slot: tx.slot,
        err: null,
        memo: null,
        blockTime: tx.blockTime,
        confirmationStatus: tx.confirmationStatus,
      }));
  }

  /**
   * Override getStateTreeInfos to return only the first tree of the correct type
   * This ensures all compress operations use the same tree, avoiding the
   * random tree selection that causes leafIndex mismatches
   */
  override async getStateTreeInfos(): Promise<any[]> {
    const allInfos = await super.getStateTreeInfos();
    // In V2, localTestActiveStateTreeInfos returns both V1 and V2 trees
    // We need to find the first V2 tree, not just take the first tree overall
    const { TreeType, featureFlags } = await import(
      "@lightprotocol/stateless.js"
    );
    const expectedType = featureFlags.isV2()
      ? TreeType.StateV2
      : TreeType.StateV1;
    const matchingTree = allInfos.find(
      (info) => info.treeType === expectedType,
    );
    if (!matchingTree) {
      throw new Error(
        `No ${expectedType} tree found in localTestActiveStateTreeInfos`,
      );
    }
    return [matchingTree];
  }

  /**
   * Override getTransaction to return stored LiteSVM transaction
   */
  override async getTransaction(
    signature: string | Uint8Array,
    options?: any,
  ): Promise<any> {
    // Convert Uint8Array signature to base58 string if needed
    const sigString =
      typeof signature === "string" ? signature : bs58.encode(signature);

    const tx = this.storedTransactions.get(sigString);
    const rawTx = this.storedRawTransactions.get(sigString);

    if (!tx || !rawTx) {
      return null;
    }

    // Extract message and account keys from transaction
    let message: any;
    let accountKeys: PublicKey[];
    let compiledInstructions: any[];

    if ("message" in rawTx) {
      // VersionedTransaction
      message = rawTx.message;
      // For VersionedTransaction, accountKeys are in staticAccountKeys property
      if ("staticAccountKeys" in message) {
        accountKeys = message.staticAccountKeys;
      } else if ("accountKeys" in message) {
        accountKeys = message.accountKeys;
      } else if (typeof message.getAccountKeys === "function") {
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
          recentBlockhash:
            message.recentBlockhash || message.header?.recentBlockhash || "",
          addressTableLookups: message.addressTableLookups || [],
        },
        signatures:
          "signatures" in rawTx ? rawTx.signatures : [(rawTx as any).signature],
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
    options?: any,
  ): Promise<(ParsedTransactionWithMeta | null)[]> {
    return signatures.map((signature) => {
      const tx = this.storedTransactions.get(signature);
      const rawTx = this.storedRawTransactions.get(signature);

      if (!tx || !rawTx) {
        return null;
      }

      // Extract message and account keys from transaction
      let message: any;
      let accountKeys: PublicKey[];

      if ("message" in rawTx) {
        // VersionedTransaction
        message = rawTx.message;
        if ("staticAccountKeys" in message) {
          accountKeys = message.staticAccountKeys;
        } else if ("accountKeys" in message) {
          accountKeys = message.accountKeys;
        } else if (typeof message.getAccountKeys === "function") {
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

      // Use the stored signature directly since we already have it as a base58 string
      const signatures = [tx.signature];

      return {
        slot: tx.slot,
        blockTime: tx.blockTime,
        transaction: {
          message: {
            accountKeys: accountKeys.map((key: PublicKey) => ({
              pubkey: key,
              signer: false,
              writable: false,
              source: "transaction" as const,
            })),
            instructions: [],
            recentBlockhash:
              message.recentBlockhash || message.header?.recentBlockhash || "",
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
    lamports: number,
  ): Promise<string> {
    this.litesvm.airdrop(pubkey, BigInt(lamports));
    return "mock-airdrop-signature";
  }

  /**
   * Get account info using LiteSVM
   */
  override async getAccountInfo(
    publicKey: PublicKey,
    commitmentOrConfig?: any,
  ): Promise<any> {
    const account = this.litesvm.getAccount(publicKey);
    if (!account) {
      return null;
    }
    return {
      executable: account.executable,
      owner: new PublicKey(account.owner),
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
    commitmentOrConfig?: any,
  ): Promise<(any | null)[]> {
    return publicKeys.map((publicKey) => {
      const account = this.litesvm.getAccount(publicKey);
      if (!account) {
        return null;
      }
      return {
        executable: account.executable,
        owner: new PublicKey(account.owner),
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
    commitment?: any,
  ): Promise<number> {
    return Number(
      this.litesvm.minimumBalanceForRentExemption(BigInt(dataLength)),
    );
  }

  /**
   * Simulate a transaction without executing it
   */
  override async simulateTransaction(
    transactionOrMessage: any,
    configOrSigners?: any,
    includeAccounts?: any,
  ): Promise<any> {
    // Extract transaction from possible message wrapper
    const transaction =
      "message" in transactionOrMessage
        ? transactionOrMessage
        : transactionOrMessage;

    const result = this.litesvm.simulateTransaction(transaction);

    // Check if simulation failed
    if ("err" in result && typeof result.err === "function") {
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
        returnData: meta.returnData()
          ? {
              programId: new PublicKey(
                meta.returnData().programId(),
              ).toBase58(),
              data: [
                Buffer.from(meta.returnData().data()).toString("base64"),
                "base64",
              ],
            }
          : null,
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
    commitment?: any,
  ): Promise<any> {
    return {
      context: { slot: 1 },
      value: { err: null },
    };
  }

  /**
   * Get signature statuses (return instant confirmation for LiteSVM)
   * Since LiteSVM executes synchronously, all transactions are immediately finalized
   */
  override async getSignatureStatuses(
    signatures: string[],
    config?: any,
  ): Promise<any> {
    // LiteSVM executes synchronously, so all transactions are immediately finalized
    const commitment = "finalized";
    return {
      context: { slot: 1 },
      value: signatures.map((signature) => {
        // Check if we have this transaction stored
        const tx = this.storedTransactions.get(signature);
        if (!tx) {
          return null; // Transaction not found
        }
        const statusObj = {
          slot: 1,
          confirmations: null,
          err: null,
          confirmationStatus: commitment as any, // Return the requested commitment level
        };
        return statusObj;
      }),
    };
  }

  /**
   * Get current slot from LiteSVM
   */
  override async getSlot(commitment?: any): Promise<number> {
    return Number(this.litesvm.getClock().slot);
  }

  /**
   * Get token account balance
   */
  override async getTokenAccountBalance(
    tokenAccount: PublicKey,
    commitment?: any,
  ): Promise<any> {
    const account = await this.getAccountInfo(tokenAccount);
    if (!account) {
      throw new Error(`Token account ${tokenAccount.toBase58()} not found`);
    }

    // Parse SPL token account data
    // Token account layout (165 bytes total):
    // 0-32: mint
    // 32-64: owner
    // 64-72: amount (u64)
    // 72-108: delegate (36 bytes, optional)
    // 108-109: state (1 byte)
    // 109-117: is_native (8 bytes, optional u64)
    // 117-125: delegated_amount (8 bytes, u64)
    // 125-157: close_authority (32 bytes, optional)
    const data = Buffer.from(account.data);

    // Read amount as u64 little-endian at offset 64
    const amount = data.readBigUInt64LE(64);

    // Read decimals from mint account (we'll default to 9 for now)
    // In a real implementation, we'd read the mint account to get decimals
    const decimals = 9; // Default SOL decimals

    return {
      context: { slot: 1 },
      value: {
        amount: amount.toString(),
        decimals,
        uiAmount: Number(amount) / Math.pow(10, decimals),
        uiAmountString: (Number(amount) / Math.pow(10, decimals)).toString(),
      },
    };
  }

  /**
   * Get address lookup table account
   */
  override async getAddressLookupTable(
    accountKey: PublicKey,
    config?: any,
  ): Promise<any> {
    const account = await this.getAccountInfo(accountKey);
    if (!account) {
      return {
        context: { slot: 1 },
        value: null,
      };
    }

    try {
      const state = AddressLookupTableAccount.deserialize(
        new Uint8Array(account.data),
      );

      return {
        context: { slot: 1 },
        value: {
          key: accountKey,
          state,
        },
      };
    } catch (error) {
      console.error(
        "[LiteSVM] Failed to deserialize address lookup table:",
        error,
      );
      return {
        context: { slot: 1 },
        value: null,
      };
    }
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
  proverEndpoint: string = "http://127.0.0.1:3001",
): Promise<LiteSVMRpc> {
  return new LiteSVMRpc(lightWasm, config, proverEndpoint);
}
