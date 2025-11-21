import * as fs from 'node:fs';
import * as path from 'node:path';
import {compile} from 'json-schema-to-typescript';
import type {JSONSchema} from 'json-schema-to-typescript';

// ==========================================
// CONFIGURATION
// ==========================================
const INPUT_PATH = '../../target/near/aurora_launchpad_contract/aurora_launchpad_contract_abi.json';
const OUTPUT_PATH = 'schema.ts';

// ==========================================
// TYPES FOR RAW ABI (Partial)
// ==========================================
interface AbiParam {
    name: string;
    type_schema: JSONSchema;
}

interface AbiFunction {
    name: string;
    kind: 'view' | 'call';
    doc?: string;
    params?: {
        args: AbiParam[];
    };
    result?: {
        type_schema: JSONSchema;
    };
}

interface NearAbi {
    body: {
        functions: AbiFunction[];
        root_schema: JSONSchema & { definitions: Record<string, JSONSchema> };
    };
}

// ==========================================
// HELPERS
// ==========================================

function toPascalCase(str: string): string {
    return str.replace(/(?:^|_)(.)/g, (_, c) => c.toUpperCase());
}

/**
 * FIX 1: WebAuthn oneOf fix
 */
function fixWebAuthnOneOf(definitions: Record<string, JSONSchema>) {
    if (!definitions['MultiPayload'] || !definitions['MultiPayload'].oneOf) {
        return;
    }

    const oneOf = definitions['MultiPayload'].oneOf as JSONSchema[];
    const newOneOf: JSONSchema[] = [];

    for (const schema of oneOf) {
        const isWebAuthn = schema.properties?.['standard']?.enum?.includes('webauthn');

        if (isWebAuthn && schema.anyOf) {
            const flattenedVariants = schema.anyOf.map((variant: JSONSchema) => {
                return {
                    type: 'object',
                    description: schema.description,
                    required: [...((schema.required as string[]) || []), ...((variant.required as string[]) || [])],
                    properties: {
                        ...(schema.properties || {}),
                        ...(variant.properties || {}),
                    },
                } as JSONSchema;
            });
            newOneOf.push(...flattenedVariants);
        } else {
            newOneOf.push(schema);
        }
    }

    definitions['MultiPayload'].oneOf = newOneOf;
}

/**
 * FIX 2: Boolean Schema Sanitizer
 */
function sanitizeBooleanSchemas(definitions: Record<string, JSONSchema>) {
    for (const key in definitions) {
        const val = definitions[key];
        if (typeof (val as any) === 'boolean' && (val as any) === true) {
            // console.log(`Sanitizing boolean definition for: "${key}" -> {}`);
            definitions[key] = {};
        }
    }
}

/**
 * INJECTOR: Creates Args and Result types in definitions
 */
function injectFunctionTypes(
    definitions: Record<string, JSONSchema>,
    functions: AbiFunction[]
) {
    for (const func of functions) {
        const pascalName = toPascalCase(func.name);

        // 1. Generate Args Interface
        if (func.params?.args) {
            definitions[`${pascalName}Args`] = {
                type: 'object',
                additionalProperties: false,
                description: func.doc || `Arguments for method: ${func.name}`,
                required: func.params.args.map((arg) => arg.name),
                properties: func.params.args.reduce((acc, arg) => {
                    acc[arg.name] = (typeof (arg.type_schema as any) === 'boolean')
                        ? {}
                        : arg.type_schema;
                    return acc;
                }, {} as Record<string, JSONSchema>),
            };


        }

        // 2. Generate Result Type
        if (func.result?.type_schema) {
            const rawSchema = func.result.type_schema as any;

            let resultSchema = func.result.type_schema;
            if (typeof rawSchema === 'boolean') {
                resultSchema = {};
            }


            const finalSchema = {...resultSchema};
            finalSchema.description = `Return value for method: ${func.name}`;
            definitions[`${pascalName}Result`] = finalSchema;
        }
    }
}

// ==========================================
// MAIN SCRIPT
// ==========================================
async function main() {
    console.log(`Reading ABI from ${INPUT_PATH}...`);

    const rawFile = fs.readFileSync(INPUT_PATH, 'utf-8');
    const abi = JSON.parse(rawFile) as NearAbi;

    // Берем definitions, но НЕ берем саму root_schema (которая type: string)
    const definitions = abi.body.root_schema.definitions || {};
    const functions = abi.body.functions;

    console.log('Applying schema fixes...');

    // 1. Fixes
    sanitizeBooleanSchemas(definitions);
    fixWebAuthnOneOf(definitions);

    // 2. Injection
    console.log('Injecting function argument types...');
    injectFunctionTypes(definitions, functions);

    // 3. СОЗДАЕМ НОВУЮ ОБЕРТКУ (Ключевое исправление)
    // Мы создаем пустой объект, который служит лишь контейнером для definitions.
    const schemaWrapper: JSONSchema = {
        title: 'AuroraLaunchpadContract',
        type: 'object',
        additionalProperties: false,
        definitions: definitions
    };

    console.log('Compiling to TypeScript...');
    const tsCode = await compile(schemaWrapper, 'AuroraLaunchpad', {
        bannerComment: '/* tslint:disable */\n/**\n * This file was automatically generated by json-schema-to-typescript.\n * DO NOT MODIFY IT BY HAND.\n */',
        unreachableDefinitions: true, // Это заставит сгенерировать всё, что лежит в definitions
        style: {
            singleQuote: true,
            bracketSpacing: true
        }
    });

    console.log('Appending runtime metadata...');
    const metadataCode = `
// ==========================================
// RUNTIME CONSTANTS
// ==========================================
export const AuroraLaunchpadMethods = ${JSON.stringify(functions, null, 2)} as const;
`;

    const finalOutput = tsCode + metadataCode;

    const dir = path.dirname(OUTPUT_PATH);
    if (!fs.existsSync(dir)) {
        fs.mkdirSync(dir, {recursive: true});
    }

    fs.writeFileSync(OUTPUT_PATH, finalOutput);
    console.log(`✅ Successfully generated types and metadata to: ${OUTPUT_PATH}`);
}

main().catch((err) => {
    console.error('Error generating types:', err);
    process.exit(1);
});
