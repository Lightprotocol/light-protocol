import {
  Rpc,
  LIGHT_TOKEN_PROGRAM_ID,
  ParsedTokenAccount,
  bn,
  assertV2Enabled,
  LightSystemProgram,
  defaultStaticAccountsStruct,
  ValidityProofWithContext,
} from "@lightprotocol/stateless.js";
import {
  ComputeBudgetProgram,
  PublicKey,
  TransactionInstruction,
  SystemProgram,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  TOKEN_2022_PROGRAM_ID,
  getAssociatedTokenAddressSync,
  createAssociatedTokenAccountIdempotentInstruction,
  TokenAccountNotFoundError,
} from "@solana/spl-token";
import { Buffer } from "buffer";
import {
  AccountView,
  checkNotFrozen,
  COLD_SOURCE_TYPES,
  getAtaView as _getAtaView,
  TokenAccountSource,
  isAuthorityForAccount,
  filterAccountForAuthority,
} from "../read/get-account";
import { getAssociatedTokenAddress } from "../read/associated-token-address";
import { createAtaIdempotent } from "./ata";
import { createWrapInstruction } from "./wrap";
import { getSplInterfaces, type SplInterface } from "../spl-interface";
import { getAtaProgramId, checkAtaAddress, AtaType } from "../read/ata-utils";
import type { LoadOptions } from "../load-options";
import { getMint } from "../read/get-mint";
import {
  COMPRESSED_TOKEN_PROGRAM_ID,
  deriveCpiAuthorityPda,
  MAX_TOP_UP,
  TokenDataVersion,
} from "../constants";
import {
  encodeTransfer2InstructionData,
  type Transfer2InstructionData,
  type MultiInputTokenDataWithContext,
  COMPRESSION_MODE_DECOMPRESS,
  type Compression,
  type Transfer2ExtensionData,
} from "./layout/layout-transfer2";
import { createSingleCompressedAccountRpc, getAtaOrNull } from "../account";
import { normalizeInstructionBatches, toLoadOptions } from "../helpers";
import { getAtaAddress } from "../read";
import type {
  CreateLoadInstructionsInput,
  TokenInterfaceAccount,
  CreateTransferInstructionsInput,
} from "../types";
import { toInstructionPlan } from "./_plan";

const COMPRESSED_ONLY_DISC = 31;
const COMPRESSED_ONLY_SIZE = 17; // u64 + u64 + u8

interface ParsedCompressedOnly {
  delegatedAmount: bigint;
  withheldTransferFee: bigint;
  isAta: boolean;
}

/**
 * Parse CompressedOnly extension from a Borsh-serialized TLV buffer
 * (Vec<ExtensionStruct>). Returns null if no CompressedOnly found.
 * @internal
 */
function parseCompressedOnlyFromTlv(
  tlv: Buffer | null,
): ParsedCompressedOnly | null {
  if (!tlv || tlv.length < 5) return null;
  try {
    let offset = 0;
    const vecLen = tlv.readUInt32LE(offset);
    offset += 4;
    for (let i = 0; i < vecLen; i++) {
      if (offset >= tlv.length) return null;
      const disc = tlv[offset];
      offset += 1;
      if (disc === COMPRESSED_ONLY_DISC) {
        if (offset + COMPRESSED_ONLY_SIZE > tlv.length) return null;
        const loDA = BigInt(tlv.readUInt32LE(offset));
        const hiDA = BigInt(tlv.readUInt32LE(offset + 4));
        const delegatedAmount = loDA | (hiDA << BigInt(32));
        const loFee = BigInt(tlv.readUInt32LE(offset + 8));
        const hiFee = BigInt(tlv.readUInt32LE(offset + 12));
        const withheldTransferFee = loFee | (hiFee << BigInt(32));
        const isAta = tlv[offset + 16] !== 0;
        return { delegatedAmount, withheldTransferFee, isAta };
      }
      const SIZES: Record<number, number | undefined> = {
        29: 8,
        30: 1,
        31: 17,
      };
      const size = SIZES[disc];
      if (size === undefined) {
        throw new Error(
          `parseCompressedOnlyFromTlv: unknown TLV extension discriminant ${disc}`,
        );
      }
      offset += size;
    }
  } catch {
    // Ignoring unknown TLV extensions.
    return null;
  }
  return null;
}

/**
 * Build inTlv array for Transfer2 from input compressed accounts.
 * For each account, if CompressedOnly TLV is present, converts it to
 * the instruction format (enriched with is_frozen, compression_index,
 * bump, owner_index). Returns null if no accounts have TLV.
 * @internal
 */
