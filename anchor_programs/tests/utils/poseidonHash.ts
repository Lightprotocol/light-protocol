import { BigNumber } from 'ethers'
const { poseidon } = require('circomlib')

export const poseidonHash = (items: any) =>
  BigNumber.from(poseidon(items).toString())
export const poseidonHash2 = (a: any, b: any) => poseidonHash([a, b])
