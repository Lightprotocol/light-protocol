const Blake = require('./blake')

module.exports = (engines) => {
  const getEngine = (algorithm) => {
    const hash = typeof algorithm === 'string' ? algorithm.toLowerCase() : algorithm
    switch (hash) {
      case 'blake224': return engines.Blake224
      case 'blake256': return engines.Blake256
      case 'blake384': return engines.Blake384
      case 'blake512': return engines.Blake512

      default: throw new Error('Invald algorithm: ' + algorithm)
    }
  }

  return (algorithm, options) => {
    const Engine = getEngine(algorithm)
    return new Blake(new Engine(), options)
  }
}