function buildInTlv(
  accounts: ParsedTokenAccount[],
  ownerIndex: number,
  owner: PublicKey,
  mint: PublicKey,
): Transfer2ExtensionData[][] | null {
  let hasAny = false;
  const result: Transfer2ExtensionData[][] = [];

  for (const acc of accounts) {
    const co = parseCompressedOnlyFromTlv(acc.parsed.tlv);
    if (!co) {
      result.push([]);
      continue;
    }
    hasAny = true;
    let bump = 0;
    if (co.isAta) {
      const seeds = [
        owner.toBuffer(),
        LIGHT_TOKEN_PROGRAM_ID.toBuffer(),
        mint.toBuffer(),
      ];
      const [, b] = PublicKey.findProgramAddressSync(
        seeds,
        LIGHT_TOKEN_PROGRAM_ID,
      );
      bump = b;
    }
    const isFrozen = acc.parsed.state === 2;
    result.push([
      {
        type: "CompressedOnly",
        data: {
          delegatedAmount: co.delegatedAmount,
          withheldTransferFee: co.withheldTransferFee,
          isFrozen,
          // This builder emits a single decompress compression per batch.
          // Keep index at 0 unless multi-compression output is added here.
          compressionIndex: 0,
          isAta: co.isAta,
          bump,
          ownerIndex,
        },
      },
    ]);
  }
  return hasAny ? result : null;
}

/**
 * Get token data version from compressed account discriminator.
 * @internal
 */
function getVersionFromDiscriminator(
  discriminator: number[] | undefined,
): number {
  if (!discriminator || discriminator.length < 8) {
    // Default to ShaFlat for new accounts without discriminator
    return TokenDataVersion.ShaFlat;
  }

  // V1 has discriminator[0] = 2
  if (discriminator[0] === 2) {
    return TokenDataVersion.V1;
  }

  // V2 and ShaFlat have version in discriminator[7]
  const versionByte = discriminator[7];
  if (versionByte === 3) {
    return TokenDataVersion.V2;
  }
  if (versionByte === 4) {
    return TokenDataVersion.ShaFlat;
  }

  // Default to ShaFlat
  return TokenDataVersion.ShaFlat;
}

/**
 * Build input token data for Transfer2 from parsed token accounts
 * @internal
 */
function buildInputTokenData(
  accounts: ParsedTokenAccount[],
  rootIndices: number[],
  packedAccountIndices: Map<string, number>,
): MultiInputTokenDataWithContext[] {
  return accounts.map((acc, i) => {
    const ownerKey = acc.parsed.owner.toBase58();
    const mintKey = acc.parsed.mint.toBase58();

    const version = getVersionFromDiscriminator(
      acc.compressedAccount.data?.discriminator,
    );

    return {
      owner: packedAccountIndices.get(ownerKey)!,
      amount: BigInt(acc.parsed.amount.toString()),
      hasDelegate: acc.parsed.delegate !== null,
      delegate: acc.parsed.delegate
        ? (packedAccountIndices.get(acc.parsed.delegate.toBase58()) ?? 0)
        : 0,
      mint: packedAccountIndices.get(mintKey)!,
      version,
      merkleContext: {
        merkleTreePubkeyIndex: packedAccountIndices.get(
          acc.compressedAccount.treeInfo.tree.toBase58(),
        )!,
        queuePubkeyIndex: packedAccountIndices.get(
          acc.compressedAccount.treeInfo.queue.toBase58(),
        )!,
        leafIndex: acc.compressedAccount.leafIndex,
        proveByIndex: acc.compressedAccount.proveByIndex,
      },
      rootIndex: rootIndices[i],
    };
  });
}

/**
 * Create decompress instruction using Transfer2.
 *
 * @internal Use createLoadAtaInstructions instead.
 *
 * Supports decompressing to both light-token accounts and SPL token accounts:
 * - For light-token destinations: No splInterface needed
 * - For SPL destinations: Provide splInterface and decimals
 *
 * @param input                             Decompress instruction input.
 * @param input.payer                       Fee payer public key.
 * @param input.inputCompressedTokenAccounts Input light-token accounts.
 * @param input.toAddress                   Destination token account address (light-token or SPL associated token account).
 * @param input.amount                      Amount to decompress.
 * @param input.validityProof               Validity proof (contains compressedProof and rootIndices).
 * @param input.splInterface                 Optional SPL pool info for SPL destinations.
 * @param input.decimals                    Mint decimals (required for SPL destinations).
 * @param input.maxTopUp                    Optional cap on rent top-up (units of 1k lamports; default no cap).
 * @param input.authority                   Optional signer (owner or delegate). When omitted, owner is the signer.
 * @returns TransactionInstruction
 */
