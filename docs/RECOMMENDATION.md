# 推荐算法规格

## 1. 目标

推荐器优化的是“这个游戏是否适合当前熟人小组”，不是 Steam 总热度。它必须同时处理：

- 新游戏样本少但具有 Demo 或明确联机卖点。
- 热门老游戏口碑普通但仍有健康玩家生态。
- 经典合作/自建服游戏 CCU 较低但不依赖公共玩家。
- Steam 标签无法准确表达开房难度、自建服质量和主导玩法。

MVP 使用可解释规则和人工标注，不直接训练黑盒模型。

## 2. 输入与输出

输入：

- 游戏规范化特征和每项特征的证据可信度。
- 评论、CCU、价格、发布日期和服务状态快照。
- 用户偏好：人数、合作/竞技、时长、预算、平台、自建服意愿、语言。
- 分区和算法配置版本。

输出：

```json
{
  "app_id": 548430,
  "section": "classic_legacy",
  "score": 0.91,
  "algorithm_version": "rules-0.1.0",
  "reasons": ["支持私人四人合作", "不依赖公共匹配"],
  "cautions": ["高难度任务需要稳定配合"],
  "components": {
    "friend_fit": 0.95,
    "quality": 0.94,
    "evidence": 0.92,
    "personal_fit": 0.88
  },
  "evidence_ids": ["feature:online_coop:548430", "review:548430:2026-07-13"]
}
```

## 3. 多人联机分类

一个游戏可以拥有多个能力，但必须标记主导体验：

| 维度 | 枚举示例 |
| --- | --- |
| 主导模式 | private_coop, private_pvp, self_hosted_survival, party, matchmaking_competitive, public_world, mmo |
| 连接方式 | private_lobby, p2p, player_hosted, dedicated_official, dedicated_self_hosted, public_matchmaking |
| 服务器依赖 | none, optional, official_required, public_population_required, unknown |
| 加入方式 | invite, join_code, server_browser, direct_ip, matchmaking, unknown |
| 进度形态 | session, run_based, persistent_world, live_service, unknown |
| 人数 | min_players, recommended_min, recommended_max, hard_max |

Steam 的“多人”“在线合作”等标签只能形成来源证据，不能单独确定主导模式或自建服质量。

## 4. 证据与未知值

每个可争议特征保存：

```text
value + confidence + source_type + source_ref + observed_at
```

来源优先级默认如下：

1. 人工核验且附证据。
2. Steam 官方明确字段或开发者明确说明。
3. 两个以上独立来源一致推断。
4. 单一商店标签或 AI 文本推断。

缺失值不等于 `false`。数值型特征按以下方式收缩到同类先验：

```text
effective(x) = confidence * x + (1 - confidence) * cohort_prior(x)
```

同时单独计算证据充分度，防止“未知很多”的游戏获得虚假高分。

## 5. 候选硬过滤

以下条件在评分前执行，AI 无权恢复被淘汰候选：

- 不是可游玩的基础游戏、Demo 或明确关联的 Playtest。
- 已下架且无可购买/可运行路径。
- 已知多人服务关闭，且没有局域网、P2P 或自建服替代。
- 不支持用户要求的平台。
- 推荐人数范围与用户硬性人数完全不相交。
- 用户明确排除的内容、模式或价格条件。
- 来源置信度过低，无法证明具备任何多人能力。

## 6. 核心特征

所有分项在进入公式前归一化为 `[0, 1]`。

### 6.1 熟人联机适配度 F

```text
F_base = 0.22 * private_session
       + 0.20 * self_host_or_dedicated
       + 0.18 * online_coop
       + 0.15 * group_size_fit
       + 0.10 * low_public_population_dependency
       + 0.08 * drop_in_out
       + 0.07 * cross_platform_fit

F_penalty = 0.18 * matchmaking_core
          + 0.15 * public_world_dependency
          + 0.10 * group_size_mismatch
          + 0.08 * service_shutdown_risk
          + 0.06 * external_account_friction
          + 0.05 * platform_or_anticheat_restriction

F = clamp(F_base - F_penalty, 0, 1)
```

`self_host_or_dedicated` 必须区分官方专服与玩家可部署专服。只有后者能显著降低停服风险。

### 6.2 评价质量 Q

对正面数 `p`、总评价数 `n`，使用 Wilson 下界而不是裸好评率。`z=1.96`：

```text
phat = p / n
W = (phat + z^2/(2n) - z*sqrt(phat*(1-phat)/n + z^2/(4n^2)))
    / (1 + z^2/n)
```

