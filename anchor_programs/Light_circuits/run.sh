#does a full transformation of a circom generate verifying key
node parse_pvk_to_bytes_254.js;
# obtain arkworks verifying key
cargo run;
# split arkworks verifying key into functions
python3 parse_prepared_verifying_key_to_rust_254.py;
