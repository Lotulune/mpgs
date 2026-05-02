# 当前 AI 评估算法标准说明

日期：2026-05-01

## 1. 结论摘要

当前项目里用户实际看到的“AI 评估分”，主路径上并不是由大模型直接自由打分，而是由一套规则分数先算出 `overall_score`，再由大模型可选地润色文案。

这意味着：

- 当前详细评估的数值本质上是规则分，不是 LLM 评分。
- 分数区间之所以容易集中在 `70` 到 `80`，是因为各维度存在较高基线、加分项偏离散、扣分项偏少、最终又采用等权平均。
- 你观察到大量结果徘徊在 `74` 左右，是当前公式形状直接推出来的，不是偶然现象。

## 2. 当前真实调用链

### 2.1 详细评估主链路

当前前端的 `assess_game_with_ai` 命令并不是直接调用 LLM 评分，而是：

1. 生成或读取详细分析报告。
2. 从报告里提取 `overall_score`、`overview`、`strengths`、`risks`。
3. 将报告摘要成 `AiAssessment` 返回给前端。

对应链路：

- `src-tauri/src/commands.rs`
  - `assess_game_with_ai()`
  - `generate_assessment_from_report_pipeline()`
  - `generate_or_load_game_analysis()`
- `src-tauri/src/game_analysis.rs`
  - `generate_game_analysis()`
  - `build_rule_report()`
  - `summarize_report_as_assessment()`

### 2.2 大模型在主链路中的职责

主链路里，LLM 只负责润色叙事内容，不负责修改数值分。

`src-tauri/src/game_analysis.rs` 中的逻辑是：

- 先调用 `build_rule_report()` 算出规则报告。
- 如果配置了 LLM Key，则调用 `llm::generate_analysis_narrative()`。
- `apply_narrative_patch()` 只会更新这些内容：
  - `overview`
  - `strengths`
  - `risks`
  - 各维度的 `reason`
- `overall_score` 和 `dimension_scores[].score` 不会被 LLM 改写。

因此，当前“AI 详细评估”的数值是规则层决定的，LLM 只改文案表达。

### 2.3 分数如何回写到游戏卡片

详细报告生成后，后端会做两件事：

1. 把 `report.overall_score` 写回 `game.ai_score`
2. 用新的 `ai_score` 重新计算 `recommendation_score`

对应代码在 `src-tauri/src/commands.rs`：

- `updated_game.ai_score = Some(report.overall_score);`
- `updated_game.recommendation_score = db::score_card(&updated_game);`

前端展示优先级在 `src/features/library/gameScoreDisplay.ts`：

- 如果 `aiScore` 存在，显示 `AI 评测`
- 否则显示 `推荐值`

所以用户在卡片上看到的“AI 分”，大多已经是详细报告里的规则分回写结果。

## 3. 当前详细评估的实际评分公式

### 3.1 总分结构

详细评估报告的总分来自 5 个维度的等权平均：

```text
overall_score = round1(
  (
    approachability
    + multiplayer_fun
    + content_depth
    + reputation_stability
    + activity_health
  ) / 5
)
```

其中 `round1` 表示保留 1 位小数。

对应实现：`src-tauri/src/game_analysis.rs -> build_rule_report()`

### 3.2 维度 1：易上手度 `approachability`

公式：

```text
score = 52
      + 10  if demo_status != Unknown
      + 12  if tags contain any of: casual / co-op / simulation / party
      + 12  if supported_languages contains "Chinese"
```

特征：

- 基线直接是 `52`
- 有 Demo 或已知试玩路径就上到 `62`
- 如果额外命中标签和中文支持，可以到 `74` 或 `86`

对应实现：`src-tauri/src/game_analysis.rs -> approachability_dimension()`

### 3.3 维度 2：联机乐趣 `multiplayer_fun`

公式：

```text
score = 68 if there is any multiplayer mode, else 20
score += 4  if generic multiplayer signal exists
score += 12 if co-op exists
score += 10 if pvp exists
score += 6  if online or lan exists
score += 8  if local multiplayer exists
score += 4  if cross-platform exists
```

特征：

- 只要是多人游戏，起步就是 `68`
- 典型 `Online Co-op + Co-op` 组合很容易直接到 `86`
- 多人属性一旦明确，这个维度天然偏高

对应实现：

- `src-tauri/src/game_analysis.rs -> multiplayer_fun_dimension()`
- `src-tauri/src/game_analysis.rs -> collect_multiplayer_mode_signals()`

### 3.4 维度 3：内容深度 `content_depth`

公式：

```text
score = 56
score += min(tag_count * 4, 12)
score -= 8 if any negative review contains:
         thin / variety / repeat / late-game / late game
```

特征：

- 基线是 `56`
- 只要有 3 个标签就能拿满 `+12`
- 负面扣分只有一个固定 `-8`
- 扣分依赖非常少数的英文关键词

对应实现：`src-tauri/src/game_analysis.rs -> content_depth_dimension()`

### 3.5 维度 4：口碑稳定性 `reputation_stability`

公式：

