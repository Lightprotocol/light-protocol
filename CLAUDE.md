# Light Protocol Enhanced Logging

## Transaction Log File

The light-program-test library automatically creates detailed transaction logs in:
```
target/light_program_test.log
```

### Features

- **Always enabled**: Logs are written to file regardless of environment variables
- **Clean format**: Plain text without ANSI color codes for easy reading and processing
- **Session-based**: Each test session starts with a timestamp header, transactions append to the same file
- **Comprehensive details**: Includes transaction signatures, fees, compute usage, instruction hierarchies, Light Protocol instruction parsing, and compressed account information

### Configuration

Enhanced logging is enabled by default. To disable:
```rust
let mut config = ProgramTestConfig::default();
config.enhanced_logging.enabled = false;
```

Console output requires `RUST_BACKTRACE` environment variable and can be controlled separately from file logging.

### Log File Location

The log file is automatically placed in the cargo workspace target directory, making it consistent across different test environments and working directories.