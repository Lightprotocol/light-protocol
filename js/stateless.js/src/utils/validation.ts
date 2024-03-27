import { BN } from '@coral-xyz/anchor';
import {
  CompressedAccount,
  CompressedAccountWithMerkleContext,
  bn,
} from '../state';

export const validateSufficientBalance = (balance: BN) => {
  if (balance.lt(bn(0))) {
    throw new Error('Not enough balance for transfer');
  }
};

export const validateSameOwner = (
  compressedAccounts:
    | CompressedAccount[]
    | CompressedAccountWithMerkleContext[],
) => {
  if (compressedAccounts.length === 0) {
    throw new Error('No accounts provided for validation');
  }
  const zerothOwner = compressedAccounts[0].owner;
  if (!compressedAccounts.every(account => account.owner.equals(zerothOwner))) {
    throw new Error('All input accounts must have the same owner');
  }
};
