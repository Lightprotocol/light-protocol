#!/bin/bash

# Test script for API key authentication on prover server
# This script demonstrates how to test the API key functionality

echo "Testing Prover Server API Key Authentication"
echo "============================================="

# Test variables
SERVER_URL="http://localhost:3001"
API_KEY="test-api-key-12345"

echo ""
echo "1. Testing /health endpoint (should work without API key):"
curl -s -o /dev/null -w "%{http_code}" $SERVER_URL/health
echo ""

echo ""
echo "2. Testing /prove endpoint without API key (should return 401):"
curl -s -o /dev/null -w "%{http_code}" -X POST $SERVER_URL/prove -H "Content-Type: application/json" -d '{"circuit_type": "inclusion"}'
echo ""

echo ""
echo "3. Testing /prove endpoint with X-API-Key header (should work if server has API key):"
curl -s -o /dev/null -w "%{http_code}" -X POST $SERVER_URL/prove -H "Content-Type: application/json" -H "X-API-Key: $API_KEY" -d '{"circuit_type": "inclusion"}'
echo ""

echo ""
echo "4. Testing /prove endpoint with Authorization Bearer header (should work if server has API key):"
curl -s -o /dev/null -w "%{http_code}" -X POST $SERVER_URL/prove -H "Content-Type: application/json" -H "Authorization: Bearer $API_KEY" -d '{"circuit_type": "inclusion"}'
echo ""

echo ""
echo "To run this test:"
echo "1. Set PROVER_API_KEY environment variable: export PROVER_API_KEY=test-api-key-12345"
echo "2. Start the prover server
echo "3. Run this script: bash test_auth.sh"
echo ""
echo "Expected results:"
echo "- /health: 200 (always accessible)"
echo "- /prove without key: 401 (if API key is set)"
echo "- /prove with key: 400 or other (depends on valid circuit data)"