```text
score = positive_review_pct * 0.72
      + clamp(log10(total_reviews) * 8, 0, 24)
```

特征：

- 好评率是主导项
- 评测量通过对数加权，最多加 `24`
- 对于 85% 到 92% 好评率、上百条评测的游戏，这项通常已经在 `75+`

对应实现：`src-tauri/src/game_analysis.rs -> reputation_stability_dimension()`

### 3.6 维度 5：活跃健康度 `activity_health`

公式：

```text
score = 35                                if current_players <= 0
score = 35 + clamp(log10(players) * 18, 0, 50)   otherwise
```

特征：

- 即使没有玩家数据，基线仍有 `35`
- 只要在线人数达到三位数，这项就会快速升到 `70` 左右
- 同样采用对数函数，头部游戏之间差距会被压缩

对应实现：`src-tauri/src/game_analysis.rs -> activity_health_dimension()`

## 4. 当前推荐值的计算方式

除了详细评估分，项目里还有一个 `recommendation_score`，它用于排序和推荐。

公式在：

- `src-tauri/src/recommendation.rs`
- 前端镜像实现：`src/domain/recommendation.ts`

公式如下：

```text
recommendation_score =
  round1(
    review_quality
    + review_confidence
    + player_activity
    + multiplayer_fit
    + demo_bonus
    + freshness
    + ai_score_component
  )
```

各项定义：

- `review_quality = positive_review_pct / 100 * 36`
- `review_confidence = log_weight(total_reviews, 10000) * 8`
- `player_activity = log_weight(current_players, 10000) * 14`
- `multiplayer_fit` 最多 `14`
- `demo_bonus` 最多 `4`
- `freshness` 最多 `5`
- `ai_score_component = (ai_score or 72) / 100 * 20`

这里有一个很重要的设计：

```text
如果 ai_score 为空，系统默认按 72 分计入推荐值
```

这意味着即使还没有真实 AI 详细评估，推荐值也会先自带一段来自 `72` 的固定贡献：

```text
72 / 100 * 20 = 14.4
```

这会进一步把推荐分往中高区间推。

## 5. 为什么大量结果会徘徊在 74 左右

## 5.1 典型样本直接推导就接近 74

假设一款游戏具备这些常见特征：

- 有 Demo 或已知试玩状态
- `Online Co-op + Co-op`
- 3 个标签
- 好评率 `88%`
- 评测数 `200`
- 当前在线 `120`
- 没有触发负面评测关键词扣分
- 没有命中中文支持额外加分

按当前公式计算：

```text
易上手度        = 52 + 10 = 62
联机乐趣        = 68 + 12 + 6 = 86
内容深度        = 56 + 12 = 68
口碑稳定性      = 88 * 0.72 + log10(200) * 8
                ≈ 63.36 + 18.41
                ≈ 81.8
活跃健康度      = 35 + log10(120) * 18
                ≈ 35 + 37.4
                ≈ 72.4

overall_score   = (62 + 86 + 68 + 81.8 + 72.4) / 5
                ≈ 74.0
```

这就是你看到大量样本停在 `74` 左右的直接原因。

## 5.2 造成同质化的核心原因

### 原因 1：多个维度基线过高

当前 5 个维度里，起始分分别是：

- 易上手度：`52`
- 联机乐趣：`68`（只要有多人模式）
- 内容深度：`56`
- 活跃健康度：`35`

这意味着很多游戏还没体现出明显优势，就已经有一条很厚的底分。

### 原因 2：多人游戏被结构性抬高

这个产品本身筛的就是多人游戏，所以很多样本天然满足：

- 有多人模式
- 有合作模式
- 有在线联机

而联机乐趣维度对此给出的加分非常集中，导致大量游戏在这一项都落在 `80+`。

### 原因 3：总分是等权平均，弱项被稀释

即使一款游戏某一项明显偏弱，只要另外几项稳定在 `70+`，最终平均分仍然容易回到中高位。

等权平均的结果是：

- 不容易把“明显普通”的游戏拉到低分
- 也不容易把“真正强”的游戏拉开特别大差距

### 原因 4：负面扣分太弱、触发条件太窄

内容深度维度唯一明确扣分只有：

- `-8`
- 而且只在负评中命中这些英文词时触发：
  - `thin`
  - `variety`
  - `repeat`
  - `late-game`
  - `late game`

如果评测是中文，或者差评问题不使用这些词，扣分就不会发生。

### 原因 5：置信度不参与数值惩罚

当前 `confidence` 只是一个标签：

- `high`
- `medium`
- `low`

但它不会折损最终分数。

所以：

- 低证据样本
- 标签不完整样本
- 评论样本稀少样本

仍然可以拿到不低的分。

### 原因 6：对数函数会压缩头部差异

`total_reviews` 和 `current_players` 都用了对数函数。

优点是不会让超热门游戏碾压全部样本。
缺点是：

- 头部和次头部之间的差距会被压缩
- 中部样本更容易挤在一起

### 原因 7：前端最终展示为整数

详细报告和卡片展示都使用了 `Math.round(...)`。

