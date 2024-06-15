# \DefaultApi

All URIs are relative to *http://127.0.0.1*

Method | HTTP request | Description
------------- | ------------- | -------------
[**get_compressed_account_post**](DefaultApi.md#get_compressed_account_post) | **POST** /getCompressedAccount | 
[**get_compressed_account_proof_post**](DefaultApi.md#get_compressed_account_proof_post) | **POST** /getCompressedAccountProof | 
[**get_compressed_accounts_by_owner_post**](DefaultApi.md#get_compressed_accounts_by_owner_post) | **POST** /getCompressedAccountsByOwner | 
[**get_compressed_balance_by_owner_post**](DefaultApi.md#get_compressed_balance_by_owner_post) | **POST** /getCompressedBalanceByOwner | 
[**get_compressed_balance_post**](DefaultApi.md#get_compressed_balance_post) | **POST** /getCompressedBalance | 
[**get_compressed_token_account_balance_post**](DefaultApi.md#get_compressed_token_account_balance_post) | **POST** /getCompressedTokenAccountBalance | 
[**get_compressed_token_accounts_by_delegate_post**](DefaultApi.md#get_compressed_token_accounts_by_delegate_post) | **POST** /getCompressedTokenAccountsByDelegate | 
[**get_compressed_token_accounts_by_owner_post**](DefaultApi.md#get_compressed_token_accounts_by_owner_post) | **POST** /getCompressedTokenAccountsByOwner | 
[**get_compressed_token_balances_by_owner_post**](DefaultApi.md#get_compressed_token_balances_by_owner_post) | **POST** /getCompressedTokenBalancesByOwner | 
[**get_compression_signatures_for_account_post**](DefaultApi.md#get_compression_signatures_for_account_post) | **POST** /getCompressionSignaturesForAccount | 
[**get_compression_signatures_for_address_post**](DefaultApi.md#get_compression_signatures_for_address_post) | **POST** /getCompressionSignaturesForAddress | 
[**get_compression_signatures_for_owner_post**](DefaultApi.md#get_compression_signatures_for_owner_post) | **POST** /getCompressionSignaturesForOwner | 
[**get_compression_signatures_for_token_owner_post**](DefaultApi.md#get_compression_signatures_for_token_owner_post) | **POST** /getCompressionSignaturesForTokenOwner | 
[**get_indexer_health_post**](DefaultApi.md#get_indexer_health_post) | **POST** /getIndexerHealth | 
[**get_indexer_slot_post**](DefaultApi.md#get_indexer_slot_post) | **POST** /getIndexerSlot | 
[**get_latest_compression_signatures_post**](DefaultApi.md#get_latest_compression_signatures_post) | **POST** /getLatestCompressionSignatures | 
[**get_latest_non_voting_signatures_post**](DefaultApi.md#get_latest_non_voting_signatures_post) | **POST** /getLatestNonVotingSignatures | 
[**get_multiple_compressed_account_proofs_post**](DefaultApi.md#get_multiple_compressed_account_proofs_post) | **POST** /getMultipleCompressedAccountProofs | 
[**get_multiple_compressed_accounts_post**](DefaultApi.md#get_multiple_compressed_accounts_post) | **POST** /getMultipleCompressedAccounts | 
[**get_multiple_new_address_proofs_post**](DefaultApi.md#get_multiple_new_address_proofs_post) | **POST** /getMultipleNewAddressProofs | 
[**get_transaction_with_compression_info_post**](DefaultApi.md#get_transaction_with_compression_info_post) | **POST** /getTransactionWithCompressionInfo | 
[**get_validity_proof_post**](DefaultApi.md#get_validity_proof_post) | **POST** /getValidityProof | 



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


## get_compressed_balance_by_owner_post

> models::GetCompressedBalancePost200Response get_compressed_balance_by_owner_post(get_compressed_balance_by_owner_post_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**get_compressed_balance_by_owner_post_request** | [**GetCompressedBalanceByOwnerPostRequest**](GetCompressedBalanceByOwnerPostRequest.md) |  | [required] |

### Return type

[**models::GetCompressedBalancePost200Response**](_getCompressedBalance_post_200_response.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_compressed_balance_post

> models::GetCompressedBalancePost200Response get_compressed_balance_post(get_compressed_balance_post_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**get_compressed_balance_post_request** | [**GetCompressedBalancePostRequest**](GetCompressedBalancePostRequest.md) |  | [required] |

### Return type

[**models::GetCompressedBalancePost200Response**](_getCompressedBalance_post_200_response.md)

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

> models::GetCompressionSignaturesForAccountPost200Response get_latest_non_voting_signatures_post(get_latest_non_voting_signatures_post_request)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**get_latest_non_voting_signatures_post_request** | [**GetLatestNonVotingSignaturesPostRequest**](GetLatestNonVotingSignaturesPostRequest.md) |  | [required] |

### Return type

[**models::GetCompressionSignaturesForAccountPost200Response**](_getCompressionSignaturesForAccount_post_200_response.md)

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

