#!/bin/bash

# Trusted Party 1
openssl genpkey -algorithm ed25519 -outform PEM -out keys/trusted-party-1-ed25519priv.pem
openssl pkey -in keys/trusted-party-1-ed25519priv.pem -pubout -outform DER -out keys/trusted-party-1-ed25519pub.der
tail -c 32 keys/trusted-party-1-ed25519pub.der > keys/trusted-party-1-ed25519pub.raw
base64 keys/trusted-party-1-ed25519pub.raw > keys/trusted-party-1-ed25519pub-contract.txt

# Trusted Party 2
openssl genpkey -algorithm ed25519 -outform PEM -out keys/trusted-party-2-ed25519priv.pem
openssl pkey -in keys/trusted-party-2-ed25519priv.pem -pubout -outform DER -out keys/trusted-party-2-ed25519pub.der
tail -c 32 keys/trusted-party-2-ed25519pub.der > keys/trusted-party-2-ed25519pub.raw
base64 keys/trusted-party-2-ed25519pub.raw > keys/trusted-party-2-ed25519pub-contract.txt