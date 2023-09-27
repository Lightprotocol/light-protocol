# Build

`cargo xtask write-lookup-table --size 16` to create a lookup table of desired size.

**Note**: A lookup table of size 16 is essential for tests as well as benchmarks.
## Benchmarks

| Lookup Table 16  | Threads | Min         | Mean        | Max         |
|------------------|---------|-------------|-------------|-------------|
|FILE SIZE: 2,3 M  | 1       | 380.99 ms   | 388.02 ms   | 395.64 ms   |
|                  | 2       | 244.07 ms   | 255.75 ms   | 268.74 ms   |
|                  | 4       | 161.66 ms   | 173.41 ms   | 186.44 ms   |
|                  | 8       | 170.37 ms   | 181.19 ms   | 192.77 ms   |
|                  | 16      | 132.64 ms   | 141.53 ms   | 151.41 ms   |




| Lookup Table 17  | Threads | Min         | Mean        | Max         |
|------------------|---------|-------------|-------------|-------------|
|FILE SIZE: 4,6 M  | 1       | 181.39 ms   | 184.52 ms   | 187.85 ms   |
|                  | 2       | 107.03 ms   | 109.92 ms   | 112.99 ms   |
|                  | 4       | 73.460 ms   | 76.732 ms   | 80.188 ms   |
|                  | 8       | 79.889 ms   | 82.946 ms   | 86.142 ms   |
|                  | 16      | 56.610 ms   | 58.392 ms   | 60.240 ms   |


| Lookup Table 18   | Threads | Min        | Mean       | Max        |
|------------------ |---------|------------|------------|------------|
|FILE SIZE: 9,1 M   | 1       | 85.268 ms  | 85.784 ms  | 86.416 ms  |
|                   | 2       | 57.342 ms  | 58.207 ms  | 59.123 ms  |
|                   | 4       | 36.816 ms  | 37.701 ms  | 38.727 ms  |
|                   | 8       | 42.512 ms  | 43.145 ms  | 43.792 ms  |
|                   | 16      | 30.491 ms  | 30.920 ms  | 31.387 ms  |


| Lookup Table 19  | Threads | Min        | Mean       | Max        |
|------------------|---------|------------|------------|------------|
|FILE SIZE: 19 M   | 1       | 50.137 ms  | 50.659 ms  | 51.383 ms  |
|                  | 2       | 30.368 ms  | 31.287 ms  | 32.339 ms  |
|                  | 4       | 21.378 ms  | 22.800 ms  | 24.430 ms  |
|                  | 8       | 25.709 ms  | 26.913 ms  | 28.214 ms  |
|                  | 16      | 17.627 ms  | 18.742 ms  | 19.925 ms  |







