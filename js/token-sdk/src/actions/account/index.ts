/**
 * Account management actions for Light Token SDK.
 */

export {
    createAssociatedTokenAccountInstruction,
    createAssociatedTokenAccountIdempotentInstruction,
    type CreateAtaParams,
    type CreateAtaResult,
} from './create-ata.js';

export {
    createCloseAccountInstruction,
    type CloseAccountParams,
} from './close.js';
