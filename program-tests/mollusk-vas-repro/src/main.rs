use {
    base64::{engine::general_purpose::STANDARD, Engine as _},
    mollusk_svm::{program::ProgramCache, Mollusk},
    mollusk_svm_result::types::TransactionResult,
    serde_json::Value,
    solana_account::Account,
    solana_instruction::{AccountMeta, Instruction},
    solana_pubkey::Pubkey,
    std::{
        env, fs,
        path::{Path, PathBuf},
        str::FromStr,
    },
};

const PROGRAM: &str = "compr6CUsB5m2jS4Y3831ztGSTnDpnKJTKS95d64XVq";
const AUTHORITY: &str = "HwXnGK3tPkkVY6P439H2p68AxpeuWXd5PcrAxFpbmfbA";
const REGISTERED_PROGRAM: &str = "35hkDgaAKwMCaxRz2ocSZ6NaUrtKkyNqU6c4RV3tYJRh";
const STATE_TREE: &str = "smt8TYxNy8SuhAdKJ8CeLtDkr2w6dgDmdz5ruiDw9Y9";

const INNER_IXS: [&str; 3] = [
    "2ctAyTe3sKSZHzm2twB7y1gJZjd6oxGarm5jf3uWTC7kbkCD14PVYSredMidiWKb2v69uAJwj76Jrj9WGT1H4aBMhta1RCGaDAhd5cpEkXM8DkpqNF84XKoyP4fB3zHEXeqw6BiDjCr4a85XyByX4poUJBQUd4atCRq3sXm4KtxcrSNCvjXaR5WXhfvJDgdm5cg2wYfSHuHHJgXGFFPs8fuC6Ld6zSyNqvdtb3YRh1kQw4p7ajcLyajsmhe1jfJ2UBW4dAgE7AyQXjA3hhz6UsXM91gMqjHmQ8CbkFqUWEchCGJqR7GFEJ2RtZgSRg7wtEB69L9nxhWZDEyscbikpEnWLP8DKn5PbbSJbhMo9WDh61aF4dHCotdBeVnhPUktzEqAkT8xdUkUj8HATW4WD6iPyCWDjijchzf1Je7Euh5cwcTLqHjoWp8dJ6rzyu",
    "2ctAyTe3sKSZHzm2twB7y1gJZjd6oxGarm5jf3uWTC7kbkCD14PVYSredMidiWKb2v69uAJwbhTx7UoUdSCjbdr52UVTL2gnp9EUKbWMSdYDStQ9gh2GWRtAHoU474cHcVx3y7bA4kWk2DymwFecEoCUxC49oFcS9yqTskpGBxeVfKA6MCCFTRzaEXmj7dfAEKUGJERT9RHDeH1qJk1U2kd4gbc7nZy4i2WjqjDsAkhLwqehxJdZdpHG5fJ4XRfewXMCYoT8q3MwqX4aW25RHbnin3vm8oUgf1kEFApyjvtv5ya4Xco4B1xDjCW8Fb7Qdeq5zefS84dFNJ4mK37MiGN2zTbU7ACPFrSBNFyZAYLxnjsXP2YWKAnk4zevV1tobaxhXsKy45gd52CituXJS9nj8uq1BsYzbcL1Fwhzqh9fCVCgQ8Xgerih7JYAaT",
    "2ctAyTe3sKSZHzm2twB7y1gJZjd6oxGarm5jf3uWTC7kbkCD14PVYSredMidiWKb2v69uAJws9jgoewPcbrYMrBtvTsXKVhNPiEwuLK9YwxFnZ7yJL6Tg7cKeHqZirMLAFYvS5pfhCBwBZsU6Gu5TvVUCETjXAbtpKjhAeiekLXPnL5XmP3hn23C5cezTxcGTH46fNYxzAR1p4op15ba5TXmPntBHdwhLKzp8pPhT8RHXksN2tmssN9usfjy8tX2KGGdqNaPrmdMn7cKG2aNSqF9rnjmMAXHA8gkFhaYTRzfVeFQjb7JChDc2v68zHpMnH6dtFtManhRwQxeLtSejwMZLzSB3f7z5LDCUdYjjejdaAuxc7jsXJrjeBphbaUwiRr9uYsG5tdQmzsP9TCGGmrF4etj13Ybwmnfj8qQ5hasMmqHiZYYbTFw7JaagF",
];

#[derive(Debug)]
struct Config {
    elf_path: PathBuf,
    registered_program_account_path: PathBuf,
    state_tree_account_path: PathBuf,
    output_dir: PathBuf,
}

impl Config {
    fn from_env() -> Self {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let fixtures_dir = manifest_dir.join("fixtures");

        Self {
            elf_path: env_path_or(
                "MOLLUSK_VAS_ELF",
                fixtures_dir.join("account-compression-mainnet.so"),
            ),
            registered_program_account_path: env_path_or(
                "MOLLUSK_VAS_REGISTERED_PROGRAM_ACCOUNT",
                fixtures_dir.join(format!("{REGISTERED_PROGRAM}.bin")),
            ),
            state_tree_account_path: env_path_or(
                "MOLLUSK_VAS_STATE_TREE_ACCOUNT",
                fixtures_dir.join(format!("{STATE_TREE}.bin")),
            ),
            output_dir: env_path_or("MOLLUSK_VAS_OUTPUT_DIR", manifest_dir.join("output")),
        }
    }
}

