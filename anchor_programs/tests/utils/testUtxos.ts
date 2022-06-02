import { BigNumber } from 'ethers'
const light = require('../../light-protocol-sdk');
const { U64 } = require('n64')
// import { RELAYER_ADDRESS, RPC_URL } from '../../src/constants'
import nacl from 'tweetnacl'
import { Connection, PublicKey } from '@solana/web3.js'

const createEncryptionKeypair = () => nacl.box.keyPair()
export const encryptionKeypair = createEncryptionKeypair()
// export const connection = new Connection(RPC_URL)
export const privkey =
  process.env.ACCOUNT_1_SHIELDED_PRIVATE_KEY
export const newKeypair = new light.Keypair();
export const shieldedKeyPair = newKeypair // is this the same? In the console it is...
export const amount = 43620599
export const inputUtxoAmount = 44620599
export const testTimeout = Date.now() + 50000
export const testInputUtxo = new light.Utxo(
  inputUtxoAmount,
  newKeypair,
  BigNumber.from(
    '0xf7e9b700d583b97a58b3819f647b80fa3dc7ed6ad169cf6b34db0d5cd1e96f',
  ),
  780,
)
export const relayerFee = U64('5379401')
export const outputUtxoAmount = 38241198
export const testOutputUtxo = new light.Utxo(outputUtxoAmount, newKeypair)
export const publicKey = new PublicKey(
  'FXNMiLfgo1wBKGFiGv8DstLg1pGzKc6dkzRQrAKuFTnC',
)
export const token = 'SOL'
export const externalAmountBigNumber = BigNumber.from('-0x0f4240')
export const recipient = 'FXNMiLfgo1wBKGFiGv8DstLg1pGzKc6dkzRQrAKuFTnC'
export const relayer = 'FXNMiLfgo1wBKGFiGv8DstLg1pGzKc6dkzRQrAKuFTnC'
export const action = 'deposit'
export const testUuid = '0x0000000000000000000000000'
