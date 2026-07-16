# M5 验收说明

M5（AI 与语义检索）按 `MVP_PLAN.md` 退出条件验收。AI 为增强层：关闭、超时、无 Key 时不得破坏普通推荐与自然语言确定性路径。

## 退出条件对照

| 条件 | 自动化证据 |
| --- | --- |
| AI 关闭/无 Key 不影响普通推荐 | `scripts/m5_acceptance.ps1`：`meta.ai_available=false`、feed 非空、NL `ai_status=fallback` |
| 用户可见具体 AI 事实需有证据 | 排序输出校验拒绝无 evidence 的 reasons；离线特征物化带 `evidence_refs` |
| 候选外 AppID / 伪造 evidence / 非法分数不可穿过 | `mpgs-ai` validate 单测 + Fake AI 集成路径校验 |
| 在线 AI 超时/失败回退 | Gateway 超时/预算/熔断 + NL enhance 失败返回 fallback |

## 本机门禁

```powershell
# 离线全量（不需要外部 Key）
.\scripts\m5_acceptance.ps1

# 可选：声明已配置 Key 的联调提醒
$env:MPGS_AI_API_KEY = '...'
.\scripts\m5_acceptance.ps1 -LiveAi
```

脚本会写入/覆盖 [`M5_ACCEPTANCE_RUN.md`](M5_ACCEPTANCE_RUN.md)。

## 工具链

```powershell
# 文档/FTS（可顺带写 hash embedding）
cargo run -p mpgs-dbtool -- sync-retrieval <db> [limit] [after_app_id]

# 离线特征 → ai_analyses
cargo run -p mpgs-dbtool -- extract-offline-features <db> [limit] [after_app_id]

# 按 MPGS_AI_EMBED_PROVIDER 批量 embedding 回写
$env:MPGS_AI_EMBED_PROVIDER = 'hash'   # or openai_compat
cargo run -p mpgs-dbtool -- embed-documents <db> [limit] [batch]
```

## 配置

| 变量 | 含义 |
| --- | --- |
| `MPGS_AI_PROVIDER` | `disabled` / `openai_compat` |
| `MPGS_AI_API_KEY` | openai_compat 必需 |
| `MPGS_AI_BASE_URL` | 默认 `https://api.openai.com/v1` |
| `MPGS_AI_MODEL` | 默认 `gpt-4o-mini` |
| `MPGS_AI_EMBED_PROVIDER` | `hash` / `openai_compat` / `disabled` |
| `MPGS_AI_EMBED_MODEL` | openai embedding 模型 |
| `MPGS_AI_EMBED_DIMENSIONS` | hash 默认 64；openai 默认 1536 |
| `MPGS_AI_TIMEOUT_SECS` | 请求超时 |

## 状态

代码侧交付与离线退出条件由 `m5_acceptance.ps1` 门禁。生产 Key 下的真实 Provider 联调为可选增强证据，不阻塞“无 Key 安全可用”的 M5 关闭标准。
