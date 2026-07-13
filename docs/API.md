# HTTP API 契约

## 1. 约定

- Base path：`/v1`
- 传输：HTTPS；本地开发可使用 HTTP。
- 编码：UTF-8 JSON，时间使用 UTC RFC 3339 字符串；数据库内部时间格式不暴露。
- 字段命名：`snake_case`。
- 未知响应字段客户端必须忽略；删除或改变字段语义需要新 API 版本。
- OpenAPI 是外部契约来源，Rust DTO 与 OpenAPI 从同一类型生成。
- 所有响应返回 `x-request-id`。

## 2. 鉴权

### 2.1 匿名用户

首次调用 `POST /v1/session/anonymous`，服务端返回短期访问令牌和轮换令牌。服务端只保存令牌哈希。

```json
{
  "access_token": "opaque",
  "expires_at": "2026-07-14T10:00:00Z",
  "refresh_token": "opaque",
  "user_id": "019..."
}
```

访问令牌通过 `Authorization: Bearer <token>` 发送。公开目录端点可允许无令牌读取，但偏好、反馈和 AI 配额必须关联匿名会话。

### 2.2 管理用户

管理 API 使用独立身份系统和 audience，不接受匿名用户令牌。MVP 可先使用部署层提供的管理员凭据，但必须支持审计到具体操作者。

## 3. 通用错误

```json
{
  "error": {
    "code": "invalid_argument",
    "message": "party_size must be between 1 and 64",
    "request_id": "019...",
    "details": {
      "field": "party_size"
    }
  }
}
```

稳定错误码：

| HTTP | code | 说明 |
| --- | --- | --- |
| 400 | `invalid_argument` | 输入格式或范围错误 |
| 401 | `unauthenticated` | 无有效令牌 |
| 403 | `forbidden` | 权限不足 |
| 404 | `not_found` | AppID 或资源不存在 |
| 409 | `version_conflict` | 偏好版本或幂等请求冲突 |
| 422 | `unsupported_constraint` | 输入合法但 MVP 不支持该约束 |
| 429 | `rate_limited` | 超过设备/IP/全局配额 |
| 500 | `internal` | 未分类内部错误 |
| 503 | `temporarily_unavailable` | 数据库迁移、只读降级或无可用数据 |

AI 失败通常不返回 5xx，而是以成功响应中的 `ai_status=fallback` 表达。

## 4. 游标分页

请求：

```text
?limit=20&cursor=<opaque>
```

响应：

```json
{
  "items": [],
  "next_cursor": "opaque-or-null",
  "snapshot_at": "2026-07-13T12:00:00Z"
}
```

游标包含签名后的排序键、快照和查询哈希。客户端不得解析。`limit` 默认 20，最大 100。

## 5. 缓存与一致性

- 推荐流、游戏详情、日历和元数据返回 `ETag`。
- 客户端可发送 `If-None-Match`，服务端返回 `304`。
- 偏好更新使用 `version` 乐观并发控制。
- 反馈写入支持 `Idempotency-Key`，相同键与相同请求返回原结果；相同键与不同请求返回 `409`。
- 响应明确 `data_updated_at` 和 `algorithm_version`，避免把缓存时间当作数据时间。

## 6. 健康与元数据

### `GET /health/live`

只表示进程可响应，不检查外部依赖。

### `GET /health/ready`

检查迁移版本、数据库可读、当前算法配置和最小目录快照。AI/Steam 暂时不可用不应使前台 API 不 ready。

### `GET /v1/meta`

```json
{
  "api_version": "v1",
  "service_version": "0.1.0",
  "algorithm_version": "rules-0.1.0",
  "supported_sections": [
    "recent_release",
    "upcoming",
    "popular_legacy",
    "classic_legacy"
  ],
  "ai_available": true
}
```

## 7. 偏好

### `GET /v1/preferences`

### `PUT /v1/preferences`

