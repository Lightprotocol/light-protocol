<!-- cargo-rdme start -->

# light-bloom-filter

Experimental bloom filter using keccak hashing.

| Type | Description |
|------|-------------|
| [`BloomFilter`] | Probabilistic set with `insert` and `contains` |
| [`BloomFilterError`] | Full or invalid store capacity |
| [`BloomFilter::calculate_bloom_filter_size`] | Optimal bit count for given `n` and `p` |
| [`BloomFilter::calculate_optimal_hash_functions`] | Optimal `k` for given `n` and `m` |
| [`BloomFilter::probe_index_keccak`] | Keccak-based probe index for a value |

<!-- cargo-rdme end -->
