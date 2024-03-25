export * from './conversion';
export * from './pipe';
export * from './airdrop';

export const sleep = (ms: number) =>
  new Promise((resolve) => setTimeout(resolve, ms));
