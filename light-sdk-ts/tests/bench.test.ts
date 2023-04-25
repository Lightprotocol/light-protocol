import { expect } from 'chai';
import 'mocha';

interface Fruit {
  id: string;
  name: string;
  quantity: number;
}

function generateFruitArray(n: number): Fruit[] {
  const fruits: Fruit[] = [];

  for (let i = 1; i <= n; i++) {
    fruits.push({ id: i.toString(), name: `fruit${i}`, quantity: i * 5 });
  }

  return fruits;
}

function createLookup(fruitArray: Fruit[]): Map<string, Fruit> {
    const lookup = new Map<string, Fruit>();
  
    for (const fruit of fruitArray) {
      lookup.set(fruit.id, fruit);
    }
  
    return lookup;
  }

describe('Lookup vs Array.find() Benchmark', function () {
  this.timeout(10000); // Set a longer timeout for the benchmark tests

  for (let n = 1; n <= 10000; n++) {
    it(`Benchmark with ${n} entries`, () => {
      const fruitArray = generateFruitArray(n);
      const fruitLookup = createLookup(fruitArray);

      const lookupStartTime = Date.now();
      console.time("record")
      const fruitIdToFind = (n / 2).toString();
      const foundByLookup = fruitLookup.get(fruitIdToFind);
      console.timeEnd("record")

      const lookupEndTime = Date.now();

      const findStartTime = Date.now();
      console.time("find")

      const foundByFind = fruitArray.find(fruit => fruit.id === fruitIdToFind);
      console.timeEnd("find")

      const findEndTime = Date.now();

      const lookupDuration = lookupEndTime - lookupStartTime;
      const findDuration = findEndTime - findStartTime;

    //   console.log(`Lookup time (${n} entries): ${lookupDuration} ms`);
    //   console.log(`Array.find() time (${n} entries): ${findDuration} ms`);

    //   expect(lookupDuration).to.be.at.most(findDuration);
    });
  }
  let n = 1;
  it(`Benchmark with ${n} entries`, () => {
    const fruitArray = generateFruitArray(n);
    const fruitLookup = createLookup(fruitArray);

    const lookupStartTime = Date.now();
    const fruitIdToFind = (n / 2).toString();
    const foundByLookup = fruitLookup.get(fruitIdToFind);
    const lookupEndTime = Date.now();

    const findStartTime = Date.now();
    const foundByFind = fruitArray.find(fruit => fruit.id === fruitIdToFind);
    const findEndTime = Date.now();

    const lookupDuration = lookupEndTime - lookupStartTime;
    const findDuration = findEndTime - findStartTime;

    console.log(`Lookup time (${n} entries): ${lookupDuration} ms`);
    console.log(`Array.find() time (${n} entries): ${findDuration} ms`);

    expect(lookupDuration).to.be.at.most(findDuration);
  });
});
