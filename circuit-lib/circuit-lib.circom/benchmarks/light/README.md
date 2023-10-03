## Build Script
```$ pnpm build-bench-circuits``` to build all artifacts needed for the light circuits benchmarks.

## Benchmark Script
`$ pnpm bench-light-tx` to run circuit benchmarks for **transaction_map2** and **transaction_app4** circuits based on different merkle tree heights.

## TransactionMasp

### Proving Time

| Merkle Tree Height | Avg. Time (ms/op) | Variance | Min. Time (ms) | Max. Time (ms) |
|--------------------|-------------------|----------|----------------|----------------|
| 18                 | 2143              | ± 1.26%  | 1567           | 2621           |
| 20                 | 2175              | ± 1.17%  | 1834           | 2527           |
| 22                 | 2392              | -        | -              | -              |
| 24                 | 2435              | ± 1.01%  | 2109           | 2823           |
| 26                 | 2464              | -        | -              | -              |

### Circuit Statistics

| Merkle Tree Height | # of Wires | # of Constraints | # of Private Inputs | # of Public Inputs | # of Labels | # of Outputs |
|--------------------|------------|------------------|---------------------|--------------------|-------------|--------------|
| 18                 | 15064      | 15032            | 95                  | 9                  | 48416       | 0            |
| 20                 | 16036      | 16000            | 99                  | 9                  | 51520       | 0            |
| 22                 | 17008      | 16968            | 103                 | 9                  | 54624       | 0            |
| 24                 | 17980      | 17936            | 107                 | 9                  | 57728       | 0            |
| 26                 | 18952      | 18904            | 111                 | 9                  | 60832       | 0            |

## TransactionApp

### Proving Time

| Merkle Tree Height | Avg. Time (ms/op) |
|--------------------|-------------------|
| 18                 | 3217              |
| 20                 | 3282              |
| 22                 | 3779              |
| 24                 | 3803              |
| 26                 | 3906              |

### Circuit Statisitics

| Merkle Tree Height | # of Wires | # of Constraints | # of Private Inputs | # of Public Inputs | # of Labels | # of Outputs |
|--------------------|------------|------------------|---------------------|--------------------|-------------|--------------|
| 18                 | 30279      | 30212            | 185                 | 15                 | 99386       | 0            |
| 20                 | 32223      | 32148            | 193                 | 15                 | 105594      | 0            |
| 22                 | 34167      | 34084            | 201                 | 15                 | 111802      | 0            |
| 24                 | 36111      | 36020            | 209                 | 15                 | 118010      | 0            |
| 26                 | 38055      | 37956            | 217                 | 15                 | 124218      | 0            |

