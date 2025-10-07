import { AccountInfo, Commitment, PublicKey } from '@solana/web3.js';
import {
    TOKEN_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
    unpackAccount as splUnpackAccount,
} from '@solana/spl-token';
import {
    Rpc,
    CTOKEN_PROGRAM_ID,
    MerkleContext,
    CompressedAccountWithMerkleContext,
} from '@lightprotocol/stateless.js';
import { Buffer } from 'buffer';
import BN from 'bn.js';

export interface Account {
    address: PublicKey;
    mint: PublicKey;
    owner: PublicKey;
    amount: bigint;
    delegate: PublicKey | null;
    delegatedAmount: bigint;
    isInitialized: boolean;
    isFrozen: boolean;
    isNative: boolean;
    rentExemptReserve: bigint | null;
    closeAuthority: PublicKey | null;
    tlvData: Buffer;
}

export enum AccountState {
    Uninitialized = 0,
    Initialized = 1,
    Frozen = 2,
}

export interface ParsedTokenAccount {
    compressedAccount: CompressedAccountWithMerkleContext;
    parsed: Account;
}

function parseTokenData(data: Buffer): {
    mint: PublicKey;
    owner: PublicKey;
    amount: BN;
    delegate: PublicKey | null;
    state: number;
    tlv: Buffer | null;
} | null {
    if (!data || data.length === 0) return null;

    try {
        let offset = 0;
        const mint = new PublicKey(data.slice(offset, offset + 32));
        offset += 32;
        const owner = new PublicKey(data.slice(offset, offset + 32));
        offset += 32;
        const amount = new BN(data.slice(offset, offset + 8), 'le');
        offset += 8;
        const delegateOption = data[offset];
        offset += 1;
        const delegate = delegateOption
            ? new PublicKey(data.slice(offset, offset + 32))
            : null;
        offset += 32;
        const state = data[offset];
        offset += 1;
        const tlvOption = data[offset];
        offset += 1;
        const tlv = tlvOption ? data.slice(offset) : null;

        return {
            mint,
            owner,
            amount,
            delegate,
            state,
            tlv,
        };
    } catch (error) {
        console.error('Token data parsing error:', error);
        return null;
    }
}

function convertTokenDataToAccount(
    address: PublicKey,
    tokenData: {
        mint: PublicKey;
        owner: PublicKey;
        amount: BN;
        delegate: PublicKey | null;
        state: number;
        tlv: Buffer | null;
    },
): Account {
    return {
        address,
        mint: tokenData.mint,
        owner: tokenData.owner,
        amount: BigInt(tokenData.amount.toString()),
        delegate: tokenData.delegate,
        delegatedAmount: BigInt(0),
        isInitialized: tokenData.state !== AccountState.Uninitialized,
        isFrozen: tokenData.state === AccountState.Frozen,
        isNative: false,
        rentExemptReserve: null,
        closeAuthority: null,
        tlvData: tokenData.tlv ? Buffer.from(tokenData.tlv) : Buffer.alloc(0),
    };
}

/**
 * Retrieve information about a token account (SPL or compressed)
 *
 * @param rpc        RPC connection to use
 * @param address    Token account address
 * @param commitment Desired level of commitment for querying the state
 * @param programId  Token program ID (defaults to TOKEN_PROGRAM_ID)
 *
 * @return Token account information with compression context if applicable
 */
export async function getAccountInterface(
    rpc: Rpc,
    address: PublicKey,
    commitment?: Commitment,
    programId: PublicKey = TOKEN_PROGRAM_ID,
): Promise<{
    accountInfo: AccountInfo<Buffer>;
    parsed: Account;
    isCompressed: boolean;
    merkleContext?: MerkleContext;
} | null> {
    if (programId.equals(CTOKEN_PROGRAM_ID)) {
        const [onchainResult, compressedResult] = await Promise.allSettled([
            rpc.getAccountInfo(address, commitment),
            rpc.getCompressedTokenAccountsByOwner(address),
        ]);

        const onchainAccount =
            onchainResult.status === 'fulfilled' ? onchainResult.value : null;
        const compressedAccount =
            compressedResult.status === 'fulfilled' &&
            compressedResult.value.items.length > 0
                ? compressedResult.value.items[0].compressedAccount
                : null;

        if (onchainAccount) {
            if (!onchainAccount.owner.equals(programId)) {
                throw new Error(
                    `Invalid owner ${onchainAccount.owner.toBase58()} for token layout`,
                );
            }

            const parsed = parseTokenData(onchainAccount.data);
            if (!parsed) {
                throw new Error('Invalid token data');
            }

            if (compressedAccount) {
                throw Error('Expected no compressed token account');
            }

            return {
                accountInfo: onchainAccount,
                merkleContext: undefined,
                parsed: convertTokenDataToAccount(address, parsed),
                isCompressed: false,
            };
        }

        if (
            compressedAccount &&
            compressedAccount.data &&
            compressedAccount.data.data.length > 0
        ) {
            const accountInfo: AccountInfo<Buffer> = {
                executable: false,
                owner: compressedAccount.owner,
                lamports: compressedAccount.lamports.toNumber(),
                data: Buffer.concat([
                    Buffer.from(compressedAccount.data!.discriminator),
                    compressedAccount.data!.data,
                ]),
                rentEpoch: undefined,
            };

            if (!compressedAccount.owner.equals(programId)) {
                throw new Error(
                    `Invalid owner ${compressedAccount.owner.toBase58()} for token layout`,
                );
            }

            const parsed = parseTokenData(compressedAccount.data!.data);
            if (!parsed) {
                throw new Error('Invalid token data');
            }

            return {
                accountInfo,
                merkleContext: {
                    treeInfo: compressedAccount.treeInfo,
                    hash: compressedAccount.hash,
                    leafIndex: compressedAccount.leafIndex,
                    proveByIndex: compressedAccount.proveByIndex,
                },
                parsed: convertTokenDataToAccount(address, parsed),
                isCompressed: true,
            };
        }

        return null;
    }

    if (
        programId.equals(TOKEN_PROGRAM_ID) ||
        programId.equals(TOKEN_2022_PROGRAM_ID)
    ) {
        const info = await rpc.getAccountInfo(address, commitment);
        if (!info) {
            return null;
        }

        const account = splUnpackAccount(address, info, programId);

        return {
            accountInfo: info,
            parsed: account,
            isCompressed: false,
            merkleContext: undefined,
        };
    }

    throw new Error(`Unsupported program ID: ${programId.toBase58()}`);
}
