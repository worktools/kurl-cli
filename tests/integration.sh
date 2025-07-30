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

# Test 2.1: Simple GET request to baidu.com
echo "Test 2.1: Simple GET to https://www.baidu.com"
$KURL_BIN https://www.baidu.com | grep "百度一下"
echo "OK"
echo

# Test 2.2: HEAD request (-I)
echo "Test 2.2: HEAD request to https://www.baidu.com"
# The body should be empty
if [ -n "$($KURL_BIN -I https://www.baidu.com | grep "百度一下")" ]; then
  echo "FAIL: Body was not empty for -I request"
  exit 1
fi
echo "OK"
echo

# Test 2.3: Save output to file (-o)
echo "Test 2.3: Save output with -o"
$KURL_BIN -o "$TEST_FILE" https://www.baidu.com
if [ ! -f "$TEST_FILE" ]; then
  echo "FAIL: Output file was not created."
  exit 1
fi
if ! grep -q "百度一下" "$TEST_FILE"; then
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
$KURL_BIN -v https://www.baidu.com 2>&1 | grep "Request completed in"
echo "OK"
echo

echo "--- All tests passed! ---"