```json
{
  "version": 3,
  "party_size": 4,
  "coop_competitive": 0.15,
  "session_minutes_min": 30,
  "session_minutes_max": 180,
  "budget": {
    "currency": "CNY",
    "max_each_minor": 15000
  },
  "platforms": ["windows"],
  "self_hosting_willingness": 0.7,
  "languages": ["schinese", "english"],
  "excluded_modes": ["mmo"]
}
```

`coop_competitive=0` 表示纯合作偏好，`1` 表示强竞技偏好。响应返回递增后的 `version`。

## 8. 推荐流

### `GET /v1/feeds/{section}`

`section`：

- `recent_release`
- `upcoming`
- `popular_legacy`
- `classic_legacy`

查询参数：

```text
limit, cursor, party_size, platforms, demo_only, max_price_minor, currency
```

响应条目：

```json
{
  "app_id": 548430,
  "name": "Deep Rock Galactic",
  "section": "classic_legacy",
  "score": 0.91,
  "confidence": 0.92,
  "cover_url": "https://...",
  "release_date": "2020-05-13",
  "price": {
    "country": "CN",
    "currency": "CNY",
    "current_minor": 9000,
    "original_minor": 9000,
    "discount_percent": 0,
    "captured_at": "2026-07-13T10:00:00Z"
  },
  "party": {
    "recommended_min": 2,
    "recommended_max": 4,
    "hard_max": 4
  },
  "multiplayer": {
    "dominant_mode": "private_coop",
    "private_session": true,
    "self_hosted_server": false,
    "public_population_dependency": "low"
  },
  "reasons": ["支持私人四人合作", "累计口碑稳定"],
  "cautions": ["高难度任务需要配合"],
  "evidence_ids": ["feature:online_coop:548430"],
  "algorithm_version": "rules-0.1.0",
  "data_updated_at": "2026-07-13T10:00:00Z"
}
```

## 9. 发售日历

### `GET /v1/calendar`

```text
?from=2026-07-01&to=2026-12-31&demo=true&party_size=4
```

`from/to` 最大跨度一年。日期不精确的条目进入 `undated_items`，不能伪造具体日期。

```json
{
  "dated_items": [],
  "undated_items": [],
  "data_updated_at": "2026-07-13T10:00:00Z"
}
```

## 10. 搜索

### `GET /v1/search`

用于名称和普通全文搜索：

```text
?q=deep+rock&party_size=4&limit=20
```

不调用在线 AI。可使用 FTS 和确定性排序。

### `POST /v1/search/semantic`

用于自然语言混合检索，但不要求生成长解释：

```json
{
  "query": "三个人一小时左右、不太卷、可以反复刷",
  "limit": 20,
  "use_ai_intent_parser": true
}
```

Embedding 或 AI 意图解析不可用时回退到 FTS 和当前偏好。

## 11. AI 推荐

### `POST /v1/recommendations/query`

```json
{
  "query": "四个人长期玩，能自己开服，预算每人一百五十元",
  "max_results": 10,
  "include_sections": ["recent_release", "popular_legacy", "classic_legacy"],
  "request_ai_analysis": true
}
```

响应：

```json
{
  "recommendations": [],
  "parsed_intent": {
    "party_size": 4,
    "self_hosting": "preferred",
    "budget": { "currency": "CNY", "max_each_minor": 15000 }
  },
  "ai_status": "used",
  "ai_notice": null,
  "algorithm_version": "rules-0.1.0",
  "data_updated_at": "2026-07-13T10:00:00Z"
}
```

`ai_status`：`used`、`cached`、`disabled`、`fallback`。即使 `fallback`，HTTP 状态仍可为 200。

## 12. 游戏详情与证据

### `GET /v1/games/{app_id}`

返回：

- 本地化基础信息、商店链接和关联 Demo。
- 联机画像、人数、连接方式、服务依赖和可信度。
- 生命周期/近期评价、7/28 日 CCU 聚合和价格。
- 推荐分项、用户适配项、风险与更新时间。

### `GET /v1/games/{app_id}/evidence`

默认返回对最终推荐产生影响的公开证据摘要，不返回内部敏感备注。支持 `?feature=private_session`。

