# GetCompressedAccountsByOwnerPostRequestParams

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**cursor** | Option<**String**> | A 32-byte hash represented as a base58 string. | [optional]
**data_slice** | Option<[**models::DataSlice**](DataSlice.md)> |  | [optional]
**filters** | Option<[**Vec<models::FilterSelector>**](FilterSelector.md)> |  | [optional]
**limit** | Option<**i32**> |  | [optional]
**owner** | **String** | A Solana public key represented as a base58 string. | [default to 111111131h1vYVSYuKP6AhS86fbRdMw9XHiZAvAaj]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


