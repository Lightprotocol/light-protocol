use zero_copy_macro::ZeroCopyAccount;

#[derive(Debug, PartialEq, Clone, ZeroCopyAccount)]
struct TestAccount {
    id: u32,
    balance: u64,
    maybe_flag: Option<u16>,
    maybe_address: Option<[u8; 32]>,
}
