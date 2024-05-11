use crate::constants::INDEXER_SERVER_ADDRESS;
use photon_api::apis::configuration::Configuration;
use photon_api::apis::default_api::{
    get_compressed_accounts_by_owner_post, GetCompressedAccountProofPostError,
    GetCompressedAccountsByOwnerPostError, GetMultipleCompressedAccountProofsPostError,
};
use photon_api::apis::Error;
use photon_api::models::GetCompressedAccountsByOwnerPost200Response;
use photon_api::models::{
    GetCompressedAccountsByOwnerPostRequest, GetMultipleCompressedAccountProofsPost200Response,
};
use solana_sdk::bs58;

pub fn get_configuration() -> Configuration {
    let mut configuration = Configuration::new();
    configuration.base_path = INDEXER_SERVER_ADDRESS.to_string();
    configuration
}

pub async fn get_compressed_accounts_by_owner(
    owner_pubkey: &str,
) -> Result<GetCompressedAccountsByOwnerPost200Response, Error<GetCompressedAccountsByOwnerPostError>>
{
    let configuration = get_configuration();
    let mut request: GetCompressedAccountsByOwnerPostRequest =
        GetCompressedAccountsByOwnerPostRequest::default();
    request.params.owner = owner_pubkey.to_string();
    get_compressed_accounts_by_owner_post(&configuration, request).await
}

pub async fn get_compressed_account_proof(
    compressed_account: &str,
) -> Result<(Vec<[u8; 32]>, u64, i64), Error<GetCompressedAccountProofPostError>> {
    let mut retries = 20;
    loop {
        let configuration = get_configuration();
        let mut request = photon_api::models::GetCompressedAccountProofPostRequest::default();
        request.params.hash = compressed_account.to_string();
        match photon_api::apis::default_api::get_compressed_account_proof_post(&configuration, request)
            .await {
            Ok(result) => {
                let result = result.result.unwrap();
                let mut proof_result_value = result.value.proof.clone();
                proof_result_value.truncate(proof_result_value.len() - 1); // Remove root
                proof_result_value.truncate(proof_result_value.len() - 10); // Remove canopy
                let proof: Vec<[u8; 32]> = proof_result_value.iter().map(|x| decode_hash(x)).collect();
                let seq = result.value.root_seq;
                return Ok((proof, result.value.leaf_index as u64, seq));
            }
            Err(e) => {
                retries -= 1;
                tokio::time::sleep(tokio::time::Duration::from_secs(20 - retries)).await;
                if retries == 0 {
                    return Err(e);
                }
            }
        }
    }
}

pub async fn get_multiple_compressed_account_proofs(
    hashes: Vec<String>,
) -> Result<
    GetMultipleCompressedAccountProofsPost200Response,
    Error<GetMultipleCompressedAccountProofsPostError>,
> {
    let configuration = get_configuration();
    let request = photon_api::models::GetMultipleCompressedAccountProofsPostRequest {
        params: hashes,
        ..Default::default()
    };
    photon_api::apis::default_api::get_multiple_compressed_account_proofs_post(
        &configuration,
        request,
    )
    .await
}

pub fn decode_hash(account: &str) -> [u8; 32] {
    let bytes = bs58::decode(account).into_vec().unwrap();
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    arr
}
