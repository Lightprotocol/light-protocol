import { atom } from "jotai";

export type LightError = {
  message: string;
  isError: boolean;
};
export const errorAtom = atom<LightError>({ message: "", isError: false });

export const setErrorAtom = atom(
  (get) => get(errorAtom),
  (get, set, message: string) => {
    set(errorAtom, { message, isError: true });
  },
);

export const resolveErrorAtom = atom(
  (get) => get(errorAtom),
  (get, set) => {
    set(errorAtom, { message: "", isError: false });
  },
);