```json
{
  "items": [
    {
      "evidence_id": "feature:private_session:548430",
      "feature": "private_session",
      "value": true,
      "source_type": "official_store",
      "source_label": "Steam store feature",
      "confidence": 0.9,
      "observed_at": "2026-07-13T10:00:00Z"
    }
  ]
}
```

## 13. 反馈

### `POST /v1/feedback`

请求必须包含 `Idempotency-Key`：

```json
{
  "app_id": 548430,
  "type": "like",
  "recommendation_run_id": "019...",
  "client_created_at": "2026-07-13T11:30:00Z"
}
```

`type`：

- `like`
- `not_interested`
- `played`
- `too_competitive`
- `party_size_mismatch`
- `hosting_friction`

### `POST /v1/feedback/{feedback_id}/undo`

追加撤销事件，不物理删除原记录。

## 14. 同步

### `GET /v1/sync`

用于客户端增量获取偏好版本、已变更缓存实体和服务端建议失效列表：

```text
?since=<opaque_sync_cursor>
```

MVP 可以先按推荐流和详情分别使用 ETag；统一 sync 端点属于 P1，不能阻塞首个垂直切片。

## 15. 内部采集 API

内部路由使用 `/internal/v1`，不出现在公开客户端 OpenAPI 中。  
M2 最小实现要求 `Authorization: Bearer <MPGS_ADMIN_TOKEN>`（与管理 API 共用部署令牌；后续可拆分 audience）。

### `POST /internal/v1/jobs/enqueue`（M2）

入队采集任务；`idempotency_key` 唯一，重复提交返回已有 `job_id`。

### `POST /internal/v1/jobs/lease`（M2）

采集节点领取限定数量、可选 `source` 过滤的任务。请求体：`owner`、`limit`、`lease_ms`、`source`。

### `POST /internal/v1/jobs/{job_id}/complete`（M2）

验证租约持有者与幂等键后标记完成；同键重复完成返回成功。

### `POST /internal/v1/jobs/{job_id}/fail`（M2）

错误必须使用稳定类别：`network`、`rate_limited`、`auth`、`not_found`、`parse_changed`、`invalid_payload`。可重试错误按 `retry_delay_ms` 回到 `pending`；否则进入 `dead`。

## 16. 管理 API

管理路由使用 `/admin/v1`，Bearer 使用 `MPGS_ADMIN_TOKEN`。

```text
GET    /admin/v1/source-runs                 # 未实现（M3+）
GET    /admin/v1/review-queue                # 未实现（M3+）
GET    /admin/v1/games/{app_id}/debug        # M2：app + multiplayer_profile
POST   /admin/v1/games/{app_id}/overrides    # M2：创建人工覆盖
POST   /admin/v1/overrides/{id}/revoke       # M2：撤销覆盖
GET    /admin/v1/algorithms                  # 未实现
POST   /admin/v1/algorithms/{version}/activate
POST   /admin/v1/golden-tests/run
```

所有写操作记录操作者、原因、前后值和请求 ID（`x-request-id` 可选）。算法激活前必须有黄金测试结果。

## 17. 限流与大小限制

建议初始值：

| 路由 | 限制 |
| --- | --- |
| 普通读取 | 每设备 120/min，叠加 IP 防滥用 |
| 普通搜索 | 每设备 30/min |
| AI 推荐 | 每设备 5/min、50/day，并受全局预算限制 |
| 反馈 | 每设备 60/min |
| 请求 JSON | 默认最大 64 KiB |
| AI 自然语言 query | 最大 2,000 Unicode 字符 |

具体值通过部署配置调整。429 响应返回 `Retry-After`。

## 18. 契约测试

- OpenAPI Schema 与 Rust DTO 快照一致。
- 每个错误码有示例和状态码测试。
- 旧客户端忽略新增字段的兼容测试。
- 游标篡改、过期和查询不匹配测试。
- 幂等键重复/冲突测试。
- AI `used/cached/disabled/fallback` 四种响应测试。
- 所有 AppID、价格、比例、人数和字符串长度边界测试。

