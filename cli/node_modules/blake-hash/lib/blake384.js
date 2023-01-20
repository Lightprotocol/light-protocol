const Blake512 = require('./blake512')

const zo = Buffer.from([0x00])
const oo = Buffer.from([0x80])

module.exports = class Blake384 extends Blake512 {
  constructor () {
    super()

    this._h = [
      0xcbbb9d5d, 0xc1059ed8, 0x629a292a, 0x367cd507,
      0x9159015a, 0x3070dd17, 0x152fecd8, 0xf70e5939,
      0x67332667, 0xffc00b31, 0x8eb44a87, 0x68581511,
      0xdb0c2e0d, 0x64f98fa7, 0x47b5481d, 0xbefa4fa4
    ]

    this._zo = zo
    this._oo = oo
  }

  digest () {
    this._padding()

    const buffer = Buffer.alloc(48)
    for (let i = 0; i < 12; ++i) buffer.writeUInt32BE(this._h[i], i * 4)
    return buffer
  }
}
