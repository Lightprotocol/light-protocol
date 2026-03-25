import { createLoadAtaInstructions } from '@lightprotocol/compressed-token';
import type { PublicKey, TransactionInstruction } from '@solana/web3.js';
import { createSingleCompressedAccountRpc, getAtaOrNull } from './account';
import { normalizeInstructionBatches, toInterfaceOptions } from './helpers';
import { getAtaAddress } from './read';
import type {
    CreateLoadInstructionsInput,
    TokenInterfaceAccount,
} from './types';

interface CreateLoadInstructionInternalInput extends CreateLoadInstructionsInput {
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
        'createLoadInstruction',
        await createLoadAtaInstructions(
            effectiveRpc,
            targetAta,
            owner,
            mint,
            payer,
            toInterfaceOptions(owner, authority, wrap),
        ),
    );

    if (instructions.length === 0) {
        return null;
    }

    return {
        instructions,
    };
}
