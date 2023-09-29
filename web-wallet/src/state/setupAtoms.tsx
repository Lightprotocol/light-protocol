import { atom } from "jotai";
import { isLoggedInAtom, isRegisteredAtom } from "./userAtoms";

export enum SetupStep {
  DERIVE_ACCOUNT = 1,
  STORE_ACCOUNT = 2,
  DONE = 3,
}
export const setupStepAtom = atom<SetupStep>(SetupStep.DERIVE_ACCOUNT);

export const fetchedSetupStepAtom = atom(
  (get) => get(setupStepAtom),
  async (get, set) => {
    let isLoggedIn = get(isLoggedInAtom);
    let isRegistered = get(isRegisteredAtom);

    if (isLoggedIn && isRegistered) {
      set(setupStepAtom, SetupStep.DONE);
    } else if (isLoggedIn && !isRegistered) {
      set(setupStepAtom, SetupStep.STORE_ACCOUNT);
    } else {
      set(setupStepAtom, SetupStep.DERIVE_ACCOUNT);
    }
  },
);

export const nextSetupStepAtom = atom(null, async (get, set) => {
  let currentStep = get(setupStepAtom);
  if (currentStep === SetupStep.DERIVE_ACCOUNT) {
    set(setupStepAtom, SetupStep.STORE_ACCOUNT);
  } else if (currentStep === SetupStep.STORE_ACCOUNT) {
    set(setupStepAtom, SetupStep.DONE);
  }
});

export const currentSetupStepAtom = atom((get) => {
  let isLoggedIn = get(isLoggedInAtom);
  let isRegistered = get(isRegisteredAtom);
  if (isLoggedIn && isRegistered) return SetupStep.DONE;
  if (isLoggedIn && !isRegistered) return SetupStep.STORE_ACCOUNT;
  if (!isLoggedIn && !isRegistered) return SetupStep.DERIVE_ACCOUNT;
});

export const isSetupDoneAtom = atom((get) => {
  let isLoggedIn = get(isLoggedInAtom);
  let isRegistered = get(isRegisteredAtom);
  return isLoggedIn && isRegistered;
});
