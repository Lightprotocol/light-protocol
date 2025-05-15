# Light Protocol Prover Client

The Light Protocol Prover Client is a Rust library for interacting with the Light Protocol ZKP prover.

## Using an External Prover

By default, tests and applications will spawn a local prover instance. However, for more efficient testing or to use a dedicated prover server, you can use the `PROVER_URL` environment variable to specify an external prover URL.

### Setting the PROVER_URL environment variable

To use an external prover, set the `PROVER_URL` environment variable before running your tests or application:

```bash
# Example: Using a local prover on a different port
export PROVER_URL="http://localhost:3001"

# Example: Using a remote prover server
export PROVER_URL="http://prover.example.com"

# Run tests using the external prover
cargo test
```

When `PROVER_URL` is set:
1. The `spawn_prover` function will not start a local prover
2. All API requests will be directed to the specified URL
3. A health check will be performed to verify the external prover is accessible

### Benefits of using an external prover

- **Faster test execution**: Avoids restarting the prover for each test suite
- **Resource efficiency**: Reduces CPU and memory usage during testing
- **Consistent environment**: Use the same prover instance across different tests or applications
- **Debugging**: Makes it easier to debug prover-related issues

### Local Development vs CI/CD

- For local development: You can start a dedicated prover in one terminal and run tests in another
- For CI/CD workflows: Configure a prover service and set `PROVER_URL` in your pipeline configuration

## API Reference

The client communicates with the prover using HTTP requests. The main endpoints are:

- `GET /health` - Health check endpoint
- `POST /prove` - Submit proof generation requests

## Configuration

The default prover configuration can be customized using the `ProverConfig` struct when spawning a prover:

```rust
let config = ProverConfig {
    run_mode: Some(ProverMode::ForesterTest),
    circuits: vec![ProofType::Inclusion, ProofType::BatchUpdate],
    restart: true,
};
spawn_prover(config).await;
```