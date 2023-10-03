## Benchmark Results
***iterations** = 2000*

| Height | Capacity  | Average Proving Time |
|--------|---------- |------------------------|
| 18     | 262,144   | 35ms/op                |
| 20     | 1,048,576 | 40ms/op                |
| 22     | 4,194,304 | 44ms/op                |
| 24     | 16,777,216| 48ms/op                |
| 26     | 67,108,864| 57ms/op                |


## Interpretation of Benchmark Results

- **Height vs. Average Proving Time**: 
  - The average proving time per operation increases with higher circuit heights. 
  - There is a **63% increase** in proving time from height 18 to height 26.

- **Regression Speed Analysis**:
  - **Minimum to Maximum Regression**: The benchmarks show a **37% regression** in operations per second at height 26 compared height 18.

  - **Step Regression Speed**: The performance regresses by approximately **10%** when moving from height 18 to height 20, and this trend continues with similar regressions for each subsequent height increment.


## Benchmark Script
```shell
$ yarn bench-merkle-tree
```
## NOTE
If you get this error:
> `AssertionError: Wrong compiler version. Must be at least 2.0.0`

Then always delete the `circom` directory inside `node_modules` when using [circom_tester](https://www.npmjs.com/package/circom_tester) to fix the bug.

```shell
$ rm -rf node_modules/circom
```