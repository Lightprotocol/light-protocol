import {
  Connection,
  PublicKey,
  Keypair as SolanaKeypair,
} from "@solana/web3.js";
import { atom } from "jotai";
import { Keypair, KeypairLegacy } from "../sdk/src/keypair";
import {
  BLACKLIST,
  PROGRAM_ID,
  RELAYER_URL,
  Token,
  USER_ACCOUNT_PUBLICKEY_OFFSET,
} from "../constants";
import { deriveKeypairsFromSignature } from "../util/keyDerivation";
import axios from "axios";
import { userUtxosAtom } from "./userUtxoAtoms";
import { Action, NavigationStatus, navigationAtom } from "./navigationAtoms";
import { transactionsAtom } from "./transactionsAtoms";
import { PublicBalance, publicBalancesAtom } from "./balancesAtoms";
import { setupStepAtom } from "./setupAtoms";
import { SetupStep } from "./setupAtoms";

export type User = {
  publicBalances: PublicBalance[];
  account: PublicKey | null;
  keypairs: UserKeypairs;
  connectedWallet: PublicKey | null;
};
export type UserKeypairs = {
  spendingKeypair: Keypair | null;
  burnerKeypair: SolanaKeypair | null;
  viewingKeypair: nacl.BoxKeyPair | null;
  spendingKeypairLegacy: KeypairLegacy | null;
  burnerKeypairLegacy: SolanaKeypair | null;
  viewingKeypairLegacy: nacl.BoxKeyPair | null;
  localStorageEncryptionKeypair: nacl.BoxKeyPair | null;
};

export const userAccountAtom = atom<PublicKey | null>(null);
export const isFetchingUserAccountAtom = atom(false);

export const userCreationBlockTimeAtom = atom<{
  slot: number;
  blockTime: number;
}>({
  slot: 0,
  blockTime: 0,
});

export const fetchedUserAccountAtom = atom(
  (get) => get(userAccountAtom),
  async (get, set, connection: Connection) => {
    set(isFetchingUserAccountAtom, true);
    let publicKey: PublicKey = get(walletConnectionAtom);
    let userAccount = await connection.getProgramAccounts(PROGRAM_ID, {
      filters: [
        {
          memcmp: {
            offset: USER_ACCOUNT_PUBLICKEY_OFFSET,
            bytes: publicKey.toBase58(),
          },
        },
      ],
      commitment: "confirmed",
    });
    if (userAccount.length === 0) {
      set(isFetchingUserAccountAtom, false);
      //@ts-ignore
      set(userAccountAtom, null);
      return;
    }
    console.log("@@userAccount?", userAccount[0].pubkey.toBase58());

    const userAccountPublicKey = userAccount[0].pubkey;
    //@ts-ignore
    set(userAccountAtom, userAccountPublicKey);
    set(isFetchingUserAccountAtom, false);

    // get the last transaction where the user account was involved
    console.time("getConfirmedSignaturesForAddress2");
    let transactions = await connection.getConfirmedSignaturesForAddress2(
      userAccountPublicKey,
      {
        limit: 1,
      },
    );
    console.log("@@user acc creation transaction", transactions);
    if (transactions.length === 0) return;
    let blockTime = transactions[0].blockTime;
    let slot = transactions[0].slot;
    console.timeEnd("getConfirmedSignaturesForAddress2");
    set(userCreationBlockTimeAtom, { slot, blockTime });
  },
);

export const userKeypairsAtom = atom<UserKeypairs>({
  spendingKeypair: null,
  burnerKeypair: null,
  viewingKeypair: null,
  spendingKeypairLegacy: null,
  burnerKeypairLegacy: null,
  viewingKeypairLegacy: null,
  localStorageEncryptionKeypair: null,
});

export const derivedUserKeypairsAtom = atom(
  (get) => get(userKeypairsAtom),
  (get, set, signature: Uint8Array) => {
    let keys = deriveKeypairsFromSignature(signature);
    const newKeypairs: UserKeypairs = {
      spendingKeypair: keys.spendingKeypair,
      burnerKeypair: keys.burnerKeypair,
      viewingKeypair: keys.viewingKeypair,
      spendingKeypairLegacy: keys.shieldedKeypairLegacy,
      burnerKeypairLegacy: keys.burnerKeypairLegacy,
      viewingKeypairLegacy: keys.encryptionKeypairLegacy,
      localStorageEncryptionKeypair: keys.localStorageEncryptionKeypair,
    };
    set(userKeypairsAtom, newKeypairs);
  },
);

// TODO: do we need this derivate?
export const userAtom = atom<User>((get) => {
  let keypairs = get(userKeypairsAtom);
  let account = get(userAccountAtom);
  let connectedWallet = get(walletConnectionAtom);
  let publicBalances = get(publicBalancesAtom);
  return { connectedWallet, keypairs, account, publicBalances };
});

export const walletConnectionAtom = atom<PublicKey | null>(null);

// connectWalletAtom derive from walletConnectionAtom
export const connectWalletAtom = atom(
  (get) => get(walletConnectionAtom),
  (get, set, publicKey) => {
    //@ts-ignore
    set(walletConnectionAtom, publicKey);
  },
);

export const disconnectWalletAtom = atom(
  (get) => get(walletConnectionAtom),
  (get, set) => {
    //@ts-ignore
    set(walletConnectionAtom, null);
  },
);

export const isWalletConnectedAtom = atom((get) => {
  let walletConnection = get(walletConnectionAtom);
  return walletConnection !== null;
});

export const isLoggedInAtom = atom((get) => {
  let keypairs = get(userKeypairsAtom);
  return keypairs.spendingKeypair !== null;
});

export const isRegisteredAtom = atom((get) => {
  let account = get(userAccountAtom);
  // let isSetupDone = get(isSetupDoneAtom);
  return account !== null;
});

export const isBlockedAtom = atom<boolean>(false);

export const checkedIsBlockedAtom = atom(
  (get) => get(isBlockedAtom),
  async (get, set) => {
    let solBalancePublicKey = get(walletConnectionAtom);
    if (!solBalancePublicKey) return false;
    let isBlocked = BLACKLIST.includes(solBalancePublicKey.toBase58());

    if (!isBlocked) {
      let res = await axios.get(`${RELAYER_URL}/checkaddress`);
      if (!res.data) return console.error("Error checking address");
      if (res.data.status === 1) {
        isBlocked = true;
      }
      console.log("Address checked");
    }
    set(isBlockedAtom, isBlocked);
  },
);

// TODO : test this
export const resetUserStateAtom = atom(null, (get, set) => {
  set(userUtxosAtom, []);
  set(userKeypairsAtom, {
    spendingKeypair: null,
    burnerKeypair: null,
    viewingKeypair: null,
    spendingKeypairLegacy: null,
    burnerKeypairLegacy: null,
    viewingKeypairLegacy: null,
    localStorageEncryptionKeypair: null,
  });
  //@ts-ignore
  set(userAccountAtom, null);
  set(navigationAtom, {
    status: NavigationStatus.IDLE,
    action: Action.SHIELD,
    processingError: false,
  });
  set(setupStepAtom, SetupStep.DERIVE_ACCOUNT);
  set(publicBalancesAtom, []);
  set(transactionsAtom, []);
  //@ts-ignore
  set(walletConnectionAtom, null);
});