export function createDecompressInstruction({
  payer,
  inputCompressedTokenAccounts,
  toAddress,
  amount,
  validityProof,
  splInterface,
  decimals,
  maxTopUp,
  authority,
}: {
  payer: PublicKey;
  inputCompressedTokenAccounts: ParsedTokenAccount[];
  toAddress: PublicKey;
  amount: bigint;
  validityProof: ValidityProofWithContext;
  splInterface?: SplInterface;
  decimals: number;
  maxTopUp?: number;
  authority?: PublicKey;
}): TransactionInstruction {
  if (inputCompressedTokenAccounts.length === 0) {
    throw new Error("No input light-token accounts provided");
  }

  const mint = inputCompressedTokenAccounts[0].parsed.mint;
  const owner = inputCompressedTokenAccounts[0].parsed.owner;

  // Build packed accounts map
  // Order: trees/queues first, then mint, owner, light-token account, light-token program
  const packedAccountIndices = new Map<string, number>();
  const packedAccounts: PublicKey[] = [];

  // Collect unique trees and queues
  const treeSet = new Set<string>();
  const queueSet = new Set<string>();
  for (const acc of inputCompressedTokenAccounts) {
    treeSet.add(acc.compressedAccount.treeInfo.tree.toBase58());
    queueSet.add(acc.compressedAccount.treeInfo.queue.toBase58());
  }

  // Add trees first (owned by account compression program)
  for (const tree of treeSet) {
    packedAccountIndices.set(tree, packedAccounts.length);
    packedAccounts.push(new PublicKey(tree));
  }

  let firstQueueIndex = 0;
  let isFirstQueue = true;
  for (const queue of queueSet) {
    if (isFirstQueue) {
      firstQueueIndex = packedAccounts.length;
      isFirstQueue = false;
    }
    packedAccountIndices.set(queue, packedAccounts.length);
    packedAccounts.push(new PublicKey(queue));
  }

  // Add mint
  const mintIndex = packedAccounts.length;
  packedAccountIndices.set(mint.toBase58(), mintIndex);
  packedAccounts.push(mint);

  // Add owner
  const ownerIndex = packedAccounts.length;
  packedAccountIndices.set(owner.toBase58(), ownerIndex);
  packedAccounts.push(owner);

  // Add destination token account (light-token or SPL)
  const destinationIndex = packedAccounts.length;
  packedAccountIndices.set(toAddress.toBase58(), destinationIndex);
  packedAccounts.push(toAddress);

  // Add unique delegate pubkeys from input accounts
  for (const acc of inputCompressedTokenAccounts) {
    if (acc.parsed.delegate) {
      const delegateKey = acc.parsed.delegate.toBase58();
      if (!packedAccountIndices.has(delegateKey)) {
        packedAccountIndices.set(delegateKey, packedAccounts.length);
        packedAccounts.push(acc.parsed.delegate);
      }
    }
  }

  // For SPL decompression, add pool account and token program
  let poolAccountIndex = 0;
  let poolIndex = 0;
  let poolBump = 0;
  let tokenProgramIndex = 0;

  if (splInterface) {
    // Add SPL interface PDA (token pool)
    poolAccountIndex = packedAccounts.length;
    packedAccountIndices.set(
      splInterface.poolPda.toBase58(),
      poolAccountIndex,
    );
    packedAccounts.push(splInterface.poolPda);

    // Add SPL token program
    tokenProgramIndex = packedAccounts.length;
    packedAccountIndices.set(
      splInterface.tokenProgramId.toBase58(),
      tokenProgramIndex,
    );
    packedAccounts.push(splInterface.tokenProgramId);

    poolIndex = splInterface.derivationIndex;
    poolBump = splInterface.bump;
  }

  // Build input token data
  const inTokenData = buildInputTokenData(
    inputCompressedTokenAccounts,
    validityProof.rootIndices,
    packedAccountIndices,
  );

  // Calculate total input amount and change
  const totalInputAmount = inputCompressedTokenAccounts.reduce(
    (sum, acc) => sum + BigInt(acc.parsed.amount.toString()),
    BigInt(0),
  );
  const changeAmount = totalInputAmount - amount;

  const outTokenData: {
    owner: number;
    amount: bigint;
    hasDelegate: boolean;
    delegate: number;
    mint: number;
    version: number;
  }[] = [];

  if (changeAmount > 0) {
    const version = getVersionFromDiscriminator(
      inputCompressedTokenAccounts[0].compressedAccount.data?.discriminator,
    );

    outTokenData.push({
      owner: ownerIndex,
      amount: changeAmount,
      hasDelegate: false,
      delegate: 0,
      mint: mintIndex,
      version,
    });
  }

  // Build decompress compression
  // For light-token: pool values are 0 (unused)
  // For SPL: pool values point to SPL interface PDA
  const compressions: Compression[] = [
    {
      mode: COMPRESSION_MODE_DECOMPRESS,
      amount,
      mint: mintIndex,
      sourceOrRecipient: destinationIndex,
      authority: 0, // Not needed for decompress
      poolAccountIndex: splInterface ? poolAccountIndex : 0,
      poolIndex: splInterface ? poolIndex : 0,
      bump: splInterface ? poolBump : 0,
      decimals,
    },
  ];

  // Build Transfer2 instruction data
  const instructionData: Transfer2InstructionData = {
    withTransactionHash: false,
    withLamportsChangeAccountMerkleTreeIndex: false,
    lamportsChangeAccountMerkleTreeIndex: 0,
    lamportsChangeAccountOwnerIndex: 0,
    outputQueue: firstQueueIndex, // First queue in packed accounts
    maxTopUp: maxTopUp ?? MAX_TOP_UP,
    cpiContext: null,
    compressions,
    proof: validityProof.compressedProof
      ? {
          a: Array.from(validityProof.compressedProof.a),
          b: Array.from(validityProof.compressedProof.b),
          c: Array.from(validityProof.compressedProof.c),
        }
      : null,
    inTokenData,
    outTokenData,
    inLamports: null,
    outLamports: null,
    inTlv: buildInTlv(inputCompressedTokenAccounts, ownerIndex, owner, mint),
    outTlv: null,
  };

  const data = encodeTransfer2InstructionData(instructionData);

  // Build accounts for Transfer2 with compressed accounts (full path)
  const {
    accountCompressionAuthority,
    registeredProgramPda,
    accountCompressionProgram,
  } = defaultStaticAccountsStruct();
  const signerIndex = (() => {
    if (!authority || authority.equals(owner)) {
      return ownerIndex;
    }
    const authorityIndex = packedAccountIndices.get(authority.toBase58());
    if (authorityIndex === undefined) {
      throw new Error(
        `Authority ${authority.toBase58()} is not present in packed accounts`,
      );
    }
    return authorityIndex;
  })();

  const keys = [
    // 0: light_system_program (non-mutable)
    {
      pubkey: LightSystemProgram.programId,
      isSigner: false,
      isWritable: false,
    },
    // 1: fee_payer (signer, mutable)
    { pubkey: payer, isSigner: true, isWritable: true },
    // 2: cpi_authority_pda
    {
      pubkey: deriveCpiAuthorityPda(),
      isSigner: false,
      isWritable: false,
    },
    // 3: registered_program_pda
    {
      pubkey: registeredProgramPda,
      isSigner: false,
      isWritable: false,
    },
    // 4: account_compression_authority
    {
      pubkey: accountCompressionAuthority,
      isSigner: false,
      isWritable: false,
    },
    // 5: account_compression_program
    {
      pubkey: accountCompressionProgram,
      isSigner: false,
      isWritable: false,
    },
    // 6: system_program
    {
      pubkey: SystemProgram.programId,
      isSigner: false,
      isWritable: false,
    },
    // 7+: packed_accounts (trees/queues come first)
    ...packedAccounts.map((pubkey, i) => {
      const isTreeOrQueue = i < treeSet.size + queueSet.size;
      const isDestination = pubkey.equals(toAddress);
      const isPool =
        splInterface !== undefined && pubkey.equals(splInterface.poolPda);
      return {
        pubkey,
        isSigner: i === signerIndex,
        isWritable: isTreeOrQueue || isDestination || isPool,
      };
    }),
  ];

  return new TransactionInstruction({
    programId: COMPRESSED_TOKEN_PROGRAM_ID,
    keys,
    data,
  });
}

