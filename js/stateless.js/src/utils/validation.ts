import BN from 'bn.js';
import {
    CompressedAccountLegacy,
    CompressedAccountWithMerkleContext,
    bn,
} from '../state';

export const validateSufficientBalance = (balance: BN) => {
    if (balance.lt(bn(0))) {
        throw new Error('Insufficient balance for transfer');
    }
};

export const validateSameOwner = (
    compressedAccounts:
        | CompressedAccountLegacy[]
        | CompressedAccountWithMerkleContext[],
) => {
    if (compressedAccounts.length === 0) {
        throw new Error('No accounts provided for validation');
    }
    const zerothOwner = compressedAccounts[0].owner;
    if (
        !compressedAccounts.every(account => account.owner.equals(zerothOwner))
    ) {
        throw new Error('All input accounts must have the same owner');
    }
};

/// Client-side pre-flight validation for proof requests.
/// V1 inclusion: {1, 2, 3, 4, 8}, V2 inclusion: {1..8}.
/// Combined proofs (hashes + addresses): max 4 hashes for both V1 and V2.
export const validateNumbersForProof = (
    hashesLength: number,
    newAddressesLength: number,
) => {
    if (hashesLength > 0 && newAddressesLength > 0) {
        // Combined circuits (V1 and V2) support max 4 hashes.
        if (hashesLength > 4) {
            throw new Error(
                `Invalid number of compressed accounts for combined proof: ${hashesLength}. Allowed: 1-4`,
            );
        }
        validateNumbers(hashesLength, [1, 2, 3, 4], 'compressed accounts');
        validateNumbersForNonInclusionProof(newAddressesLength);
    } else {
        if (hashesLength > 0) {
            validateNumbersForInclusionProof(hashesLength);
        } else {
            validateNumbersForNonInclusionProof(newAddressesLength);
        }
    }
};

/// Validate inclusion proof input count.
/// Accepts 1-8 (union of V1 {1,2,3,4,8} and V2 {1..8}).
/// Version-specific validation happens in the chunking layer.
export const validateNumbersForInclusionProof = (hashesLength: number) => {
    validateNumbers(
        hashesLength,
        [1, 2, 3, 4, 5, 6, 7, 8],
        'compressed accounts',
    );
};

/// Ensure that the amount if new addresses is allowed.
export const validateNumbersForNonInclusionProof = (
    newAddressesLength: number,
) => {
    validateNumbers(newAddressesLength, [1, 2], 'new addresses');
};

/// V1 circuit safeguards.
export const validateNumbers = (
    length: number,
    allowedNumbers: number[],
    type: string,
) => {
    if (!allowedNumbers.includes(length)) {
        throw new Error(
            `Invalid number of ${type}: ${length}. Allowed numbers: ${allowedNumbers.join(', ')}`,
        );
    }
};
