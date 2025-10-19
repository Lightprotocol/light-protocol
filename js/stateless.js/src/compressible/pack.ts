import {
    PublicKey,
    AccountMeta,
    KeyedAccountInfo,
    AccountInfo,
} from '@solana/web3.js';
import { PackedStateTreeInfo } from '../state/compressed-account';
import { CompressedProof, TreeInfo } from '../state/types';
import { ValidityProof } from '../state/types';
import { CTOKEN_PROGRAM_ID } from '../constants';
import BN from 'bn.js';
import {
    createPackedAccountsSmall,
    createPackedAccountsSmallWithCpiContext,
} from '../utils';
import { ValidityProofWithContext } from '../rpc-interface';
import { packTreeInfos, ValidityProofWithContextV2 } from '../programs';
import { Buffer } from 'buffer';

// Type to transform PublicKey fields to numbers recursively
export type PackedType<T> = T extends PublicKey
    ? number
    : T extends BN
      ? BN
      : T extends (infer U)[]
        ? PackedType<U>[]
        : T extends object
          ? { [K in keyof T]: PackedType<T[K]> }
          : T;

/**
 * Recursively replaces all PublicKey instances with their packed index.
 * Leaves all other values untouched.
 */
export function packWithAccounts<TInput, TOutput = PackedType<TInput>>(
    obj: TInput,
    packedAccounts: ReturnType<typeof createPackedAccountsSmall>,
): TOutput {
    if (obj instanceof PublicKey) {
        return packedAccounts.insertOrGet(obj) as TOutput;
    }
    if (typeof obj === 'bigint') {
        return new BN(obj.toString()) as TOutput;
    }
    if (obj instanceof BN) {
        return obj as TOutput;
    }
    if (Array.isArray(obj)) {
        return obj.map(item =>
            packWithAccounts(item, packedAccounts),
        ) as TOutput;
    }
    if (obj !== null && typeof obj === 'object') {
        const result: any = Array.isArray(obj) ? [] : {};
        for (const key in obj) {
            if (Object.prototype.hasOwnProperty.call(obj, key)) {
                const value = (obj as any)[key];
                result[key] = packWithAccounts(value, packedAccounts);
            }
        }
        return result as TOutput;
    }
    return obj as unknown as TOutput;
}

/**
 * Builds compressed account metas. Returns instruction data params and
 * remaining accounts to add to your instruction. Note,this assumes the standard
 * implemention of the decompressAccountsIdempotent instruction.
 *
 * @param programId                 Program ID
 * @param validityProofWithContext  Validity proof with context
 * @param compressedAccounts        Compressed accounts
 * @param decompressedAccountAddresses Decompressed account addresses
 * @returns                         Compressed account metas, system accounts offset, proof option, and
 * remaining accounts
 */
export async function packDecompressAccountsIdempotent(
    programId: PublicKey,
    validityProofWithContext: ValidityProofWithContext,
    compressedAccounts: {
        key: string;
        treeInfo: TreeInfo;
        data: any;
    }[],
    decompressedAccountAddresses: PublicKey[],
): Promise<{
    compressedAccounts: {
        meta: {
            treeInfo: PackedStateTreeInfo;
            outputStateTreeIndex: number;
        };
        data: any;
    }[];
    systemAccountsOffset: number;
    proofOption: { 0: ValidityProof | null };
    remainingAccounts: AccountMeta[];
}> {
    let hasPdas = false;
    let hasTokens = false;
    for (const account of compressedAccounts) {
        if (account.key === 'cTokenData') {
            hasTokens = true;
        } else {
            hasPdas = true;
        }

        if (hasPdas && hasTokens) {
            break;
        }
    }
    const foundCpiContext = compressedAccounts.find(
        account =>
            account.treeInfo.cpiContext !== null &&
            account.treeInfo.cpiContext !== undefined,
    )?.treeInfo.cpiContext;
    if (hasPdas && hasTokens && !foundCpiContext) {
        throw new Error('No cpi context found in compressed accounts');
    }
    const remainingAccounts =
        hasPdas && hasTokens
            ? createPackedAccountsSmallWithCpiContext(
                  programId,
                  foundCpiContext!,
              )
            : createPackedAccountsSmall(programId);

    const outputQueue = compressedAccounts[0].treeInfo.nextTreeInfo
        ? compressedAccounts[0].treeInfo.nextTreeInfo.queue
        : compressedAccounts[0].treeInfo.queue;

    const _ = remainingAccounts.insertOrGet(outputQueue);
    const packedTreeInfos = packTreeInfos(
        validityProofWithContext,
        remainingAccounts,
    );

    const compressedAccountData: {
        meta: {
            treeInfo: PackedStateTreeInfo;
            outputStateTreeIndex: number;
        };
        data: any;
    }[] = compressedAccounts.map(({ data }, index) => {
        const packedData = packWithAccounts(data, remainingAccounts);
        if (!packedTreeInfos.stateTrees) {
            throw new Error(
                'No state trees found in passed ValidityproofWithContext instance',
            );
        }

        return {
            meta: {
                treeInfo: packedTreeInfos.stateTrees.packedTreeInfos[index],
                outputStateTreeIndex:
                    packedTreeInfos.stateTrees.outputTreeIndex,
            },
            data: {
                ['packed' +
                compressedAccounts[index].key[0].toUpperCase() +
                compressedAccounts[index].key.slice(1)]: [packedData],
            },
        };
    });
    const { remainingAccounts: remainingAccountMetas, systemStart } =
        remainingAccounts.toAccountMetas();
    if (compressedAccounts.length !== decompressedAccountAddresses.length) {
        throw new Error(
            'Compressed accounts and decompressed account addresses must have the same length',
        );
    }

    // Add solana target accounts
    for (const account of decompressedAccountAddresses) {
        remainingAccountMetas.push({
            pubkey: account,
            isSigner: false,
            isWritable: true,
        });
    }

    return {
        compressedAccounts: compressedAccountData,
        systemAccountsOffset: systemStart,
        remainingAccounts: remainingAccountMetas,
        proofOption: { 0: validityProofWithContext.compressedProof },
    };
}

