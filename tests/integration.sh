#!/bin/bash

# integration.sh
#
# A simple integration test script for kurl.
# It builds the project and runs a series of tests against live servers.

set -e

KURL_BIN=./target/debug/kurl
TEST_FILE="test_output.html"

# Cleanup function to remove test files
cleanup() {
  rm -f "$TEST_FILE"
}

# Trap cleanup function to be called on script exit
trap cleanup EXIT

# 1. Build the project
echo "--- Building kurl ---"
cargo build
echo "Build complete."
echo

# 2. Run tests
echo "--- Running Tests ---"

# Test 2.1: Simple GET request to httpbin.org/html
echo "Test 2.1: Simple GET to https://httpbin.org/html"
$KURL_BIN https://httpbin.org/html | grep "Herman Melville"
echo "OK"
echo

# Test 2.2: HEAD request (-I)
echo "Test 2.2: HEAD request to https://httpbin.org/html"
# The body should be empty
if [ -n "$($KURL_BIN -I https://httpbin.org/html | grep "Herman Melville")" ]; then
  echo "FAIL: Body was not empty for -I request"
  exit 1
fi
echo "OK"
echo

# Test 2.3: Save output to file (-o)
echo "Test 2.3: Save output with -o"
$KURL_BIN -o "$TEST_FILE" https://httpbin.org/html
if [ ! -f "$TEST_FILE" ]; then
  echo "FAIL: Output file was not created."
  exit 1
fi
if ! grep -q "Herman Melville" "$TEST_FILE"; then
  echo "FAIL: Output file does not contain expected content."
  exit 1
fi
echo "OK"
echo

# Test 2.4: Follow redirect (-L)
echo "Test 2.4: Follow redirect with -L"
# http://www.sina.com.cn redirects to https://www.sina.com.cn
$KURL_BIN -L http://www.sina.com.cn | grep -i "sina"
echo "OK"
echo

# Test 2.5: Insecure connection (-k)
# This test still uses an external site and may fail due to network issues.
echo "Test 2.5: Allow insecure connection with -k"
$KURL_BIN -k https://self-signed.badssl.com/ | grep "self-signed"
echo "OK"
echo

# Test 2.6: Verbose output with timing (-v)
echo "Test 2.6: Verbose output with timing"
$KURL_BIN -v https://httpbin.org/html 2>&1 | grep "Request completed in"
echo "OK"
echo

# Test 2.7: Send cookie with -b
echo "Test 2.7: Send cookie with -b"
$KURL_BIN -b "mycookie=myvalue" https://httpbin.org/cookies | grep '"mycookie": "myvalue"'
echo "OK"
echo

# Test 2.8: Send raw JSON with --data-raw
echo "Test 2.8: Send raw JSON with --data-raw"
$KURL_BIN -X POST --data-raw '{"key":"value"}' -H "Content-Type: application/json" https://httpbin.org/post | grep -q '"json": {
    "key": "value"
  }'
echo "OK"
echo

# Test 2.9: Auto-detect POST with -d
echo "Test 2.9: Auto-detect POST with -d"
$KURL_BIN -d "key=value" https://httpbin.org/post | grep -q '"form": {
    "key": "value"
  }'
echo "OK"
echo


echo "--- All tests passed! ---"
