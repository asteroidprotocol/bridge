#!/bin/bash

# Trusted Party 1
openssl genpkey -algorithm ed25519 -outform PEM -out keys/trusted-party-1-ed25519priv.pem

openssl pkey -in keys/trusted-party-1-ed25519priv.pem -outform DER -out keys/trusted-party-1-ed25519priv.der
openssl base64 -in keys/trusted-party-1-ed25519priv.der -out keys/trusted-party-1-ed25519priv-base64.txt

openssl pkey -in keys/trusted-party-1-ed25519priv.pem -pubout -outform DER -out keys/trusted-party-1-ed25519pub.der
openssl base64 -in keys/trusted-party-1-ed25519pub.der -out keys/trusted-party-1-ed25519pub-base64.txt

tail -c 32 keys/trusted-party-1-ed25519pub.der > keys/trusted-party-1-ed25519pub.raw
base64 keys/trusted-party-1-ed25519pub.raw > keys/trusted-party-1-ed25519pub-contract.txt

# Trusted Party 2
openssl genpkey -algorithm ed25519 -outform PEM -out keys/trusted-party-2-ed25519priv.pem

openssl pkey -in keys/trusted-party-2-ed25519priv.pem -outform DER -out keys/trusted-party-2-ed25519priv.der
openssl base64 -in keys/trusted-party-2-ed25519priv.der -out keys/trusted-party-2-ed25519priv-base64.txt

openssl pkey -in keys/trusted-party-2-ed25519priv.pem -pubout -outform DER -out keys/trusted-party-2-ed25519pub.der
openssl base64 -in keys/trusted-party-2-ed25519pub.der -out keys/trusted-party-2-ed25519pub-base64.txt

tail -c 32 keys/trusted-party-2-ed25519pub.der > keys/trusted-party-2-ed25519pub.raw
base64 keys/trusted-party-2-ed25519pub.raw > keys/trusted-party-2-ed25519pub-contract.txt