import { describe, expect, it } from "vitest";
import {
  buildServiceInfoUrl,
  evaluateServiceAddressPolicy,
  evaluateServiceInfoCompatibility,
  normalizeServiceBaseUrl,
  validateServiceAddress,
} from "./serviceConnection";
import type { ServiceInfo } from "../types";

const compatibleInfo: ServiceInfo = {
  serviceInstanceId: "018fb770-8998-7699-a6e4-b7b59f2f9c01",
  serviceName: "MPGS Test Service",
  serviceVersion: "0.1.0",
  apiVersion: "v1",
  publicCatalogStatus: "empty",
  capabilities: ["public_catalog_read"],
};

describe("service connection validation model", () => {
  it("normalizes a base URL and builds the service-info endpoint", () => {
    expect(normalizeServiceBaseUrl(" https://mpgs.example.test/// ")).toBe(
      "https://mpgs.example.test",
    );
    expect(buildServiceInfoUrl("https://mpgs.example.test/")).toBe(
      "https://mpgs.example.test/api/v1/service-info",
    );
  });

  it("accepts compatible v1 service identity information", () => {
    expect(evaluateServiceInfoCompatibility(compatibleInfo)).toEqual({
      compatible: true,
      reason: "服务兼容。",
      info: compatibleInfo,
    });
  });

  it("rejects missing public read capability and incompatible API versions", () => {
    expect(
      evaluateServiceInfoCompatibility({
        ...compatibleInfo,
        apiVersion: "v2",
      }),
    ).toMatchObject({
      compatible: false,
      reason: "当前客户端只支持 MPGS API v1。",
    });

    expect(
      evaluateServiceInfoCompatibility({
        ...compatibleInfo,
        capabilities: [],
      }),
    ).toMatchObject({
      compatible: false,
      reason: "服务未声明公共库只读能力。",
    });
  });

  it("allows HTTPS and local HTTP but rejects public HTTP by default", () => {
    expect(evaluateServiceAddressPolicy("https://mpgs.example.test")).toMatchObject({
      allowed: true,
    });
    expect(evaluateServiceAddressPolicy("http://localhost:4310")).toMatchObject({
      allowed: true,
    });
    expect(evaluateServiceAddressPolicy("http://192.168.1.10:4310")).toMatchObject({
      allowed: false,
      reason: "公网服务地址必须使用 HTTPS。",
    });
    expect(
      evaluateServiceAddressPolicy("http://192.168.1.10:4310", {
        allowPrivateHttp: true,
      }),
    ).toMatchObject({
      allowed: true,
    });
  });

  it("fetches service-info and returns a validation result", async () => {
    const seenUrls: string[] = [];
    const result = await validateServiceAddress(
      "https://mpgs.example.test/",
      async (url) => {
        seenUrls.push(url);
        return {
          ok: true,
          status: 200,
          json: async () => compatibleInfo,
        };
      },
    );

    expect(seenUrls).toEqual(["https://mpgs.example.test/api/v1/service-info"]);
    expect(result).toMatchObject({
      success: true,
      message: "服务地址验证通过。",
      baseUrl: "https://mpgs.example.test",
    });
  });
});