const MAX_INPUT_ACCOUNTS = 8;

function chunkArray<T>(array: T[], chunkSize: number): T[][] {
  const chunks: T[][] = [];
  for (let i = 0; i < array.length; i += chunkSize) {
    chunks.push(array.slice(i, i + chunkSize));
  }
  return chunks;
}

function selectInputsForAmount(
  accounts: ParsedTokenAccount[],
  neededAmount: bigint,
): ParsedTokenAccount[] {
  if (accounts.length === 0 || neededAmount <= BigInt(0)) return [];

  const sorted = [...accounts].sort((a, b) => {
    const amtA = BigInt(a.parsed.amount.toString());
    const amtB = BigInt(b.parsed.amount.toString());
    if (amtB > amtA) return 1;
    if (amtB < amtA) return -1;
    return 0;
  });

  let accumulated = BigInt(0);
  let countNeeded = 0;
  for (const acc of sorted) {
    countNeeded++;
    accumulated += BigInt(acc.parsed.amount.toString());
    if (accumulated >= neededAmount) break;
  }

  const selectCount = Math.min(
    Math.max(countNeeded, MAX_INPUT_ACCOUNTS),
    sorted.length,
  );

  return sorted.slice(0, selectCount);
}

function assertUniqueInputHashes(chunks: ParsedTokenAccount[][]): void {
  const seen = new Set<string>();
  for (const chunk of chunks) {
    for (const acc of chunk) {
      const hashStr = acc.compressedAccount.hash.toString();
      if (seen.has(hashStr)) {
        throw new Error(
          `Duplicate compressed account hash across chunks: ${hashStr}. ` +
            `Each compressed account must appear in exactly one chunk.`,
        );
      }
      seen.add(hashStr);
    }
  }
}

