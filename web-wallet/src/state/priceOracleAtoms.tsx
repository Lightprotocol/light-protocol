import axios from "axios";
import { atom } from "jotai";

export type Prices = {
  usdPerSol: number;
  usdPerUsdc: number;
};

export const priceOracleAtom = atom<Prices>({
  usdPerSol: 0,
  usdPerUsdc: 0,
});

export const updatePriceOracleAtom = atom(
  (get) => get(priceOracleAtom), // Getter returns the current userAtom state
  async (get, set) => {
    let recentPrices = get(priceOracleAtom);
    let res =
      await axios.get(`https://api.coingecko.com/api/v3/simple/price?ids=solana,usd-coin&vs_currencies=usd
  `);
    if (!res.data) {
      console.error("coingecko - Could not fetch price");
      return;
    }
    const newPrices = {
      usdPerSol: Number(`${res.data["solana"].usd}`),
      usdPerUsdc: Number(`${res.data["usd-coin"].usd}`),
    };
    if (newPrices !== recentPrices) set(priceOracleAtom, newPrices);
  },
);
