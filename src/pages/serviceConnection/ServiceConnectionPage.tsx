import { useState } from "react";
import { validateServiceAddress } from "../../domain/serviceConnection";
import type { ServiceAddressValidationResult } from "../../types";
import "./ServiceConnectionPage.css";

export interface ServiceConnectionPageProps {
  onConnected: (result: ServiceAddressValidationResult) => void;
  onImportFile: (fileText: string) => void;
}

export function ServiceConnectionPage({
  onConnected,
  onImportFile,
}: ServiceConnectionPageProps) {
  const [address, setAddress] = useState("");
  const [isValidating, setIsValidating] = useState(false);
  const [validationResult, setValidationResult] =
    useState<ServiceAddressValidationResult | null>(null);
  const [allowPrivateHttp, setAllowPrivateHttp] = useState(false);

  async function handleValidate() {
    if (!address.trim()) {
      setValidationResult({
        success: false,
        message: "请输入服务地址。",
      });
      return;
    }

    setIsValidating(true);
    setValidationResult(null);

    try {
      const result = await validateServiceAddress(address, undefined, {
        allowPrivateHttp,
      });
      setValidationResult(result);
      if (result.success) {
        onConnected(result);
      }
    } catch (error) {
      setValidationResult({
        success: false,
        message: error instanceof Error ? error.message : String(error),
      });
    } finally {
      setIsValidating(false);
    }
  }

  async function handleImportFile(file: File | null) {
    if (!file) {
      return;
    }

    try {
      const fileText = await file.text();
      onImportFile(fileText);
    } catch (error) {
      setValidationResult({
        success: false,
        message: `读取文件失败：${error instanceof Error ? error.message : String(error)}`,
      });
    }
  }

  function handleKeyDown(event: React.KeyboardEvent<HTMLInputElement>) {
    if (event.key === "Enter" && !isValidating) {
      void handleValidate();
    }
  }

  return (
    <main className="service-connection-page">
      <div className="service-connection-card">
        <div className="service-connection-header">
          <span className="service-connection-icon">⚡</span>
          <h1>连接 MPGS 服务</h1>
          <p>输入公共发现服务地址以开始使用</p>
        </div>

        <div className="service-connection-body">
          <div className="service-connection-input-group">
            <label htmlFor="service-address">服务地址</label>
            <input
              id="service-address"
              type="text"
              value={address}
              onChange={(e) => setAddress(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="https://example.com"
              disabled={isValidating}
              autoFocus
            />
          </div>

          <div className="service-connection-options">
            <label className="service-connection-checkbox">
              <input
                type="checkbox"
                checked={allowPrivateHttp}
                onChange={(e) => setAllowPrivateHttp(e.target.checked)}
                disabled={isValidating}
              />
              <span>允许局域网 HTTP 地址（不推荐用于生产）</span>
            </label>
          </div>

          <button
            className="service-connection-primary-button"
            onClick={() => void handleValidate()}
            disabled={isValidating || !address.trim()}
          >
            {isValidating ? "正在验证连接…" : "连接"}
          </button>

          {validationResult && (
            <div
              className={`service-connection-result ${
                validationResult.success ? "success" : "error"
              }`}
            >
              <strong>
                {validationResult.success ? "✓ 验证成功" : "✗ 验证失败"}
              </strong>
              <p>{validationResult.message}</p>
              {!validationResult.success && validationResult.diagnostic && (
                <p className="service-connection-diagnostic">
                  {validationResult.diagnostic}
                </p>
              )}
              {validationResult.success && validationResult.info && (
                <div className="service-connection-info">
                  <div>
                    <span>服务名称</span>
                    <strong>{validationResult.info.serviceName}</strong>
                  </div>
                  <div>
                    <span>实例 ID</span>
                    <strong>{validationResult.info.serviceInstanceId}</strong>
                  </div>
                  <div>
                    <span>API 版本</span>
                    <strong>{validationResult.info.apiVersion}</strong>
                  </div>
                  <div>
                    <span>公共库状态</span>
                    <strong>
                      {formatPublicCatalogStatus(
                        validationResult.info.publicCatalogStatus
                      )}
                    </strong>
                  </div>
                </div>
              )}
            </div>
          )}

          <div className="service-connection-divider">
            <span>或</span>
          </div>

          <div className="service-connection-import">
            <label htmlFor="service-connection-file" className="service-connection-file-label">
              导入服务连接文件
            </label>
            <input
              id="service-connection-file"
              type="file"
              accept="application/json,.json"
              onChange={(e) => {
                const file = e.target.files?.[0] ?? null;
                e.target.value = "";
                void handleImportFile(file);
              }}
              disabled={isValidating}
            />
          </div>

          <div className="service-connection-hints">
            <h3>连接要求</h3>
            <ul>
              <li>公网服务地址必须使用 HTTPS</li>
              <li>localhost 地址允许使用 HTTP（开发用途）</li>
              <li>局域网 HTTP 地址需要手动启用</li>
              <li>服务必须支持 API v1 和公共库只读能力</li>
            </ul>
          </div>
        </div>
      </div>
    </main>
  );
}

function formatPublicCatalogStatus(status: string): string {
  switch (status) {
    case "ready":
      return "就绪";
    case "empty":
      return "空库";
    case "updating":
      return "更新中";
    case "unavailable":
      return "不可用";
    default:
      return status;
  }
}
