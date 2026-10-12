#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
project_dir="$(cd "${script_dir}/.." && pwd)"
cert_dir="${CERT_DIR:-${project_dir}/certs}"
work_dir="$(mktemp -d)"

cleanup() {
  rm -rf "${work_dir}"
}
trap cleanup EXIT

umask 077
mkdir -p "${cert_dir}"

openssl genpkey \
  -algorithm RSA \
  -pkeyopt rsa_keygen_bits:3072 \
  -out "${work_dir}/ca-key.pem"

openssl req \
  -x509 \
  -new \
  -sha256 \
  -days 3650 \
  -key "${work_dir}/ca-key.pem" \
  -subj "/CN=StockGuard Development CA" \
  -out "${work_dir}/ca.pem"

openssl genpkey \
  -algorithm RSA \
  -pkeyopt rsa_keygen_bits:2048 \
  -out "${work_dir}/inventory-key.pem"

openssl req \
  -new \
  -sha256 \
  -key "${work_dir}/inventory-key.pem" \
  -subj "/CN=inventory.local" \
  -out "${work_dir}/inventory.csr"

printf '%s\n' \
  'basicConstraints=critical,CA:FALSE' \
  'keyUsage=critical,digitalSignature,keyEncipherment' \
  'extendedKeyUsage=serverAuth' \
  'subjectAltName=DNS:inventory.local' \
  > "${work_dir}/inventory.ext"

openssl x509 \
  -req \
  -sha256 \
  -days 825 \
  -in "${work_dir}/inventory.csr" \
  -CA "${work_dir}/ca.pem" \
  -CAkey "${work_dir}/ca-key.pem" \
  -CAcreateserial \
  -extfile "${work_dir}/inventory.ext" \
  -out "${work_dir}/inventory.pem"

openssl genpkey \
  -algorithm RSA \
  -pkeyopt rsa_keygen_bits:2048 \
  -out "${work_dir}/checkout-key.pem"

openssl req \
  -new \
  -sha256 \
  -key "${work_dir}/checkout-key.pem" \
  -subj "/CN=checkout.local" \
  -out "${work_dir}/checkout.csr"

printf '%s\n' \
  'basicConstraints=critical,CA:FALSE' \
  'keyUsage=critical,digitalSignature,keyEncipherment' \
  'extendedKeyUsage=clientAuth' \
  > "${work_dir}/checkout.ext"

openssl x509 \
  -req \
  -sha256 \
  -days 825 \
  -in "${work_dir}/checkout.csr" \
  -CA "${work_dir}/ca.pem" \
  -CAkey "${work_dir}/ca-key.pem" \
  -CAserial "${work_dir}/ca.srl" \
  -extfile "${work_dir}/checkout.ext" \
  -out "${work_dir}/checkout.pem"

for public_file in ca.pem inventory.pem checkout.pem; do
  cp "${work_dir}/${public_file}" "${cert_dir}/${public_file}"
  chmod 644 "${cert_dir}/${public_file}"
done

for private_file in ca-key.pem inventory-key.pem checkout-key.pem; do
  cp "${work_dir}/${private_file}" "${cert_dir}/${private_file}"
  chmod 600 "${cert_dir}/${private_file}"
done

openssl verify \
  -CAfile "${cert_dir}/ca.pem" \
  "${cert_dir}/inventory.pem" \
  "${cert_dir}/checkout.pem"

openssl x509 \
  -in "${cert_dir}/inventory.pem" \
  -noout \
  -subject \
  -issuer \
  -dates \
  -ext subjectAltName \
  -ext extendedKeyUsage

openssl x509 \
  -in "${cert_dir}/checkout.pem" \
  -noout \
  -subject \
  -issuer \
  -dates \
  -ext extendedKeyUsage

echo "Development certificates generated in ${cert_dir}"
