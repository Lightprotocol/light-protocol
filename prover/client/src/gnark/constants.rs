pub fn get_server_address() -> String {
    std::env::var("PROVER_URL").unwrap_or_else(|_| "http://localhost:3001".to_string())
}

pub const HEALTH_CHECK: &str = "/health";
pub const PROVE_PATH: &str = "/prove";