function getCompressedTokenAccountsFromAtaSources(
  sources: TokenAccountSource[],
): ParsedTokenAccount[] {
  return sources
    .filter((source) => source.loadContext !== undefined)
    .filter((source) => COLD_SOURCE_TYPES.has(source.type))
    .map((source) => {
      const fullData = source.accountInfo.data;
      const discriminatorBytes = fullData.subarray(
        0,
        Math.min(8, fullData.length),
      );
      const accountDataBytes =
        fullData.length > 8 ? fullData.subarray(8) : Buffer.alloc(0);

      const compressedAccount = {
        treeInfo: source.loadContext!.treeInfo,
        hash: source.loadContext!.hash,
        leafIndex: source.loadContext!.leafIndex,
        proveByIndex: source.loadContext!.proveByIndex,
        owner: source.accountInfo.owner,
        lamports: bn(source.accountInfo.lamports),
        address: null,
        data:
          fullData.length === 0
            ? null
            : {
                discriminator: Array.from(discriminatorBytes),
                data: Buffer.from(accountDataBytes),
                dataHash: new Array(32).fill(0),
              },
        readOnly: false,
      };

      const state = !source.parsed.isInitialized
        ? 0
        : source.parsed.isFrozen
          ? 2
          : 1;

      return {
        compressedAccount: compressedAccount as any,
        parsed: {
          mint: source.parsed.mint,
          owner: source.parsed.owner,
          amount: bn(source.parsed.amount.toString()),
          delegate: source.parsed.delegate,
          state,
          tlv: source.parsed.tlvData.length > 0 ? source.parsed.tlvData : null,
        },
      } satisfies ParsedTokenAccount;
    });
}

export async function createLoadAtaInstructionsInner(
  rpc: Rpc,
  ata: PublicKey,
  owner: PublicKey,
  mint: PublicKey,
  decimals: number,
  payer?: PublicKey,
  loadOptions?: LoadOptions,
): Promise<TransactionInstruction[][]> {
  assertV2Enabled();
  payer ??= owner;
  const wrap = loadOptions?.wrap ?? false;

  const effectiveOwner = owner;
  const authorityPubkey = loadOptions?.delegatePubkey ?? owner;

  let accountView: AccountView;
  try {
    accountView = await _getAtaView(
      rpc,
      ata,
      effectiveOwner,
      mint,
      undefined,
      undefined,
      wrap,
    );
  } catch (e) {
    if (e instanceof TokenAccountNotFoundError) {
      return [];
    }
    throw e;
  }

  const isDelegate = !effectiveOwner.equals(authorityPubkey);
  if (isDelegate) {
    if (!isAuthorityForAccount(accountView, authorityPubkey)) {
      throw new Error("Signer is not the owner or a delegate of the account.");
    }
    accountView = filterAccountForAuthority(accountView, authorityPubkey);
  }

  const internalBatches = await _buildLoadBatches(
    rpc,
    payer,
    accountView,
    loadOptions,
    wrap,
    ata,
    undefined,
    authorityPubkey,
    decimals,
  );

  return internalBatches.map((batch) => [
    ComputeBudgetProgram.setComputeUnitLimit({
      units: calculateLoadBatchComputeUnits(batch),
    }),
    ...batch.instructions,
  ]);
}

interface InternalLoadBatch {
  instructions: TransactionInstruction[];
  compressedAccounts: ParsedTokenAccount[];
  wrapCount: number;
  hasAtaCreation: boolean;
}

const CU_ATA_CREATION = 30_000;
const CU_WRAP = 50_000;
const CU_DECOMPRESS_BASE = 50_000;
const CU_FULL_PROOF = 100_000;
const CU_PER_ACCOUNT_PROVE_BY_INDEX = 10_000;
const CU_PER_ACCOUNT_FULL_PROOF = 30_000;
const CU_BUFFER_FACTOR = 1.3;
const CU_MIN = 50_000;
const CU_MAX = 1_400_000;

