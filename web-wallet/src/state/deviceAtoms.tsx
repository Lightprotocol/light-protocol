import { atom } from "jotai";

export const DEVICE_MOBILE_THRESHOLD_WIDTH = 1000;

export const deviceWidthAtom = atom<number>(window.innerWidth);

export const setDeviceWidthAtom = atom(
  (get) => get(deviceWidthAtom),
  (get, set, devicewidth: number) => {
    set(deviceWidthAtom, devicewidth);
  },
);

// derive isMobileAtom from deviceWidthAtom
export const isMobileAtom = atom((get) => {
  const deviceWidth = get(deviceWidthAtom);
  return deviceWidth < DEVICE_MOBILE_THRESHOLD_WIDTH;
});
