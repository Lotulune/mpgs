import type { ServiceInfo } from "../types";
import {
  evaluateServiceInfoCompatibility,
  normalizeServiceBaseUrl,
} from "./serviceConnection";

export interface CurrentServiceConnection {
  baseUrl: string;
  info: ServiceInfo;
  validatedAt: string;
}

export const CURRENT_SERVICE_CONNECTION_STORAGE_KEY =
  "mpgs.currentServiceConnection.v1";
export const RECENT_SERVICE_CONNECTIONS_STORAGE_KEY =
  "mpgs.recentServiceConnections.v1";
const RECENT_SERVICE_CONNECTIONS_LIMIT = 5;

export function getCurrentServiceConnection(): CurrentServiceConnection | null {
  const storage = getStorage();
  if (!storage) {
    return null;
  }

  const rawValue = storage.getItem(CURRENT_SERVICE_CONNECTION_STORAGE_KEY);
  if (!rawValue) {
    return null;
  }

  try {
    const parsed = JSON.parse(rawValue);
    if (!isCurrentServiceConnection(parsed)) {
      return null;
    }

    return normalizeCurrentServiceConnection(parsed);
  } catch {
    return null;
  }
}

export function getRecentServiceConnections(): CurrentServiceConnection[] {
  const storage = getStorage();
  if (!storage) {
    return [];
  }

  const rawValue = storage.getItem(RECENT_SERVICE_CONNECTIONS_STORAGE_KEY);
  if (!rawValue) {
    return [];
  }

  try {
    const parsed = JSON.parse(rawValue);
    if (!Array.isArray(parsed)) {
      return [];
    }

    return deduplicateRecentConnections(
      parsed
        .filter(isCurrentServiceConnection)
        .map(normalizeCurrentServiceConnection),
    ).slice(0, RECENT_SERVICE_CONNECTIONS_LIMIT);
  } catch {
    return [];
  }
}

export function saveCurrentServiceConnection(connection: CurrentServiceConnection) {
  const storage = getStorage();
  if (!storage) {
    return;
  }

  const normalizedConnection = normalizeCurrentServiceConnection(connection);
  if (!isCompatibleServiceInfo(normalizedConnection.info)) {
    throw new Error("服务身份信息格式不兼容。");
  }

  storage.setItem(
    CURRENT_SERVICE_CONNECTION_STORAGE_KEY,
    JSON.stringify(normalizedConnection),
  );
  storage.setItem(
    RECENT_SERVICE_CONNECTIONS_STORAGE_KEY,
    JSON.stringify(upsertRecentConnection(normalizedConnection)),
  );
}

export function clearCurrentServiceConnection() {
  getStorage()?.removeItem(CURRENT_SERVICE_CONNECTION_STORAGE_KEY);
}

function getStorage(): Storage | null {
  if (typeof window === "undefined") {
    return null;
  }

  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

function isCurrentServiceConnection(value: unknown): value is CurrentServiceConnection {
  if (!value || typeof value !== "object") {
    return false;
  }

  const candidate = value as Record<string, unknown>;
  return (
    typeof candidate.baseUrl === "string" &&
    typeof candidate.validatedAt === "string" &&
    isCompatibleServiceInfo(candidate.info)
  );
}

function normalizeCurrentServiceConnection(
  connection: CurrentServiceConnection,
): CurrentServiceConnection {
  return {
    baseUrl: normalizeServiceBaseUrl(connection.baseUrl),
    info: connection.info,
    validatedAt: connection.validatedAt,
  };
}

function upsertRecentConnection(
  connection: CurrentServiceConnection,
): CurrentServiceConnection[] {
  return [
    connection,
    ...getRecentServiceConnections().filter(
      (recentConnection) =>
        recentConnection.info.serviceInstanceId !== connection.info.serviceInstanceId,
    ),
  ].slice(0, RECENT_SERVICE_CONNECTIONS_LIMIT);
}

function deduplicateRecentConnections(
  connections: CurrentServiceConnection[],
): CurrentServiceConnection[] {
  const seenServiceInstanceIds = new Set<string>();
  const deduplicatedConnections: CurrentServiceConnection[] = [];

  for (const connection of connections) {
    if (seenServiceInstanceIds.has(connection.info.serviceInstanceId)) {
      continue;
    }

    seenServiceInstanceIds.add(connection.info.serviceInstanceId);
    deduplicatedConnections.push(connection);
  }

  return deduplicatedConnections;
}

function isCompatibleServiceInfo(value: unknown): value is ServiceInfo {
  if (!value || typeof value !== "object") {
    return false;
  }

  const candidate = value as Record<string, unknown>;
  if (
    typeof candidate.serviceInstanceId !== "string" ||
    typeof candidate.serviceName !== "string" ||
    typeof candidate.serviceVersion !== "string" ||
    candidate.apiVersion !== "v1" ||
    !isPublicCatalogStatus(candidate.publicCatalogStatus) ||
    !Array.isArray(candidate.capabilities) ||
    !candidate.capabilities.every((capability) => capability === "public_catalog_read")
  ) {
    return false;
  }

  const serviceInfo: ServiceInfo = {
    serviceInstanceId: candidate.serviceInstanceId,
    serviceName: candidate.serviceName,
    serviceVersion: candidate.serviceVersion,
    apiVersion: candidate.apiVersion,
    publicCatalogStatus: candidate.publicCatalogStatus,
    capabilities: candidate.capabilities,
  } as ServiceInfo;

  return evaluateServiceInfoCompatibility(serviceInfo).compatible;
}

function isPublicCatalogStatus(value: unknown): value is ServiceInfo["publicCatalogStatus"] {
  return (
    value === "empty" ||
    value === "ready" ||
    value === "updating" ||
    value === "unavailable"
  );
}