function rawLoadBatchComputeUnits(batch: InternalLoadBatch): number {
  let cu = 0;
  if (batch.hasAtaCreation) cu += CU_ATA_CREATION;
  cu += batch.wrapCount * CU_WRAP;
  if (batch.compressedAccounts.length > 0) {
    cu += CU_DECOMPRESS_BASE;
    const needsFullProof = batch.compressedAccounts.some(
      (acc) => !(acc.compressedAccount.proveByIndex ?? false),
    );
    if (needsFullProof) cu += CU_FULL_PROOF;
    for (const acc of batch.compressedAccounts) {
      cu +=
        (acc.compressedAccount.proveByIndex ?? false)
          ? CU_PER_ACCOUNT_PROVE_BY_INDEX
          : CU_PER_ACCOUNT_FULL_PROOF;
    }
  }
  return cu;
}

function calculateLoadBatchComputeUnits(batch: InternalLoadBatch): number {
  const cu = Math.ceil(rawLoadBatchComputeUnits(batch) * CU_BUFFER_FACTOR);
  return Math.max(CU_MIN, Math.min(CU_MAX, cu));
}

async function _buildLoadBatches(
  rpc: Rpc,
  payer: PublicKey,
  ata: AccountView,
  options: LoadOptions | undefined,
  wrap: boolean,
  targetAta: PublicKey,
  targetAmount: bigint | undefined,
  authority: PublicKey | undefined,
  decimals: number,
): Promise<InternalLoadBatch[]> {
  if (!ata._isAta || !ata._owner || !ata._mint) {
    throw new Error(
      "AccountView must be from getAtaView (requires _isAta, _owner, _mint)",
    );
  }

  checkNotFrozen(ata, "load");

  const owner = ata._owner;
  const mint = ata._mint;
  const sources = ata._sources ?? [];

  const allCompressedAccounts =
    getCompressedTokenAccountsFromAtaSources(sources);

  const lightTokenAtaAddress = getAssociatedTokenAddress(mint, owner);
  const splAta = getAssociatedTokenAddressSync(
    mint,
    owner,
    false,
    TOKEN_PROGRAM_ID,
    getAtaProgramId(TOKEN_PROGRAM_ID),
  );
  const t22Ata = getAssociatedTokenAddressSync(
    mint,
    owner,
    false,
    TOKEN_2022_PROGRAM_ID,
    getAtaProgramId(TOKEN_2022_PROGRAM_ID),
  );

  let ataType: AtaType = "light-token";
  const validation = checkAtaAddress(targetAta, mint, owner);
  ataType = validation.type;
  if (wrap && ataType !== "light-token") {
    throw new Error(
      `For wrap=true, targetAta must be light-token associated token account. Got ${ataType} associated token account.`,
    );
  }

  const splSource = sources.find((s) => s.type === "spl");
  const t22Source = sources.find((s) => s.type === "token2022");
  const lightTokenHotSource = sources.find((s) => s.type === "light-token-hot");
  const coldSources = sources.filter((s) => COLD_SOURCE_TYPES.has(s.type));

  const splBalance = splSource?.amount ?? BigInt(0);
  const t22Balance = t22Source?.amount ?? BigInt(0);
  const coldBalance = coldSources.reduce((sum, s) => sum + s.amount, BigInt(0));

  if (
    splBalance === BigInt(0) &&
    t22Balance === BigInt(0) &&
    coldBalance === BigInt(0)
  ) {
    return [];
  }

  let splInterface: SplInterface | undefined;
  const needsSplInfo =
    wrap ||
    ataType === "spl" ||
    ataType === "token2022" ||
    splBalance > BigInt(0) ||
    t22Balance > BigInt(0);
  if (needsSplInfo) {
    try {
      const splInterfaces =
        options?.splInterfaces ?? (await getSplInterfaces(rpc, mint));
      splInterface = splInterfaces.find(
        (info: SplInterface) => info.isInitialized,
      );
    } catch (e) {
      if (splBalance > BigInt(0) || t22Balance > BigInt(0)) {
        throw e;
      }
    }
  }

  const setupInstructions: TransactionInstruction[] = [];
  let wrapCount = 0;
  let needsAtaCreation = false;

  let decompressTarget: PublicKey = lightTokenAtaAddress;
  let decompressSplInfo: SplInterface | undefined;
  let canDecompress = false;

  if (wrap) {
    decompressTarget = lightTokenAtaAddress;
    decompressSplInfo = undefined;
    canDecompress = true;

    if (!lightTokenHotSource) {
      needsAtaCreation = true;
      setupInstructions.push(
        createAtaIdempotent({
          payer,
          associatedToken: lightTokenAtaAddress,
          owner,
          mint,
          programId: LIGHT_TOKEN_PROGRAM_ID,
        }),
      );
    }

    if (splBalance > BigInt(0) && splInterface) {
      setupInstructions.push(
        createWrapInstruction({
          source: splAta,
          destination: lightTokenAtaAddress,
          owner,
          mint,
          amount: splBalance,
          splInterface,
          decimals,
          payer,
        }),
      );
      wrapCount++;
    }

    if (t22Balance > BigInt(0) && splInterface) {
      setupInstructions.push(
        createWrapInstruction({
          source: t22Ata,
          destination: lightTokenAtaAddress,
          owner,
          mint,
          amount: t22Balance,
          splInterface,
          decimals,
          payer,
        }),
      );
      wrapCount++;
    }
  } else {
    if (ataType === "light-token") {
      decompressTarget = lightTokenAtaAddress;
      decompressSplInfo = undefined;
      canDecompress = true;
      if (!lightTokenHotSource) {
        needsAtaCreation = true;
        setupInstructions.push(
          createAtaIdempotent({
            payer,
            associatedToken: lightTokenAtaAddress,
            owner,
            mint,
            programId: LIGHT_TOKEN_PROGRAM_ID,
          }),
        );
      }
    } else if (ataType === "spl" && splInterface) {
      decompressTarget = splAta;
      decompressSplInfo = splInterface;
      canDecompress = true;
      if (!splSource) {
        needsAtaCreation = true;
        setupInstructions.push(
          createAssociatedTokenAccountIdempotentInstruction(
            payer,
            splAta,
            owner,
            mint,
            TOKEN_PROGRAM_ID,
          ),
        );
      }
    } else if (ataType === "token2022" && splInterface) {
      decompressTarget = t22Ata;
      decompressSplInfo = splInterface;
      canDecompress = true;
      if (!t22Source) {
        needsAtaCreation = true;
        setupInstructions.push(
          createAssociatedTokenAccountIdempotentInstruction(
            payer,
            t22Ata,
            owner,
            mint,
            TOKEN_2022_PROGRAM_ID,
          ),
        );
      }
    }
  }

  let accountsToLoad = allCompressedAccounts;

  if (
    targetAmount !== undefined &&
    canDecompress &&
    allCompressedAccounts.length > 0
  ) {
    const isDelegate = authority !== undefined && !authority.equals(owner);
    const hotBalance = (() => {
      if (!lightTokenHotSource) return BigInt(0);
      if (isDelegate) {
        const delegated =
          lightTokenHotSource.parsed.delegatedAmount ?? BigInt(0);
        return delegated < lightTokenHotSource.amount
          ? delegated
          : lightTokenHotSource.amount;
      }
      return lightTokenHotSource.amount;
    })();
    let effectiveHotAfterSetup: bigint;

    if (wrap) {
      effectiveHotAfterSetup = hotBalance + splBalance + t22Balance;
    } else if (ataType === "light-token") {
      effectiveHotAfterSetup = hotBalance;
    } else if (ataType === "spl") {
      effectiveHotAfterSetup = splBalance;
    } else {
      effectiveHotAfterSetup = t22Balance;
    }

    const neededFromCold =
      targetAmount > effectiveHotAfterSetup
        ? targetAmount - effectiveHotAfterSetup
        : BigInt(0);

    if (neededFromCold === BigInt(0)) {
      accountsToLoad = [];
    } else {
      accountsToLoad = selectInputsForAmount(
        allCompressedAccounts,
        neededFromCold,
      );
    }
  }

  if (!canDecompress || accountsToLoad.length === 0) {
    if (setupInstructions.length === 0) return [];
    return [
      {
        instructions: setupInstructions,
        compressedAccounts: [],
        wrapCount,
        hasAtaCreation: needsAtaCreation,
      },
    ];
  }

  const chunks = chunkArray(accountsToLoad, MAX_INPUT_ACCOUNTS);
  assertUniqueInputHashes(chunks);

  const proofs = await Promise.all(
    chunks.map(async (chunk) => {
      const proofInputs = chunk.map((acc) => ({
        hash: acc.compressedAccount.hash,
        tree: acc.compressedAccount.treeInfo.tree,
        queue: acc.compressedAccount.treeInfo.queue,
      }));
      return rpc.getValidityProofV0(proofInputs);
    }),
  );

  const idempotentAtaIx = (() => {
    if (wrap || ataType === "light-token") {
      return createAtaIdempotent({
        payer,
        associatedToken: lightTokenAtaAddress,
        owner,
        mint,
        programId: LIGHT_TOKEN_PROGRAM_ID,
      });
    } else if (ataType === "spl") {
      return createAssociatedTokenAccountIdempotentInstruction(
        payer,
        splAta,
        owner,
        mint,
        TOKEN_PROGRAM_ID,
      );
    } else {
      return createAssociatedTokenAccountIdempotentInstruction(
        payer,
        t22Ata,
        owner,
        mint,
        TOKEN_2022_PROGRAM_ID,
      );
    }
  })();

  const batches: InternalLoadBatch[] = [];

  for (let i = 0; i < chunks.length; i++) {
    const chunk = chunks[i];
    const proof = proofs[i];
    const chunkAmount = chunk.reduce(
      (sum, acc) => sum + BigInt(acc.parsed.amount.toString()),
      BigInt(0),
    );

    const batchIxs: TransactionInstruction[] = [];
    let batchWrapCount = 0;
    let batchHasAtaCreation = false;

    if (i === 0) {
      batchIxs.push(...setupInstructions);
      batchWrapCount = wrapCount;
      batchHasAtaCreation = needsAtaCreation;
    } else {
      batchIxs.push(idempotentAtaIx);
      batchHasAtaCreation = true;
    }

    const authorityForDecompress = authority ?? owner;
    batchIxs.push(
      createDecompressInstruction({
        payer,
        inputCompressedTokenAccounts: chunk,
        toAddress: decompressTarget,
        amount: chunkAmount,
        validityProof: proof,
        splInterface: decompressSplInfo,
        decimals,
        authority: authorityForDecompress,
      }),
    );

    batches.push({
      instructions: batchIxs,
      compressedAccounts: chunk,
      wrapCount: batchWrapCount,
      hasAtaCreation: batchHasAtaCreation,
    });
  }

  return batches;
}

