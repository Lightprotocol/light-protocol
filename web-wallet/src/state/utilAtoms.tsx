import { atom } from "jotai";
import { PublicKey } from "@solana/web3.js";
import { activeFeeConfigAtom } from "./activeAtoms";
import { isValidPublicKey } from "../util/isAssociatedTokenAccount";

/**
 * verification pda to cache for pending
 */
export const verificationPdaAtom = atom<PublicKey | null>(null);
export const updateVerificationPdaAtom = atom(
  (get) => get(verificationPdaAtom),
  (get, set, newVerificationPda: PublicKey) => {
    //@ts-ignore
    set(verificationPdaAtom, newVerificationPda);
  },
);

/**
 * visual loading indicator state
 */
export const bufferAtom = atom<boolean>(false);

/**
 *  form state to maintain across components
 */
export const formInputAmountAtom = atom<number | string>(0);
export const formInputRecipientAtom = atom<string>("");
// derive baseUnitAmount from formInputAmountAtom
export const baseUnitAmountAtom = atom<number>((get) => {
  let amount = get(formInputAmountAtom);

  let activeFeeConfig = get(activeFeeConfigAtom);

  let baseUnitAmount = Number(amount) * activeFeeConfig.DECIMALS;
  return baseUnitAmount;
});

export type RecipientState = {
  isValidPublicKey: boolean;
  solAddress: PublicKey;
  associatedTokenAccount: PublicKey;
  isInitialized: boolean;
};

// derive recipientStateAtom from formInputRecipientAtom
export const recipientStateAtom = atom<RecipientState>({
  isValidPublicKey: false,
  solAddress: null,
  associatedTokenAccount: null,
  isInitialized: true,
});

export const recipientFieldIsValidAtom = atom<boolean>((get) => {
  let recipientField = get(formInputRecipientAtom);
  return isValidPublicKey(recipientField);
});

/**
 * btn state maintaining across components
 */
export const privacyDetailsAtom = atom<boolean>(false);
export const switchPrivacyDetailsAtom = atom(
  (get) => get(privacyDetailsAtom),
  (get, set) => {
    let current = get(privacyDetailsAtom);
    set(privacyDetailsAtom, !current);
  },
);

export const recipientAtaIsInitializedAtom = atom<boolean>(true);
