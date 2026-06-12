import { beforeEach, describe, expect, it } from "vitest";
import type { ServiceInfo } from "../types";
import {
  clearCurrentServiceConnection,
  getCurrentServiceConnection,
  getRecentServiceConnections,
  saveCurrentServiceConnection,
} from "./serviceConnectionStorage";

const RECENT_SERVICE_CONNECTIONS_STORAGE_KEY =
  "mpgs.recentServiceConnections.v1";

function buildCompatibleInfo(
  serviceInstanceId = "018fb770-8998-7699-a6e4-b7b59f2f9c01",
  serviceName = "MPGS Test Service",
): ServiceInfo {
  return {
    serviceInstanceId,
    serviceName,
    serviceVersion: "0.1.0",
    apiVersion: "v1",
    publicCatalogStatus: "ready",
    capabilities: ["public_catalog_read"],
  };
}

function buildConnection(index: number) {
  const names = [
    "MPGS Test Service",
    "MPGS Secondary Service",
    "MPGS Tertiary Service",
    "MPGS Fourth Service",
    "MPGS Fifth Service",
    "MPGS Sixth Service",
  ];

  return {
    baseUrl: ` https://mpgs-${index}.example.test/// `,
    info: buildCompatibleInfo(
      `018fb770-8998-7699-a6e4-b7b59f2f9c0${index}`,
      names[index - 1],
    ),
    validatedAt: `2026-06-08T00:0${index}:00.000Z`,
  };
}

const compatibleInfo = buildCompatibleInfo();
const currentConnection = {
  baseUrl: " https://mpgs.example.test/// ",
  info: compatibleInfo,
  validatedAt: "2026-06-08T00:00:00.000Z",
};
const normalizedCurrentConnection = {
  baseUrl: "https://mpgs.example.test",
  info: compatibleInfo,
  validatedAt: "2026-06-08T00:00:00.000Z",
};
const secondaryConnection = {
  ...buildConnection(2),
  baseUrl: "https://secondary.example.test///",
};
const normalizedSecondaryConnection = {
  baseUrl: "https://secondary.example.test",
  info: secondaryConnection.info,
  validatedAt: secondaryConnection.validatedAt,
};
const updatedCurrentConnection = {
  baseUrl: "https://mpgs-renamed.example.test///",
  info: {
    ...compatibleInfo,
    serviceName: "MPGS Renamed Service",
  },
  validatedAt: "2026-06-09T00:00:00.000Z",
};
const normalizedUpdatedCurrentConnection = {
  baseUrl: "https://mpgs-renamed.example.test",
  info: updatedCurrentConnection.info,
  validatedAt: "2026-06-09T00:00:00.000Z",
};

describe("service connection storage", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("stores the single current service connection with a normalized base URL", () => {
    saveCurrentServiceConnection(currentConnection);

    expect(getCurrentServiceConnection()).toEqual(normalizedCurrentConnection);
  });

  it("ignores incompatible stored data instead of returning a partial connection", () => {
    localStorage.setItem(
      "mpgs.currentServiceConnection.v1",
      JSON.stringify({
        baseUrl: "https://mpgs.example.test",
        info: { apiVersion: "v2" },
        validatedAt: "2026-06-08T00:00:00.000Z",
      }),
    );

    expect(getCurrentServiceConnection()).toBeNull();
  });

  it("stores recent service connections with the newest connection first", () => {
    saveCurrentServiceConnection(currentConnection);
    saveCurrentServiceConnection(secondaryConnection);

    expect(getRecentServiceConnections()).toEqual([
      normalizedSecondaryConnection,
      normalizedCurrentConnection,
    ]);
  });

  it("updates an existing recent service connection and moves it to the front", () => {
    saveCurrentServiceConnection(currentConnection);
    saveCurrentServiceConnection(secondaryConnection);
    saveCurrentServiceConnection(updatedCurrentConnection);

    expect(getRecentServiceConnections()).toEqual([
      normalizedUpdatedCurrentConnection,
      normalizedSecondaryConnection,
    ]);
  });

  it("keeps recent service history after clearing the current connection", () => {
    saveCurrentServiceConnection(currentConnection);

    clearCurrentServiceConnection();

    expect(getCurrentServiceConnection()).toBeNull();
    expect(getRecentServiceConnections()).toEqual([normalizedCurrentConnection]);
  });

  it("keeps only the five most recent service connections", () => {
    [1, 2, 3, 4, 5, 6].forEach((index) => {
      saveCurrentServiceConnection(buildConnection(index));
    });

    expect(getRecentServiceConnections().map((connection) => connection.info.serviceName)).toEqual([
      "MPGS Sixth Service",
      "MPGS Fifth Service",
      "MPGS Fourth Service",
      "MPGS Tertiary Service",
      "MPGS Secondary Service",
    ]);
  });

  it("ignores incompatible recent service history entries", () => {
    localStorage.setItem(
      RECENT_SERVICE_CONNECTIONS_STORAGE_KEY,
      JSON.stringify([
        {
          baseUrl: "https://mpgs.example.test",
          info: { apiVersion: "v2" },
          validatedAt: "2026-06-08T00:00:00.000Z",
        },
        secondaryConnection,
      ]),
    );

    expect(getRecentServiceConnections()).toEqual([normalizedSecondaryConnection]);
  });

  it("returns no recent service history for malformed stored data", () => {
    localStorage.setItem(RECENT_SERVICE_CONNECTIONS_STORAGE_KEY, "{not-json");

    expect(getRecentServiceConnections()).toEqual([]);
  });
});