/**
 * Build load/decompress instruction batches for a specific associated token account.
 *
 * @param input             Load ATA instruction input.
 * @param input.rpc         RPC connection.
 * @param input.ata         Target associated token account address.
 * @param input.owner       Owner of the target token account.
 * @param input.mint        Mint address.
 * @param input.payer       Optional fee payer.
 * @param input.loadOptions Optional load options.
 * @returns Instruction batches that can require multiple transactions.
 */
export async function createLoadAtaInstructions({
  rpc,
  ata,
  owner,
  mint,
  payer,
  loadOptions,
}: {
  rpc: Rpc;
  ata: PublicKey;
  owner: PublicKey;
  mint: PublicKey;
  payer?: PublicKey;
  loadOptions?: LoadOptions;
}): Promise<TransactionInstruction[][]> {
  const mintInfo = await getMint(rpc, mint);
  return createLoadAtaInstructionsInner(
    rpc,
    ata,
    owner,
    mint,
    mintInfo.mint.decimals,
    payer,
    loadOptions,
  );
}

interface CreateLoadInstructionInternalInput
  extends CreateLoadInstructionsInput {
  authority?: PublicKey;
  account?: TokenInterfaceAccount | null;
  wrap?: boolean;
}

export async function createLoadInstructionInternal({
  rpc,
  payer,
  owner,
  mint,
  authority,
  account,
  wrap = false,
}: CreateLoadInstructionInternalInput): Promise<{
  instructions: TransactionInstruction[];
} | null> {
  const resolvedAccount =
    account ??
    (await getAtaOrNull({
      rpc,
      owner,
      mint,
    }));
  const targetAta = getAtaAddress({ owner, mint });

  const effectiveRpc =
    resolvedAccount && resolvedAccount.compressedAccount
      ? createSingleCompressedAccountRpc(
          rpc,
          owner,
          mint,
          resolvedAccount.compressedAccount,
        )
      : rpc;
  const instructions = normalizeInstructionBatches(
    "createLoadInstruction",
    await createLoadAtaInstructions({
      rpc: effectiveRpc,
      ata: targetAta,
      owner,
      mint,
      payer,
      loadOptions: toLoadOptions(owner, authority, wrap),
    }),
  );

  if (instructions.length === 0) {
    return null;
  }

  return {
    instructions,
  };
}

export async function buildLoadInstructionList(
  input: CreateLoadInstructionsInput & {
    authority?: CreateTransferInstructionsInput["authority"];
    account?: TokenInterfaceAccount | null;
    wrap?: boolean;
  },
): Promise<TransactionInstruction[]> {
  const load = await createLoadInstructionInternal(input);

  if (!load) {
    return [];
  }

  return load.instructions;
}

export async function createLoadInstruction({
  rpc,
  payer,
  owner,
  mint,
}: CreateLoadInstructionsInput): Promise<TransactionInstruction | null> {
  const load = await createLoadInstructionInternal({
    rpc,
    payer,
    owner,
    mint,
  });

  return load?.instructions[load.instructions.length - 1] ?? null;
}

export async function createLoadInstructions({
  rpc,
  payer,
  owner,
  mint,
}: CreateLoadInstructionsInput): Promise<TransactionInstruction[]> {
  return buildLoadInstructionList({
    rpc,
    payer,
    owner,
    mint,
    wrap: true,
  });
}

export async function createLoadInstructionPlan(
  input: CreateLoadInstructionsInput,
) {
  return toInstructionPlan(await createLoadInstructions(input));
}
