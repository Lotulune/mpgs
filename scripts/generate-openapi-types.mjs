import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { dirname, relative } from "node:path";

const [, , inputPath = "docs/openapi/mpgs-server.openapi.json", outputPath = "src/api/generated/mpgsServerApi.ts"] =
  process.argv;

const openapi = JSON.parse(readFileSync(inputPath, "utf8"));
const schemas = openapi.components?.schemas ?? {};

const lines = [
  "/* eslint-disable */",
  "// This file is generated. Do not edit by hand.",
  `// Generated from ${inputPath.replaceAll("\\", "/")}.`,
  "",
];

for (const [name, schema] of Object.entries(schemas).sort(([left], [right]) =>
  left.localeCompare(right),
)) {
  lines.push(renderSchema(name, schema), "");
}

mkdirSync(dirname(outputPath), { recursive: true });
writeFileSync(outputPath, `${lines.join("\n").trimEnd()}\n`, "utf8");

console.log(`Generated ${relative(process.cwd(), outputPath)} from ${relative(process.cwd(), inputPath)}`);

function renderSchema(name, schema) {
  if (schema.type === "object" || schema.properties) {
    return renderInterface(name, schema);
  }

  return `export type ${name} = ${renderType(schema)};`;
}

function renderInterface(name, schema) {
  const required = new Set(schema.required ?? []);
  const properties = schema.properties ?? {};
  const body = Object.entries(properties)
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([propertyName, propertySchema]) => {
      const optional = required.has(propertyName) ? "" : "?";
      return `  ${propertyName}${optional}: ${renderType(propertySchema)};`;
    });

  if (body.length === 0) {
    body.push("  [key: string]: unknown;");
  }

  return [`export interface ${name} {`, ...body, "}"].join("\n");
}

function renderType(schema) {
  if (!schema || Object.keys(schema).length === 0) {
    return "unknown";
  }

  if (schema.$ref) {
    return refName(schema.$ref);
  }

  if (schema.oneOf) {
    return union(schema.oneOf.map(renderType));
  }

  if (schema.anyOf) {
    return union(schema.anyOf.map(renderType));
  }

  if (Array.isArray(schema.type)) {
    return union(schema.type.map((type) => renderType({ ...schema, type })));
  }

  if (schema.enum) {
    return union(schema.enum.map((value) => JSON.stringify(value)));
  }

  switch (schema.type) {
    case "array":
      return `${renderType(schema.items)}[]`;
    case "boolean":
      return "boolean";
    case "integer":
    case "number":
      return "number";
    case "null":
      return "null";
    case "object":
      if (schema.properties) {
        const required = new Set(schema.required ?? []);
        const fields = Object.entries(schema.properties)
          .sort(([left], [right]) => left.localeCompare(right))
          .map(([propertyName, propertySchema]) => {
            const optional = required.has(propertyName) ? "" : "?";
            return `${propertyName}${optional}: ${renderType(propertySchema)}`;
          });
        return `{ ${fields.join("; ")} }`;
      }
      if (schema.additionalProperties) {
        return `Record<string, ${renderType(schema.additionalProperties)}>`;
      }
      return "Record<string, unknown>";
    case "string":
      return "string";
    default:
      return "unknown";
  }
}

function refName(ref) {
  const prefix = "#/components/schemas/";
  if (!ref.startsWith(prefix)) {
    throw new Error(`Unsupported OpenAPI ref: ${ref}`);
  }
  return ref.slice(prefix.length);
}

function union(types) {
  return [...new Set(types)].join(" | ");
}