```text
Q = 0.65 * Wilson(lifetime) + 0.35 * Wilson(recent_90d)
```

- 没有近期样本时，近期项回退到带低置信度的同类先验。
- 新游样本过少不会被判为差评，但会得到较低 `E`。
- 保存 Steam 官方过滤后的统计与原始统计；异常差异进入风险特征。
- 简中评价可作为“中文用户适配”辅助特征，不替代全语言质量。

### 6.3 证据充分度 E

```text
review_volume = clamp(log(1 + total_reviews) / log(1 + 50000), 0, 1)
feature_coverage = weighted_known_features / weighted_required_features
source_quality = weighted_mean(source_confidence)

E = 0.45 * review_volume
  + 0.35 * feature_coverage
  + 0.20 * source_quality
```

即将发售游戏没有评论时，`review_volume` 不参与，权重重新归一化。

### 6.4 活跃度 P

单次 CCU 波动大，使用 7 日窗口并在可比 cohort 内计算百分位：

```text
P = 0.60 * percentile(log1p(median_ccu_7d))
  + 0.25 * percentile(log1p(peak_ccu_7d))
  + 0.15 * normalized_trend(median_7d / median_28d)
```

cohort 至少按联机依赖类型区分，避免把四人合作游戏与 MMO 直接比较。CCU 只统计连接 Steam 的玩家，因此它是活跃度信号，不是真实总玩家数。

### 6.5 增长势头 M

```text
M = 0.45 * review_velocity_7d_vs_28d
  + 0.35 * ccu_trend_7d_vs_28d
  + 0.20 * update_or_release_event_freshness
```

对发售首周、免费周末、大版本和促销事件做事件标记，避免把短期尖峰永久视为增长。

### 6.6 风险 R

风险是可叠加惩罚，主要包含：

- 多人服务器宣布关闭或持续不可达。
- 大规模异常评价波动。
- 发售日期反复变更或仍为模糊日期。
- 强制第三方账号、地区限制、反作弊导致的平台不兼容。
- 信息冲突或关键联机能力只有低置信 AI 推断。
- 价格/DLC 结构明显影响小组共同进入成本。

风险项必须生成可见提示，不能只暗中扣分。

## 7. 分区规则

### 7.1 最近发售

候选门槛：

- 正式发售 `0～180` 天。
- `F >= 0.45`，或人工确认为面向熟人联机的新作。
- 无多人服务关闭等硬风险。

```text
S_recent = 0.35F + 0.22Q + 0.15M + 0.10E
         + 0.10*freshness + 0.08*data_confidence - R
```

评论不足时允许出现，但必须显示“早期数据”并降低置信度。

### 7.2 即将发售/Demo

候选门槛：

- 未正式发售且有日期、Coming Soon、Demo 或公开 Playtest。
- 有至少一项多人能力证据。
- Demo 与本体关系已确认或达到可接受置信度。

```text
S_upcoming = 0.40F + 0.25*demo_playability
           + 0.12*release_date_confidence
           + 0.10*release_proximity
           + 0.08*studio_prior
           + 0.05*data_confidence - R
```

`studio_prior` 权重受限，防止大厂天然压制独立游戏。

### 7.3 人气老游

候选门槛初始值：

- 发售超过 180 天。
- `median_ccu_7d >= 1000`，或活跃度处于对应 cohort 前 `20%`。
- `F >= 0.45`。
- Wilson 质量下界通常不低于 `0.58`；活跃度前 `5%` 时可放宽到 `0.55`，不得完全取消。

```text
S_popular = 0.35F + 0.32P + 0.12Q + 0.10M
          + 0.11*data_confidence - R
```

这实现“人气可部分豁免好评度加权”，但不会把持续差评的游戏无条件推荐。

### 7.4 经典老游

候选门槛初始值：

- 发售超过 180 天。
- 总评价数不少于 `3000`。
- Wilson 质量下界不低于 `0.82`。
- `F >= 0.55`。
- 若依赖公共匹配，必须满足最低活跃度；自建服、私人房或 P2P 可豁免。

```text
S_classic = 0.40F + 0.30Q + 0.18E
          + 0.08*longevity + 0.04*maintenance_health - R
```

所有门槛是 `rules-0.1.0` 种子参数，必须通过真实数据分布校准，不能长期作为不可变业务常量。

M3 实现从活动 `algorithm_configs.config_json` 读取并校验这些门槛；配置内容和版本共同进入游标/ETag 上下文。配置变化后旧游标失效，避免跨规则版本续页。

## 8. 个性化

个人/小组适配度 `U`：