这会把：

- `73.6`
- `74.1`
- `74.4`

都变成用户感知上非常接近的整数分数。

## 6. 当前实现里会进一步放大同质化的代码问题

下面这些不是抽象上的“模型局限”，而是当前实现中的具体问题。

### 6.1 中文支持加分判断和实际存储格式不一致

`approachability_dimension()` 用的是：

```text
supported_languages.any(|lang| lang.contains("Chinese"))
```

但 `src-tauri/src/steam.rs -> normalize_supported_language()` 会把中文归一化为：

- `schinese`
- `tchinese`

这意味着：

- 真实数据里即使支持简中
- `易上手度` 的中文支持 `+12` 也可能经常拿不到

结果就是该维度更容易固定在 `52` 或 `62` 附近。

### 6.2 标签加分只匹配英文关键词，但标签来源通常是本地化文案

`approachability_dimension()` 的标签判断只匹配：

- `casual`
- `co-op`
- `simulation`
- `party`

但 `src-tauri/src/steam.rs -> tags()` 直接读取 `genres/categories.description`。

在默认 `schinese` 配置下，这些标签很可能是：

- `合作`
- `休闲`
- `模拟`
- `派对`

于是标签加分也可能频繁失效。

### 6.3 差评关键词扣分同样偏英文

内容深度的负面扣分依赖英文关键词。

而评测抓取逻辑在中文环境下会优先拿 `schinese` 评论，因此：

- 中文差评对内容深度的打击可能远低于预期
- 导致分数更难真正拉开

## 7. 当前标准的边界

当前这套标准更适合做：

- 多人游戏候选的第一轮粗筛
- 口碑、活跃度、联机属性的结构化摘要
- UI 上稳定、可解释、可缓存的规则结果

不太适合做：

- 强区分度排行榜
- 需要明显拉开前 10% 和中位样本差距的场景
- 中文评论语义驱动的深度优劣判断
- 低样本新游和高样本成熟老游的公平同尺比较

## 8. 如果目标是提升区分度，建议优先改哪些点

以下是建议，不是当前已实现行为。

### 建议 1：降低维度基线

优先检查这些起始值是否过高：

- `52`
- `68`
- `56`
- `35`

如果基线下降，普通样本会更容易落回 `60` 到 `70` 区间，头部空间也会更大。

### 建议 2：把“缺证据”直接转成分数惩罚

当前置信度只展示，不入分。

建议把：

- 评论样本少
- 联机模式不清晰
- 在线人数缺失
- 负评证据为空

转化为明确扣分项，而不是只打 `low confidence` 标签。

### 建议 3：修复多语言归一化不一致

至少应统一这些判断：

- `schinese` / `tchinese`
- `合作` / `co-op`
- `休闲` / `casual`
- 中文差评关键词与英文差评关键词

否则很多规则看上去存在，实际并不会触发。

### 建议 4：减少二值型大台阶，加大连续型差异

现在很多规则是：

- 有就加一大块
- 没有就完全没有

建议把更多信号改为连续型，例如：

- 评论主题覆盖度
- 评测正负关键词密度
- 玩家活跃的分段曲线
- 标签丰富度和玩法跨度

### 建议 5：不要只做等权平均

可以考虑：

- 加权平均
- 惩罚项先扣后平均
- 几何平均
- “短板维度上限约束”机制

否则单个弱项会被其他中高分维度淹没。

### 建议 6：增加“分布校准”测试

当前仓库里有推荐值范围测试，但没有针对“分数分布是否过于集中”的测试。

建议新增一组固定样本，检查：

- 均值
- 标准差
- 分位数
- 头部和中位样本间距

否则以后继续调公式，也很难及时发现“全部又挤回 74”。

## 9. 与当前问题最相关的结论

如果只回答“为什么现在分数太同质化”，最关键的原因是这 4 条：

1. 详细 AI 分实际是规则平均分，不是模型开放式打分。
2. 多个维度存在较高基线，且多人游戏天然会吃到联机乐趣高分。
3. 扣分项很少、很弱，而且部分还依赖英文关键词。
4. 多语言归一化和评分规则之间存在错配，导致本该拉开差距的信号没有真正生效。

因此，`74` 不是异常值，反而是当前规则下的“自然吸附点”。

## 10. 证据索引

- `src-tauri/src/commands.rs`
  - AI 评估命令入口
  - 报告生成与回写 `ai_score`
- `src-tauri/src/game_analysis.rs`
  - 详细评估五维打分规则
  - 总分平均逻辑
  - 置信度、优势、风险生成规则
- `src-tauri/src/recommendation.rs`
  - 推荐值公式
  - `ai_score` 默认 `72`
- `src/features/library/gameScoreDisplay.ts`
  - 前端展示优先显示 `aiScore`
- `src-tauri/src/steam.rs`
  - `supported_languages` 归一化为 `schinese/tchinese`
  - 评论语言偏好
  - 标签来源
- `src/domain/recommendation.test.ts`
  - 当前仅覆盖推荐值大范围正确性，不覆盖分布离散度