fn env_path_or(name: &str, default: PathBuf) -> PathBuf {
    env::var_os(name).map(PathBuf::from).unwrap_or(default)
}

fn load_solana_cli_account(path: &Path) -> Account {
    let json: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let account = &json["account"];
    let data = account["data"][0].as_str().unwrap();
    let encoding = account["data"][1].as_str().unwrap();
    assert_eq!(encoding, "base64");

    Account {
        lamports: account["lamports"].as_u64().unwrap(),
        data: STANDARD.decode(data).unwrap(),
        owner: Pubkey::from_str(account["owner"].as_str().unwrap()).unwrap(),
        executable: account["executable"].as_bool().unwrap(),
        rent_epoch: account["rentEpoch"].as_u64().unwrap(),
    }
}

fn run(config: &Config, feature_enabled: bool) -> TransactionResult {
    let program_id = Pubkey::from_str(PROGRAM).unwrap();
    let authority = Pubkey::from_str(AUTHORITY).unwrap();
    let registered_program = Pubkey::from_str(REGISTERED_PROGRAM).unwrap();
    let state_tree = Pubkey::from_str(STATE_TREE).unwrap();

    let mut mollusk = Mollusk::default();
    mollusk.feature_set.virtual_address_space_adjustments = feature_enabled;
    mollusk.program_cache = ProgramCache::new(&mollusk.feature_set, &mollusk.compute_budget, false);

    let elf = fs::read(&config.elf_path).unwrap();
    mollusk.add_program_with_loader_and_elf(
        &program_id,
        &solana_sdk_ids::bpf_loader_upgradeable::ID,
        &elf,
    );
    mollusk.warp_to_slot(431_327_345);

    let instructions: Vec<Instruction> = INNER_IXS
        .iter()
        .map(|data| Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new_readonly(authority, true),
                AccountMeta::new_readonly(registered_program, false),
                AccountMeta::new(state_tree, false),
            ],
            data: bs58::decode(data).into_vec().unwrap(),
        })
        .collect();

    let accounts = vec![
        (
            authority,
            Account {
                lamports: 1,
                owner: solana_sdk_ids::system_program::ID,
                ..Account::default()
            },
        ),
        (
            registered_program,
            load_solana_cli_account(&config.registered_program_account_path),
        ),
        (
            state_tree,
            load_solana_cli_account(&config.state_tree_account_path),
        ),
    ];

    mollusk.process_transaction_instructions(&instructions, &accounts)
}

fn main() {
    let config = Config::from_env();
    println!("config: {config:#?}");

    let disabled = run(&config, false);
    let enabled = run(&config, true);
    let state_tree = Pubkey::from_str(STATE_TREE).unwrap();
    let before = load_solana_cli_account(&config.state_tree_account_path).data;
    let disabled_data = &disabled.get_account(&state_tree).unwrap().data;
    let enabled_data = &enabled.get_account(&state_tree).unwrap().data;

    fs::create_dir_all(&config.output_dir).unwrap();
    fs::write(
        config.output_dir.join("disabled-state-tree.raw"),
        disabled_data,
    )
    .unwrap();
    fs::write(
        config.output_dir.join("enabled-state-tree.raw"),
        enabled_data,
    )
    .unwrap();

    println!(
        "disabled result: {:?}, CU: {}",
        disabled.program_result, disabled.compute_units_consumed
    );
    println!(
        "enabled result:  {:?}, CU: {}",
        enabled.program_result, enabled.compute_units_consumed
    );
    println!(
        "disabled changed bytes: {}",
        before
            .iter()
            .zip(disabled_data)
            .filter(|(before, after)| before != after)
            .count()
    );
    println!(
        "enabled changed bytes:  {}",
        before
            .iter()
            .zip(enabled_data)
            .filter(|(before, after)| before != after)
            .count()
    );
    println!("outputs equal: {}", disabled_data == enabled_data);

    let diffs: Vec<_> = disabled_data
        .iter()
        .zip(enabled_data)
        .enumerate()
        .filter_map(|(i, (disabled_byte, enabled_byte))| {
            (disabled_byte != enabled_byte).then_some((i, *disabled_byte, *enabled_byte))
        })
        .take(32)
        .collect();
    println!("first output diffs (offset, disabled, enabled): {diffs:?}");

    let diff_ranges = diff_ranges(disabled_data, enabled_data);
    println!(
        "disabled/enabled differing bytes: {}",
        disabled_data
            .iter()
            .zip(enabled_data)
            .filter(|(disabled_byte, enabled_byte)| disabled_byte != enabled_byte)
            .count()
    );
    println!("disabled/enabled diff ranges: {}", diff_ranges.len());
    for (start, end) in diff_ranges {
        println!(
            "range {start}..{end} len={} disabled={} enabled={}",
            end - start,
            to_hex(&disabled_data[start..end]),
            to_hex(&enabled_data[start..end]),
        );
    }
}

fn diff_ranges(left: &[u8], right: &[u8]) -> Vec<(usize, usize)> {
    let mut diff_ranges = Vec::new();
    let mut start = None;

    for (i, (left_byte, right_byte)) in left.iter().zip(right).enumerate() {
        if left_byte != right_byte {
            if start.is_none() {
                start = Some(i);
            }
        } else if let Some(range_start) = start.take() {
            diff_ranges.push((range_start, i));
        }
    }

    if let Some(range_start) = start {
        diff_ranges.push((range_start, left.len().min(right.len())));
    }

    diff_ranges
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
