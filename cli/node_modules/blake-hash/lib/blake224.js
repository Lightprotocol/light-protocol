const Blake256 = require('./blake256')

const zo = Buffer.from([0x00])
const oo = Buffer.from([0x80])

module.exports = class Blake224 extends Blake256 {
  constructor () {
    super()

    this._h = [
      0xc1059ed8, 0x367cd507, 0x3070dd17, 0xf70e5939,
      0xffc00b31, 0x68581511, 0x64f98fa7, 0xbefa4fa4
    ]

    this._zo = zo
    this._oo = oo
  }

  digest () {
    this._padding()

    const buffer = Buffer.alloc(28)
    for (let i = 0; i < 7; ++i) buffer.writeUInt32BE(this._h[i], i * 4)
    return buffer
  }
}
