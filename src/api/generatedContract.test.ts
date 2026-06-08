import { describe, expect, it } from "vitest";
import generatedContractSource from "./generated/mpgsServerApi.ts?raw";

describe("generated MPGS server API contract", () => {
  it("exposes TypeScript types generated from the Rust OpenAPI document", () => {
    expect(generatedContractSource).toContain(
      "Generated from docs/openapi/mpgs-server.openapi.json",
    );
    expect(generatedContractSource).toContain("export interface ServiceInfo");
    expect(generatedContractSource).toContain(
      'export type ServiceCapability = "public_catalog_read";',
    );
    expect(generatedContractSource).toContain("export interface DiscoveryHomeResponse");
    expect(generatedContractSource).toContain("export interface PublicGamesPage");
    expect(generatedContractSource).toContain("export interface HealthResponse");
  });
});