type KeyedParsedAccountInfo<T> = {
    accountId: PublicKey;
    accountInfo: AccountInfo<Buffer>;
    parsed: T;
};
/**
 * Pack remaining accounts for compressAccountsIdempotent Returns instruction
 * data params and remaining accounts to add to your instruction. Note,this
 * assumes the standard implemention of the decompressAccountsIdempotent
 * instruction.
 *
 * @param programId                 Program ID
 * @param validityProofWithContext  Validity proof with context
 * @param accountsToCompress        AccountInfo + address of onchain acccount to
 *                                  compress.
 * @param outputStateTreeInfo       Output state tree info
 * @returns                         Compressed account metas, system accounts
 *                                  offset, proof option, and remaining accounts
 */
export async function packCompressAccountsIdempotent(
    programId: PublicKey,
    validityProofWithContext: ValidityProofWithContext,
    accountsToCompress: KeyedParsedAccountInfo<any>[],
    outputStateTreeInfo: TreeInfo,
): Promise<{
    compressedAccountMetas: {
        treeInfo: PackedStateTreeInfo;
        outputStateTreeIndex: number;
    }[];
    systemAccountsOffset: number;
    proofOption: { 0: ValidityProof | null };
    remainingAccounts: AccountMeta[];
}> {
    const remainingAccounts = createPackedAccountsSmall(programId);

    // Ensure output queue is present; offset is relative to remaining accounts
    const _ = remainingAccounts.insertOrGet(outputStateTreeInfo.queue);

    const packedTreeInfos = packTreeInfos(
        validityProofWithContext,
        remainingAccounts,
    );

    const compressedAccountMetas =
        packedTreeInfos.stateTrees!.packedTreeInfos.map(pti => ({
            treeInfo: pti,
            outputStateTreeIndex: packedTreeInfos.stateTrees!.outputTreeIndex,
        }));

    const { remainingAccounts: remainingAccountMetas, systemStart } =
        remainingAccounts.toAccountMetas();

    for (const keyedAccountInfo of accountsToCompress) {
        if (
            new PublicKey(keyedAccountInfo.accountInfo.owner).equals(
                new PublicKey(CTOKEN_PROGRAM_ID),
            )
        ) {
            const mint = new PublicKey(
                Array.from(keyedAccountInfo.accountInfo.data.slice(0, 32)),
            );
            // readonly
            remainingAccountMetas.push({
                pubkey: mint,
                isSigner: false,
                isWritable: false,
            });

            const owner = new PublicKey(
                keyedAccountInfo.accountInfo.data.slice(32, 64),
            );
            remainingAccountMetas.push({
                pubkey: owner,
                isSigner: false,
                isWritable: false,
            });
        }
    }
    for (const keyedAccountInfo of accountsToCompress) {
        const pubkey = keyedAccountInfo.accountId;
        remainingAccountMetas.push({
            pubkey,
            isSigner: false,
            isWritable: true,
        });
    }

    return {
        compressedAccountMetas,
        systemAccountsOffset: systemStart,
        proofOption: { 0: validityProofWithContext.compressedProof },
        remainingAccounts: remainingAccountMetas,
    };
}
