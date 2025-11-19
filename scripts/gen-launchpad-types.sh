#!/bin/bash -e

existing_json="./target/near/aurora_launchpad_contract/aurora_launchpad_contract_abi.json"
type_output="./res/schema.ts"

# shellcheck disable=SC2016
jq_filter='
# Extract JSON Schema
.body.root_schema

# Get rid of unnecessary "type"
| del(.type)

# Set reasonable title
| .title = "Aurora Launchpad Contract ABI"

# Problem: WebAuthn definition includes "properties" and "onOf" together.
#          Tools cannot resolve such ambiguity. So we separate them into
#          two different definitions.
| .definitions.MultiPayload.oneOf |= (
      map(
        if ((.properties.standard.enum // []) | contains(["webauthn"])) then
          . as $webauthn |
          $webauthn.anyOf | map(
            {
              type: "object",
              description: .description,
              required: ($webauthn.required + .required),
              properties: ($webauthn.properties + .properties)
            }
          )
        else
          [.]
        end
      ) | flatten
    )
'

# Ensure the output directory exists
mkdir -p "$(dirname "$type_output")"

# Step 1: Set schema
schema=$(jq "$jq_filter" "$existing_json")

# Step 2: Pass the modified JSON directly to json-schema-to-typescript
echo "$schema" | npm_config_registry="https://registry.npmjs.org" npx json-schema-to-typescript -o "$type_output" --unreachableDefinitions

echo "Types generated successfully in $type_output"

