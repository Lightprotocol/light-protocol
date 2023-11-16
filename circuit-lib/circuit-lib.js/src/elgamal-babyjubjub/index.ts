export * from "./elgamal";
export * from "./pointEncoding";
/// we're not exporting the `precompute` module by default, because it's a node only process.
/// that's fine because precompute is only consumed in elgamal.test.ts
/// we may change this when migrating to a bundler for zk.js