```text
U = 0.30 * party_size_fit
  + 0.20 * coop_competitive_fit
  + 0.15 * session_length_fit
  + 0.12 * budget_fit
  + 0.10 * platform_fit
  + 0.08 * hosting_willingness_fit
  + 0.05 * language_fit
```

```text
personalized_score = 0.75 * section_score + 0.25 * U
```

明确的用户硬条件在候选过滤阶段执行；`U` 只处理软偏好。

M3 对已知的平台、语言、典型局时长和同币种价格执行硬过滤；字段未知时保持候选资格并以中性值参与个性化，遵守“缺失不等于 false”。

反馈更新：

- “不感兴趣”抑制该 AppID，默认不永久影响整个类型。
- “太竞技”“人数不合适”“开服麻烦”更新对应偏好维度。
- “玩过”默认从发现流降权，但可保留在经典对比和小组库场景。
- 负反馈保留原因枚举，文本说明可选且默认不发送 AI。

## 9. 多样性与探索

基础排序后使用 MMR：

```text
MMR(candidate) = 0.85 * relevance
               - 0.15 * max_similarity(candidate, selected)
```

约束：

- 同一系列在首屏最多 2 个。
- 同一发行商和高度相似玩法设置软上限。
- 默认保留约 `20%` 探索位，从高不确定但满足硬条件的候选中选取。
- 新游探索位必须显示低置信标识，不允许隐藏不确定性。

## 10. AI 二次排序

AI 仅接收确定性 Top 20 与证据摘要。有效 AI 分数为：

```text
ai_effective = ai_confidence * ai_fit
             + (1 - ai_confidence) * personalized_score

final_score = 0.85 * personalized_score + 0.15 * ai_effective
```

规则：

- AI 权重最大 `0.15`，不能改变分区和硬过滤结果。
- AI 返回 AppID 必须属于本次候选集。
- 每条理由必须引用已提供 `evidence_id`。
- 无效、超时或低置信输出回退到 `personalized_score`。
- AI 可以指出新的待审核特征，但不能立即把它提升为高置信事实。

## 11. 推荐解释

解释由确定性模板优先生成，AI 可做语言润色但不能改变事实。

每个条目至少包含：

- 两个最强正向原因。
- 最重要的一个风险或限制；没有已知风险时显示“暂无已确认限制”，不能声称没有限制。
- 适合人数、联机方式和公共玩家依赖。
- 数据快照时间与主要证据来源。
- “为什么出现在该分区”的简短说明。

## 12. 计算策略

```text
定时任务：生成全局分区候选和基础分快照
请求时：读取快照 -> 叠加用户偏好 -> MMR -> 返回
自然语言请求：混合检索 -> 基础排序 -> AI Top 20 -> 返回
```

不在每个普通推荐请求中调用 AI。相同输入按以下缓存键复用：

```text
algorithm_version + feature_snapshot + preference_hash
+ normalized_query_hash + ai_model + prompt_version
```

## 13. 测试与评估

### 13.1 黄金测试集

至少 50 个游戏，覆盖：

- 私人合作：深岩银河、雨中冒险 2 等。
- 自建服生存：方舟、帕鲁等。
- 匹配核心竞技：CS2、永劫无间等。
- MMO/公共世界、派对游戏、停服游戏、错误人数和 Demo 关系。

核心断言：

- 默认偏好下，私人合作/自建服组整体高于公共匹配核心组。
- 自建服经典游戏不会仅因低 CCU 被淘汰。
- 人气老游可以降低质量权重，但差评底线仍生效。
- 改变人数、竞技偏好和自建服意愿后，排序方向符合预期。
- AI 关闭、超时和输出攻击场景与确定性结果可用性一致。

### 13.2 离线指标

- `Precision@20`：首屏熟人联机合格率。
- `NDCG@20`：人工相关性排序质量。
- 分区准确率和跨分区重复率。
- 类型、系列和发行商覆盖率。
- 低置信候选曝光占比。
- 解释证据覆盖率。

### 13.3 在线指标

- 详情打开、Steam/Demo 跳转和明确负反馈。
- “人数不合适”“太竞技”“开服麻烦”的原因率。
- AI 回退率、无效输出拦截率和单次成本。
- 不优化单纯停留时长，避免制造无效浏览。

## 14. 版本管理

每次推荐保存：

```text
algorithm_version
config_version
feature_snapshot_at
candidate_set_hash
ai_model (optional)
prompt_version (optional)
```

修改权重、阈值、缺失值策略或分区规则都必须生成新 `config_version`。发布前必须对旧版本和新版本运行同一黄金集并保存差异报告。
