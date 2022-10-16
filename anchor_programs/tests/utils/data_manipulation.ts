var ffjavascript = require('ffjavascript')
const { unstringifyBigInts, leInt2Buff, beInt2Buff } = ffjavascript.utils
const { ethers } = require('ethers')
const { BigNumber } = ethers

// TODO functions exists under the name leInt2Buffer verify if same

export const intToBuffer = (hash: any, len = 32) =>
  beInt2Buff(unstringifyBigInts(hash), len)

export const leInt2Buffer = (data: BigInt, bytes = 32) =>
  leInt2Buff(unstringifyBigInts(data), bytes)

export const toBuffer = (value: any, length: any) =>
  Buffer.from(
    BigNumber.from(value)
      .toHexString()
      .slice(2)
      .padStart(length * 2, '0'),
    'hex',
  )
export const toBytes = (string: string) => {
  const buffer = Buffer.from(string, 'utf8')
  const result = Array(buffer.length)
  for (var i = 0; i < buffer.length; i++) {
    result[i] = buffer[i]
  }
  return result
}

/** BigNumber to hex string of specified length */
export const toFixedHex = function (number: any, length: number = 32) {
  let result =
    '0x' +
    (number instanceof Buffer
      ? number.toString('hex')
      : BigNumber.from(number).toHexString().replace('0x', '')
    ).padStart(length * 2, '0')
  if (result.indexOf('-') > -1) {
    result = '-' + result.replace('-', '')
  }
  return result
}
// TODO This is weird update the TS here
export const toUintArray = function (value: any) {
  let buffer
  // console.log(value)
  // @ts-ignore
  if (typeof value !== Buffer) {
    // @ts-ignore
    buffer = Buffer.from(Object.values(value))
    // @ts-ignore
  } else if (typeof value === Uint8Array) {
    buffer = value
    return value
  } else {
    // @ts-ignore
    buffer = Buffer.from(value)
  }
  const asArr = new Uint8Array(buffer.length)
  asArr.set(buffer)
  return asArr
}

export function parseSol({
  amount,
  digits = 1e9,
}: {
  amount: number
  digits: number
}) {
  let x = BigNumber.from(amount).mul(BigNumber.from(digits))
  return x.toString()
}

export async function parseInputsToBytes(data: any) {
  var mydata = JSON.parse(data.toString())

  for (var i in mydata) {
    mydata[i] = leInt2Buff(unstringifyBigInts(mydata[i]), 32).toString()
  }
  return mydata
}

export const parseInputsToBytesArray = async function (data: any) {
  var mydata = JSON.parse(data.toString())

  for (var i in mydata) {
    mydata[i] = leInt2Buff(unstringifyBigInts(mydata[i]), 32)
  }
  let x: any[] = []
  mydata.map((array: any) => {
    array.map((byte: any) => {
      x.push(byte)
    })
  })
  return x
}

export function toHexString(byteArray: any) {
  return Array.from(byteArray, function (byte: any) {
    return ('0' + (byte & 0xff).toString(16)).slice(-2)
  }).join('')
}

export const parseProofToBytesArray = async function (data: any) {
  var mydata = JSON.parse(data.toString())

  for (var i in mydata) {
    if (i == 'pi_a') {
      for (var j in mydata[i]) {
        mydata[i][j] = leInt2Buff(
          unstringifyBigInts(mydata[i][j]),
          32, // 48
        )
      }
    } else if (i == 'pi_b') {
      for (var j in mydata[i]) {
        for (var z in mydata[i][j]) {
          mydata[i][j][z] = leInt2Buff(
            unstringifyBigInts(mydata[i][j][z]),
            32, // 48
          )
        }
      }
    } else if (i == 'pi_c') {
      for (var j in mydata[i]) {
        mydata[i][j] = leInt2Buff(
          unstringifyBigInts(mydata[i][j]),
          32, //48
        )
      }
    }
  }
  let mydataStripped = [
    mydata.pi_a[0],
    mydata.pi_a[1],
    mydata.pi_b[0],
    mydata.pi_b[1],
    mydata.pi_c[0],
    mydata.pi_c[1],
  ]
  var merged = [].concat.apply([], mydataStripped)
  let x: any = []
  merged.map((array: any) => {
    array.map((byte: any) => {
      x.push(byte)
    })
  })

  return x
}
