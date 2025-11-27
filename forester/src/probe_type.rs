#[cfg(test)]
mod tests {
    use light_batched_merkle_tree::address_merkle_tree::AddressMerkleTreeAccount;

    #[test]
    fn test_import() {
        let _ = AddressMerkleTreeAccount::default();
    }
}
