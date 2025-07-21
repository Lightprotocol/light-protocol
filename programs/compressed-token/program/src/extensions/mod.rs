// pub mod metadata_pointer;
pub mod processor;
pub mod token_metadata;
pub mod token_metadata_ui;

// Import from ctoken-types instead of local modules
use light_ctoken_types::{
    instructions::extensions::ZExtensionInstructionData,
    state::{
        AdditionalMetadataConfig, ExtensionStructConfig, MetadataConfig, TokenMetadata,
        TokenMetadataConfig,
    },
    CTokenError,
};
use light_zero_copy::ZeroCopyNew;

/// Processes extension instruction data and returns the configuration tuple and additional data length
/// Returns: (has_extensions, extension_configs, additional_data_len)
pub fn process_extensions_config(
    extensions: Option<&Vec<ZExtensionInstructionData>>,
) -> Result<(bool, Vec<ExtensionStructConfig>, usize), CTokenError> {
    if let Some(extensions) = extensions {
        let mut additional_mint_data_len = 0;
        let mut config_vec = Vec::new();

        for extension in extensions.iter() {
            match extension {
                /* ZExtensionInstructionData::MetadataPointer(extension) => {
                   let config = MetadataPointerConfig {
                        authority: (extension.authority.is_some(), ()),
                        metadata_address: (extension.metadata_address.is_some(), ()),
                    };
                    let byte_len = MetadataPointer::byte_len(&config);
                    additional_mint_data_len += byte_len;
                    config_vec.push(ExtensionStructConfig::MetadataPointer(config));
                }*/
                ZExtensionInstructionData::TokenMetadata(token_metadata_data) => {
                    process_token_metadata_config(
                        &mut additional_mint_data_len,
                        &mut config_vec,
                        token_metadata_data,
                    )
                }
                _ => return Err(CTokenError::UnsupportedExtension),
            }
        }
        Ok((true, config_vec, additional_mint_data_len))
    } else {
        Ok((false, Vec::new(), 0))
    }
}

fn process_token_metadata_config(
    additional_mint_data_len: &mut usize,
    config_vec: &mut Vec<ExtensionStructConfig>,
    token_metadata_data: &light_ctoken_types::instructions::extensions::ZTokenMetadataInstructionData<'_>,
) {
    let additional_metadata_configs =
        if let Some(ref additional_metadata) = token_metadata_data.additional_metadata {
            additional_metadata
                .iter()
                .map(|item| AdditionalMetadataConfig {
                    key: item.key.len() as u32,
                    value: item.value.len() as u32,
                })
                .collect()
        } else {
            vec![]
        };

    let config = TokenMetadataConfig {
        update_authority: (token_metadata_data.update_authority.is_some(), ()),
        metadata: MetadataConfig {
            name: token_metadata_data.metadata.name.len() as u32,
            symbol: token_metadata_data.metadata.symbol.len() as u32,
            uri: token_metadata_data.metadata.uri.len() as u32,
        },
        additional_metadata: additional_metadata_configs,
    };
    let byte_len = TokenMetadata::byte_len(&config);
    *additional_mint_data_len += byte_len;
    config_vec.push(ExtensionStructConfig::TokenMetadata(config));
}
