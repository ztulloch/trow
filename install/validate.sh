#!/bin/bash
set -eo pipefail
unset CDPATH
IFS=$'\n\t'

echo "Setting up trow as a validating webhook"
echo "WARNING: This will limit what images can run in your cluster"
echo

cabundle=$(kubectl get configmap trow-cert -n kube-public -o jsonpath='{.data.cert}' | base64 | tr -d '\n')
#Really not happy about use of sed here
sed "s/{{cabundle}}/${cabundle}/" validate-tmpl.yaml > validate.yaml
kubectl apply -f validate.yaml
