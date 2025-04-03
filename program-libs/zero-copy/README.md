
# Light Zero Copy

### Security Considerations
- do not use on a 32 bit target with length greater than u32
- only length until u64 is supported

### Tests
- `cargo test --features std`
