import { atom } from "jotai";

export type NavigationState = {
  status: NavigationStatus;
  action: Action;
  processingError: boolean;
};

export enum Action {
  SHIELD = "SHIELD",
  UNSHIELD = "UNSHIELD",
}
export enum NavigationStatus {
  IDLE = "IDLE",
  REGISTER = "REGISTER",
  SIGNING = "SIGNING",
  PREPARING = "PREPARING",
  PROCESSING = "PROCESSING",
  DONE = "DONE",
}

export const navigationAtom = atom<NavigationState>({
  status: NavigationStatus.IDLE,
  action: Action.SHIELD,
  processingError: false,
});

export const updateNavigationAtom = atom(
  (get) => get(navigationAtom),
  (get, set, newNavigation: NavigationState) => {
    set(navigationAtom, newNavigation);
  },
);

export const startSignAtom = atom(
  (get) => get(navigationAtom),
  (get, set) => {
    let currentNavigation = get(navigationAtom);
    set(navigationAtom, {
      ...currentNavigation,
      status: NavigationStatus.SIGNING,
    });
  },
);

export const startPrepareAtom = atom(
  (get) => get(navigationAtom),
  (get, set) => {
    let currentNavigation = get(navigationAtom);
    set(navigationAtom, {
      ...currentNavigation,
      status: NavigationStatus.PREPARING,
    });
  },
);

export const startProcessAtom = atom(
  (get) => get(navigationAtom),
  (get, set) => {
    let currentNavigation = get(navigationAtom);
    set(navigationAtom, {
      ...currentNavigation,
      status: NavigationStatus.PROCESSING,
    });
  },
);

export const setProcessDoneAtom = atom(
  (get) => get(navigationAtom),
  (get, set) => {
    let currentNavigation = get(navigationAtom);
    set(navigationAtom, {
      ...currentNavigation,
      status: NavigationStatus.DONE,
    });
  },
);

export const updateActionAtom = atom(
  (get) => get(navigationAtom),
  (get, set, newAction: Action) => {
    let currentNavigation = get(navigationAtom);
    set(navigationAtom, { ...currentNavigation, action: newAction });
  },
);

export const setProcessingErrorAtom = atom(
  (get) => get(navigationAtom),
  (get, set) => {
    let currentNavigation = get(navigationAtom);
    set(navigationAtom, {
      ...currentNavigation,
      processingError: true,
    });
  },
);

export const resolveProcessingErrorAtom = atom(
  (get) => get(navigationAtom),
  (get, set) => {
    let currentNavigation = get(navigationAtom);
    set(navigationAtom, {
      ...currentNavigation,
      processingError: false,
    });
  },
);

export const cancelActionAtom = atom(
  (get) => get(navigationAtom),
  (get, set) => {
    let currentAction = get(navigationAtom).action;
    set(navigationAtom, {
      action: currentAction,
      status: NavigationStatus.IDLE,
      processingError: false,
    });
  },
);

export const setIdleAtom = atom(
  (get) => get(navigationAtom),
  (get, set) => {
    let currentAction = get(navigationAtom).action;

    set(navigationAtom, {
      action: currentAction,
      status: NavigationStatus.IDLE,
      processingError: false,
    });
  },
);

// derive processStateAtom from navigationAtom
export const processStatusAtom = atom((get) => {
  let status = get(navigationAtom).status;
  return status;
});

// derive processingErrorAtom from navigationAtom
export const processingErrorAtom = atom((get) => {
  let processingError = get(navigationAtom).processingError;
  return processingError;
});

// derive isLoading from navigationAtom
export const isInProgressAtom = atom((get) => {
  let status = get(navigationAtom).status;
  return (
    status !== NavigationStatus.IDLE &&
    // status !== NavigationStatus.DONE &&
    status !== NavigationStatus.REGISTER
  );
});

// derive pendingActionAtom from navigationAtom
export const actionIsPendingAtom = atom((get) => {
  let status = get(navigationAtom).status;
  return (
    status === NavigationStatus.PREPARING ||
    status === NavigationStatus.PROCESSING
  );
});
