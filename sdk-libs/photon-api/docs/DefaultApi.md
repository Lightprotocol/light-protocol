# \DefaultApi

All URIs are relative to *https://devnet.helius-rpc.com?api-key=<api_key>*

Method | HTTP request | Description
------------- | ------------- | -------------
[**get_batch_address_update_info_post**](DefaultApi.md#get_batch_address_update_info_post) | **POST** /getBatchAddressUpdateInfo | 
[**get_compressed_account_balance_post**](DefaultApi.md#get_compressed_account_balance_post) | **POST** /getCompressedAccountBalance | 
[**get_compressed_account_post**](DefaultApi.md#get_compressed_account_post) | **POST** /getCompressedAccount | 
[**get_compressed_account_proof_post**](DefaultApi.md#get_compressed_account_proof_post) | **POST** /getCompressedAccountProof | 
[**get_compressed_account_proof_v2_post**](DefaultApi.md#get_compressed_account_proof_v2_post) | **POST** /getCompressedAccountProofV2 | 
[**get_compressed_account_v2_post**](DefaultApi.md#get_compressed_account_v2_post) | **POST** /getCompressedAccountV2 | 
[**get_compressed_accounts_by_owner_post**](DefaultApi.md#get_compressed_accounts_by_owner_post) | **POST** /getCompressedAccountsByOwner | 
[**get_compressed_accounts_by_owner_v2_post**](DefaultApi.md#get_compressed_accounts_by_owner_v2_post) | **POST** /getCompressedAccountsByOwnerV2 | 
[**get_compressed_balance_by_owner_post**](DefaultApi.md#get_compressed_balance_by_owner_post) | **POST** /getCompressedBalanceByOwner | 
[**get_compressed_mint_token_holders_post**](DefaultApi.md#get_compressed_mint_token_holders_post) | **POST** /getCompressedMintTokenHolders | 
[**get_compressed_token_account_balance_post**](DefaultApi.md#get_compressed_token_account_balance_post) | **POST** /getCompressedTokenAccountBalance | 
[**get_compressed_token_accounts_by_delegate_post**](DefaultApi.md#get_compressed_token_accounts_by_delegate_post) | **POST** /getCompressedTokenAccountsByDelegate | 
[**get_compressed_token_accounts_by_delegate_v2_post**](DefaultApi.md#get_compressed_token_accounts_by_delegate_v2_post) | **POST** /getCompressedTokenAccountsByDelegateV2 | 
[**get_compressed_token_accounts_by_owner_post**](DefaultApi.md#get_compressed_token_accounts_by_owner_post) | **POST** /getCompressedTokenAccountsByOwner | 
[**get_compressed_token_accounts_by_owner_v2_post**](DefaultApi.md#get_compressed_token_accounts_by_owner_v2_post) | **POST** /getCompressedTokenAccountsByOwnerV2 | 
[**get_compressed_token_balances_by_owner_post**](DefaultApi.md#get_compressed_token_balances_by_owner_post) | **POST** /getCompressedTokenBalancesByOwner | 
[**get_compressed_token_balances_by_owner_v2_post**](DefaultApi.md#get_compressed_token_balances_by_owner_v2_post) | **POST** /getCompressedTokenBalancesByOwnerV2 | 
[**get_compression_signatures_for_account_post**](DefaultApi.md#get_compression_signatures_for_account_post) | **POST** /getCompressionSignaturesForAccount | 
[**get_compression_signatures_for_address_post**](DefaultApi.md#get_compression_signatures_for_address_post) | **POST** /getCompressionSignaturesForAddress | 
[**get_compression_signatures_for_owner_post**](DefaultApi.md#get_compression_signatures_for_owner_post) | **POST** /getCompressionSignaturesForOwner | 
[**get_compression_signatures_for_token_owner_post**](DefaultApi.md#get_compression_signatures_for_token_owner_post) | **POST** /getCompressionSignaturesForTokenOwner | 
[**get_indexer_health_post**](DefaultApi.md#get_indexer_health_post) | **POST** /getIndexerHealth | 
[**get_indexer_slot_post**](DefaultApi.md#get_indexer_slot_post) | **POST** /getIndexerSlot | 
[**get_latest_compression_signatures_post**](DefaultApi.md#get_latest_compression_signatures_post) | **POST** /getLatestCompressionSignatures | 
[**get_latest_non_voting_signatures_post**](DefaultApi.md#get_latest_non_voting_signatures_post) | **POST** /getLatestNonVotingSignatures | 
[**get_multiple_compressed_account_proofs_post**](DefaultApi.md#get_multiple_compressed_account_proofs_post) | **POST** /getMultipleCompressedAccountProofs | 
[**get_multiple_compressed_account_proofs_v2_post**](DefaultApi.md#get_multiple_compressed_account_proofs_v2_post) | **POST** /getMultipleCompressedAccountProofsV2 | 
[**get_multiple_compressed_accounts_post**](DefaultApi.md#get_multiple_compressed_accounts_post) | **POST** /getMultipleCompressedAccounts | 
[**get_multiple_compressed_accounts_v2_post**](DefaultApi.md#get_multiple_compressed_accounts_v2_post) | **POST** /getMultipleCompressedAccountsV2 | 
[**get_multiple_new_address_proofs_post**](DefaultApi.md#get_multiple_new_address_proofs_post) | **POST** /getMultipleNewAddressProofs | 
[**get_multiple_new_address_proofs_v2_post**](DefaultApi.md#get_multiple_new_address_proofs_v2_post) | **POST** /getMultipleNewAddressProofsV2 | 
[**get_queue_elements_post**](DefaultApi.md#get_queue_elements_post) | **POST** /getQueueElements | 
[**get_transaction_with_compression_info_post**](DefaultApi.md#get_transaction_with_compression_info_post) | **POST** /getTransactionWithCompressionInfo | 
[**get_transaction_with_compression_info_v2_post**](DefaultApi.md#get_transaction_with_compression_info_v2_post) | **POST** /getTransactionWithCompressionInfoV2 | 
[**get_validity_proof_post**](DefaultApi.md#get_validity_proof_post) | **POST** /getValidityProof | 
[**get_validity_proof_v2_post**](DefaultApi.md#get_validity_proof_v2_post) | **POST** /getValidityProofV2 | 



## get_batch_address_update_info_post

> models::GetBatchAddressUpdateInfoPost200Response get_batch_address_update_info_post(get_batch_address_update_info_post_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**get_batch_address_update_info_post_request** | [**GetBatchAddressUpdateInfoPostRequest**](GetBatchAddressUpdateInfoPostRequest.md) |  | [required] |

### Return type

[**models::GetBatchAddressUpdateInfoPost200Response**](_getBatchAddressUpdateInfo_post_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_compressed_account_balance_post

> models::GetCompressedAccountBalancePost200Response get_compressed_account_balance_post(get_compressed_account_balance_post_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**get_compressed_account_balance_post_request** | [**GetCompressedAccountBalancePostRequest**](GetCompressedAccountBalancePostRequest.md) |  | [required] |

### Return type

[**models::GetCompressedAccountBalancePost200Response**](_getCompressedAccountBalance_post_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_compressed_account_post

> models::GetCompressedAccountPost200Response get_compressed_account_post(get_compressed_account_post_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**get_compressed_account_post_request** | [**GetCompressedAccountPostRequest**](GetCompressedAccountPostRequest.md) |  | [required] |

### Return type

[**models::GetCompressedAccountPost200Response**](_getCompressedAccount_post_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_compressed_account_proof_post

> models::GetCompressedAccountProofPost200Response get_compressed_account_proof_post(get_compressed_account_proof_post_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**get_compressed_account_proof_post_request** | [**GetCompressedAccountProofPostRequest**](GetCompressedAccountProofPostRequest.md) |  | [required] |

### Return type

[**models::GetCompressedAccountProofPost200Response**](_getCompressedAccountProof_post_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_compressed_account_proof_v2_post

> models::GetCompressedAccountProofV2Post200Response get_compressed_account_proof_v2_post(get_compressed_account_proof_v2_post_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**get_compressed_account_proof_v2_post_request** | [**GetCompressedAccountProofV2PostRequest**](GetCompressedAccountProofV2PostRequest.md) |  | [required] |

### Return type

[**models::GetCompressedAccountProofV2Post200Response**](_getCompressedAccountProofV2_post_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_compressed_account_v2_post

> models::GetCompressedAccountV2Post200Response get_compressed_account_v2_post(get_compressed_account_v2_post_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**get_compressed_account_v2_post_request** | [**GetCompressedAccountV2PostRequest**](GetCompressedAccountV2PostRequest.md) |  | [required] |

### Return type

[**models::GetCompressedAccountV2Post200Response**](_getCompressedAccountV2_post_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_compressed_accounts_by_owner_post

> models::GetCompressedAccountsByOwnerPost200Response get_compressed_accounts_by_owner_post(get_compressed_accounts_by_owner_post_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**get_compressed_accounts_by_owner_post_request** | [**GetCompressedAccountsByOwnerPostRequest**](GetCompressedAccountsByOwnerPostRequest.md) |  | [required] |

### Return type

[**models::GetCompressedAccountsByOwnerPost200Response**](_getCompressedAccountsByOwner_post_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_compressed_accounts_by_owner_v2_post

> models::GetCompressedAccountsByOwnerV2Post200Response get_compressed_accounts_by_owner_v2_post(get_compressed_accounts_by_owner_v2_post_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**get_compressed_accounts_by_owner_v2_post_request** | [**GetCompressedAccountsByOwnerV2PostRequest**](GetCompressedAccountsByOwnerV2PostRequest.md) |  | [required] |

### Return type

[**models::GetCompressedAccountsByOwnerV2Post200Response**](_getCompressedAccountsByOwnerV2_post_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_compressed_balance_by_owner_post

> models::GetCompressedAccountBalancePost200Response get_compressed_balance_by_owner_post(get_compressed_balance_by_owner_post_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**get_compressed_balance_by_owner_post_request** | [**GetCompressedBalanceByOwnerPostRequest**](GetCompressedBalanceByOwnerPostRequest.md) |  | [required] |

### Return type

[**models::GetCompressedAccountBalancePost200Response**](_getCompressedAccountBalance_post_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_compressed_mint_token_holders_post

> models::GetCompressedMintTokenHoldersPost200Response get_compressed_mint_token_holders_post(get_compressed_mint_token_holders_post_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**get_compressed_mint_token_holders_post_request** | [**GetCompressedMintTokenHoldersPostRequest**](GetCompressedMintTokenHoldersPostRequest.md) |  | [required] |

### Return type

[**models::GetCompressedMintTokenHoldersPost200Response**](_getCompressedMintTokenHolders_post_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_compressed_token_account_balance_post

> models::GetCompressedTokenAccountBalancePost200Response get_compressed_token_account_balance_post(get_compressed_token_account_balance_post_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**get_compressed_token_account_balance_post_request** | [**GetCompressedTokenAccountBalancePostRequest**](GetCompressedTokenAccountBalancePostRequest.md) |  | [required] |

### Return type

[**models::GetCompressedTokenAccountBalancePost200Response**](_getCompressedTokenAccountBalance_post_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_compressed_token_accounts_by_delegate_post

> models::GetCompressedTokenAccountsByDelegatePost200Response get_compressed_token_accounts_by_delegate_post(get_compressed_token_accounts_by_delegate_post_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**get_compressed_token_accounts_by_delegate_post_request** | [**GetCompressedTokenAccountsByDelegatePostRequest**](GetCompressedTokenAccountsByDelegatePostRequest.md) |  | [required] |

### Return type

[**models::GetCompressedTokenAccountsByDelegatePost200Response**](_getCompressedTokenAccountsByDelegate_post_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_compressed_token_accounts_by_delegate_v2_post

> models::GetCompressedTokenAccountsByDelegateV2Post200Response get_compressed_token_accounts_by_delegate_v2_post(get_compressed_token_accounts_by_delegate_v2_post_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**get_compressed_token_accounts_by_delegate_v2_post_request** | [**GetCompressedTokenAccountsByDelegateV2PostRequest**](GetCompressedTokenAccountsByDelegateV2PostRequest.md) |  | [required] |

### Return type

[**models::GetCompressedTokenAccountsByDelegateV2Post200Response**](_getCompressedTokenAccountsByDelegateV2_post_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_compressed_token_accounts_by_owner_post

> models::GetCompressedTokenAccountsByDelegatePost200Response get_compressed_token_accounts_by_owner_post(get_compressed_token_accounts_by_owner_post_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**get_compressed_token_accounts_by_owner_post_request** | [**GetCompressedTokenAccountsByOwnerPostRequest**](GetCompressedTokenAccountsByOwnerPostRequest.md) |  | [required] |

### Return type

[**models::GetCompressedTokenAccountsByDelegatePost200Response**](_getCompressedTokenAccountsByDelegate_post_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_compressed_token_accounts_by_owner_v2_post

> models::GetCompressedTokenAccountsByDelegateV2Post200Response get_compressed_token_accounts_by_owner_v2_post(get_compressed_token_accounts_by_owner_v2_post_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**get_compressed_token_accounts_by_owner_v2_post_request** | [**GetCompressedTokenAccountsByOwnerV2PostRequest**](GetCompressedTokenAccountsByOwnerV2PostRequest.md) |  | [required] |

### Return type

[**models::GetCompressedTokenAccountsByDelegateV2Post200Response**](_getCompressedTokenAccountsByDelegateV2_post_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_compressed_token_balances_by_owner_post

> models::GetCompressedTokenBalancesByOwnerPost200Response get_compressed_token_balances_by_owner_post(get_compressed_token_balances_by_owner_post_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**get_compressed_token_balances_by_owner_post_request** | [**GetCompressedTokenBalancesByOwnerPostRequest**](GetCompressedTokenBalancesByOwnerPostRequest.md) |  | [required] |

### Return type

[**models::GetCompressedTokenBalancesByOwnerPost200Response**](_getCompressedTokenBalancesByOwner_post_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_compressed_token_balances_by_owner_v2_post

> models::GetCompressedTokenBalancesByOwnerV2Post200Response get_compressed_token_balances_by_owner_v2_post(get_compressed_token_balances_by_owner_v2_post_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**get_compressed_token_balances_by_owner_v2_post_request** | [**GetCompressedTokenBalancesByOwnerV2PostRequest**](GetCompressedTokenBalancesByOwnerV2PostRequest.md) |  | [required] |

### Return type

[**models::GetCompressedTokenBalancesByOwnerV2Post200Response**](_getCompressedTokenBalancesByOwnerV2_post_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_compression_signatures_for_account_post

> models::GetCompressionSignaturesForAccountPost200Response get_compression_signatures_for_account_post(get_compression_signatures_for_account_post_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**get_compression_signatures_for_account_post_request** | [**GetCompressionSignaturesForAccountPostRequest**](GetCompressionSignaturesForAccountPostRequest.md) |  | [required] |

### Return type

[**models::GetCompressionSignaturesForAccountPost200Response**](_getCompressionSignaturesForAccount_post_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_compression_signatures_for_address_post

> models::GetCompressionSignaturesForAddressPost200Response get_compression_signatures_for_address_post(get_compression_signatures_for_address_post_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**get_compression_signatures_for_address_post_request** | [**GetCompressionSignaturesForAddressPostRequest**](GetCompressionSignaturesForAddressPostRequest.md) |  | [required] |

### Return type

[**models::GetCompressionSignaturesForAddressPost200Response**](_getCompressionSignaturesForAddress_post_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_compression_signatures_for_owner_post

> models::GetCompressionSignaturesForAddressPost200Response get_compression_signatures_for_owner_post(get_compression_signatures_for_owner_post_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**get_compression_signatures_for_owner_post_request** | [**GetCompressionSignaturesForOwnerPostRequest**](GetCompressionSignaturesForOwnerPostRequest.md) |  | [required] |

### Return type

[**models::GetCompressionSignaturesForAddressPost200Response**](_getCompressionSignaturesForAddress_post_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_compression_signatures_for_token_owner_post

> models::GetCompressionSignaturesForAddressPost200Response get_compression_signatures_for_token_owner_post(get_compression_signatures_for_token_owner_post_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**get_compression_signatures_for_token_owner_post_request** | [**GetCompressionSignaturesForTokenOwnerPostRequest**](GetCompressionSignaturesForTokenOwnerPostRequest.md) |  | [required] |

### Return type

[**models::GetCompressionSignaturesForAddressPost200Response**](_getCompressionSignaturesForAddress_post_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_indexer_health_post

> models::GetIndexerHealthPost200Response get_indexer_health_post(get_indexer_health_post_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**get_indexer_health_post_request** | [**GetIndexerHealthPostRequest**](GetIndexerHealthPostRequest.md) |  | [required] |

### Return type

[**models::GetIndexerHealthPost200Response**](_getIndexerHealth_post_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_indexer_slot_post

> models::GetIndexerSlotPost200Response get_indexer_slot_post(get_indexer_slot_post_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**get_indexer_slot_post_request** | [**GetIndexerSlotPostRequest**](GetIndexerSlotPostRequest.md) |  | [required] |

### Return type

[**models::GetIndexerSlotPost200Response**](_getIndexerSlot_post_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_latest_compression_signatures_post

> models::GetCompressionSignaturesForAddressPost200Response get_latest_compression_signatures_post(get_latest_compression_signatures_post_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**get_latest_compression_signatures_post_request** | [**GetLatestCompressionSignaturesPostRequest**](GetLatestCompressionSignaturesPostRequest.md) |  | [required] |

### Return type

[**models::GetCompressionSignaturesForAddressPost200Response**](_getCompressionSignaturesForAddress_post_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_latest_non_voting_signatures_post

> models::GetLatestNonVotingSignaturesPost200Response get_latest_non_voting_signatures_post(get_latest_non_voting_signatures_post_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**get_latest_non_voting_signatures_post_request** | [**GetLatestNonVotingSignaturesPostRequest**](GetLatestNonVotingSignaturesPostRequest.md) |  | [required] |

### Return type

[**models::GetLatestNonVotingSignaturesPost200Response**](_getLatestNonVotingSignatures_post_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_multiple_compressed_account_proofs_post

> models::GetMultipleCompressedAccountProofsPost200Response get_multiple_compressed_account_proofs_post(get_multiple_compressed_account_proofs_post_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**get_multiple_compressed_account_proofs_post_request** | [**GetMultipleCompressedAccountProofsPostRequest**](GetMultipleCompressedAccountProofsPostRequest.md) |  | [required] |

### Return type

[**models::GetMultipleCompressedAccountProofsPost200Response**](_getMultipleCompressedAccountProofs_post_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_multiple_compressed_account_proofs_v2_post

> models::GetMultipleCompressedAccountProofsV2Post200Response get_multiple_compressed_account_proofs_v2_post(get_multiple_compressed_account_proofs_v2_post_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**get_multiple_compressed_account_proofs_v2_post_request** | [**GetMultipleCompressedAccountProofsV2PostRequest**](GetMultipleCompressedAccountProofsV2PostRequest.md) |  | [required] |

### Return type

[**models::GetMultipleCompressedAccountProofsV2Post200Response**](_getMultipleCompressedAccountProofsV2_post_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_multiple_compressed_accounts_post

> models::GetMultipleCompressedAccountsPost200Response get_multiple_compressed_accounts_post(get_multiple_compressed_accounts_post_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**get_multiple_compressed_accounts_post_request** | [**GetMultipleCompressedAccountsPostRequest**](GetMultipleCompressedAccountsPostRequest.md) |  | [required] |

### Return type

[**models::GetMultipleCompressedAccountsPost200Response**](_getMultipleCompressedAccounts_post_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_multiple_compressed_accounts_v2_post

> models::GetMultipleCompressedAccountsV2Post200Response get_multiple_compressed_accounts_v2_post(get_multiple_compressed_accounts_v2_post_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**get_multiple_compressed_accounts_v2_post_request** | [**GetMultipleCompressedAccountsV2PostRequest**](GetMultipleCompressedAccountsV2PostRequest.md) |  | [required] |

### Return type

[**models::GetMultipleCompressedAccountsV2Post200Response**](_getMultipleCompressedAccountsV2_post_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_multiple_new_address_proofs_post

> models::GetMultipleNewAddressProofsPost200Response get_multiple_new_address_proofs_post(get_multiple_new_address_proofs_post_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**get_multiple_new_address_proofs_post_request** | [**GetMultipleNewAddressProofsPostRequest**](GetMultipleNewAddressProofsPostRequest.md) |  | [required] |

### Return type

[**models::GetMultipleNewAddressProofsPost200Response**](_getMultipleNewAddressProofs_post_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_multiple_new_address_proofs_v2_post

> models::GetMultipleNewAddressProofsPost200Response get_multiple_new_address_proofs_v2_post(get_multiple_new_address_proofs_v2_post_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**get_multiple_new_address_proofs_v2_post_request** | [**GetMultipleNewAddressProofsV2PostRequest**](GetMultipleNewAddressProofsV2PostRequest.md) |  | [required] |

### Return type

[**models::GetMultipleNewAddressProofsPost200Response**](_getMultipleNewAddressProofs_post_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_queue_elements_post

> models::GetQueueElementsPost200Response get_queue_elements_post(get_queue_elements_post_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**get_queue_elements_post_request** | [**GetQueueElementsPostRequest**](GetQueueElementsPostRequest.md) |  | [required] |

### Return type

[**models::GetQueueElementsPost200Response**](_getQueueElements_post_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_transaction_with_compression_info_post

> models::GetTransactionWithCompressionInfoPost200Response get_transaction_with_compression_info_post(get_transaction_with_compression_info_post_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**get_transaction_with_compression_info_post_request** | [**GetTransactionWithCompressionInfoPostRequest**](GetTransactionWithCompressionInfoPostRequest.md) |  | [required] |

### Return type

[**models::GetTransactionWithCompressionInfoPost200Response**](_getTransactionWithCompressionInfo_post_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_transaction_with_compression_info_v2_post

> models::GetTransactionWithCompressionInfoV2Post200Response get_transaction_with_compression_info_v2_post(get_transaction_with_compression_info_v2_post_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**get_transaction_with_compression_info_v2_post_request** | [**GetTransactionWithCompressionInfoV2PostRequest**](GetTransactionWithCompressionInfoV2PostRequest.md) |  | [required] |

### Return type

[**models::GetTransactionWithCompressionInfoV2Post200Response**](_getTransactionWithCompressionInfoV2_post_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_validity_proof_post

> models::GetValidityProofPost200Response get_validity_proof_post(get_validity_proof_post_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**get_validity_proof_post_request** | [**GetValidityProofPostRequest**](GetValidityProofPostRequest.md) |  | [required] |

### Return type

[**models::GetValidityProofPost200Response**](_getValidityProof_post_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_validity_proof_v2_post

> models::GetValidityProofV2Post200Response get_validity_proof_v2_post(get_validity_proof_v2_post_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**get_validity_proof_v2_post_request** | [**GetValidityProofV2PostRequest**](GetValidityProofV2PostRequest.md) |  | [required] |

### Return type

[**models::GetValidityProofV2Post200Response**](_getValidityProofV2_post_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

