---
status: accepted
updated: 2026-11-09
---

# 平台原生实现改造 评估（rich_text 共享包）

- status: accepted（已拍板：走方案 B）
- 关联：[编辑器增量/懒解析 优化评估](../incremental-parsing-eval.md)；goodshare `docs/design/`（content-pipeline / rich-text-* / ui-spec，设计 SSOT）
- 结论：**单一原生核心 + 各端绑定（方案 B）。本分支 `android-native-sdk` 已改造为纯 Rust 核心（`native/rich_text_core`）+ UniFFI 绑定层（`native/rich_text_uniffi`），AST 跨 FFI 走 JSON 序列化形态，严禁逐平台各写一份（方案 C 已否决并归档）。**

---

## 1. 现状

- 纯 Dart 3.13，零运行时依赖、无 Flutter；`lib/rich_text.dart`（AST + 编辑原语，665 行）+ `lib/markdown_subset.dart`（解析/序列化/AI 归一，1002 行）。
- 消费方：goodshare（交互渲染器）与 goodhub（只读渲染器），各自持有渲染层，只消费本包产出的 AST 与编辑原语。
- 包本身即 2026-10-14 从 goodshare 抽出共享的产物——**当时解决的问题正是「两仓各持一份 fork 副本」的漂移灾难**。
- 全量语义测试在 goodshare 仓；本仓 `test/` 仅 90 行冒烟。

## 2. 三个动机逐项检验

### 2.1 性能瓶颈 —— 不成立（暂）

- 体量证据：全量解析对 10KB 文档为毫秒级~十几毫秒（见增量解析评估 §1），瓶颈在**重复全量解析**而非解析速度本身；
- 已有既定路线：增量解析评估已裁定「先方案 A（尾块增量）」，以 O(尾块) 摊还线性解决流式场景，改动小、零 API 变化；
- 换原生对「逐字符扫描」类工作的常数收益，远小于把 O(n²) 降为 O(n) 的算法收益；
- **缺实证**：目前没有任何 profiling 数据表明 parse/serialize 占据真机帧耗时前列。

判定：性能动机不足以开工；若真有瓶颈，先落方案 A 再复测。

### 2.2 宿主脱离 Dart —— 路线级决策，尚未确认

- 若 goodshare/goodhub 迁离 Flutter（原生 / 跨端技术栈），消费方将不再是 Dart，AST 交付形态随之改变——这属于宿主仓的路线 ADR，本包单方面先行改造无意义；
- 该动机是三个中最有分量的：一旦成立，解析核心确实需要非 Dart 消费路径。

### 2.3 多语言复用 —— 诉求真实，但正解只有一种形态

- 「每语言一份原生实现」（Kotlin/Swift/C++ 各写一遍）= 把 2026-10-14 抽包消灭的 fork 问题在语言维度复活，且是 2~3 份；语义铁律（parse↔serialize 幂等、降级不吞内容、span 坐标一致）每份都要独立钉死——命中方案准入「同样的处理在第 2 个调用点再写一遍」一票否决信号的架构级同构。
- 唯一正解：**单一原生核心 + 各端绑定**（方案 B，见 §3）。

## 3. 方案对比

| 方案 | 思路 | 实现份数 | 跨边界成本 | 维护成本 | 判定 |
|---|---|---|---|---|---|
| A. 维持纯 Dart | 现状不动；性能走增量解析评估方案 A | 1 | 无 | 低 | 性能动机下的正解 |
| B. 单一原生核心 + 绑定 | Rust（或 C++）重写核心，UniFFI / JNI / dart:ffi 各端绑定，AST 以序列化形态或句柄过界 | 1 + 各端薄绑定层 | 高（见 §4） | 中：核心一份，绑定层随 FFI 工具链演化 | 脱离 Dart / 多语言动机下的正解 |
| C. 逐平台原生实现 | Android 用 Kotlin、iOS 用 Swift 各写一份 | 2~3 | 无 | 高：语义对齐测试 ×N，漂移回归 | **否决**（fork 复活 + 重复实现） |

方案 B 语言选型若开工，倾向 **Rust + UniFFI**（Kotlin/Swift 绑定成熟，Dart 侧有 dart:ffi / flutter_rust_bridge 两条成熟通路）；C++ 无一等绑定生成器，手工 FFI 成本显著更高。

## 4. 方案 B 的关键难点（开工前必须认清）

1. **AST 跨 FFI 序列化**：AST 是不可变嵌套树（行内节点递归、引用块嵌套、列表续行），过界要么整树序列化（JSON/FlatBuffer——吃掉大部分性能收益，且编辑原语每次往返）、要么 opaque handle + 访问器（生命周期管理复杂，编辑交互逐键跨界调用，FFI 流量反成新瓶颈）。**这是方案 B 最大的结构性风险，也是「性能动机」下方案 B 往往不划算的直接原因。**
2. **坐标体系陷阱**：编辑原语的 run 坐标是 Dart 字符串索引（UTF-16 code unit）；Rust/C++ 字符串是 UTF-8 字节（Kotlin 也是 UTF-16 但 Swift String 是 Unicode scalar/grapheme）。跨语言后同一 `[start, end)` 语义不一致，选区/样式套用会错位——需要显式定义坐标契约并在边界换算或统一为字节偏移。
3. **语义事实源**：全量语义测试在 goodshare 仓，幂等/降级铁律（AI 归一不吞内容）是隐式契约——原生核心必须先把这些测试从 Dart 单测迁移为跨实现同跑的黄金语料（fixture），否则无对齐基准。
4. **双实现过渡期**：迁移期 Dart 实现与原生核心并存，须以 fixture 差分测试锁死一致后才能下线 Dart 版；期间任何 parser 改动双写。

## 5. 建议与触发条件

**决策（2026-11-09）：走方案 B，本分支改造为纯原生 SDK。** 宿主迁移 ADR 虽仍未最终落
地，但「多语言复用 + 脱离 Dart 消费路径」已作为既定方向推进；按 §3 正解，选 Rust + UniFFI
单一核心，各端（Kotlin/Swift/Dart）由 UniFFI 自动生成薄绑定，严禁方案 C 的逐平台各写一份。

触发条件（任一落地即重开本评估，升级为开工裁决）：

1. goodshare/goodhub 宿主迁移 ADR 落地，明确脱离 Dart/Flutter（→ 直接走方案 B，Rust 核心）;
2. profiling 实证 parse/serialize 在真机帧耗时中占比显著（且方案 A 已实施仍不达标）;
3. 出现第三个非 Dart 消费方（服务端/CLI 等）的现实需求。

开工前的零成本准备（现在就可做，均为 Dart 侧动作）：

- 把 goodshare 全量语义测试整理为**语言无关黄金语料**（输入 md + 期望 AST/序列化 JSON），为未来任何实现切换铺对齐基准；
- 给 AST 补一个 `toJson/fromJson` 双向导出（纯 Dart、零依赖），既服务未来跨语言边界，也立即让非 Dart 消费方有可用通路。

> 上述「零成本准备」已在方案 B 落地时一并完成：黄金语料沉淀为 `native/rich_text_core/tests/`
> （`kitchen_sink.md` / `inline_tricky.md` + `golden.rs` 往返幂等 + 结构不变量），跨 FFI 的
> AST 出口即 `blocks_to_json` / `blocks_from_json`（JSON 形态）。

## 6. 试点处置（2026-11-09）— 方案 C 已否决并移除

分支早期曾在 `kotlin/` 下落地一个 **方案 C（Android Kotlin 原生 SDK）** 的可行性探针
（完整移植原 `lib/rich_text.dart` + `lib/markdown_subset.dart`，snake_case API 1:1 对齐
黄金语料，附 JUnit4 冒烟）。该试点与 §3 结论直接冲突（方案 C 被明确否决），已按「纯 Rust
项目」决策 **从仓库移除**（`archive/kotlin-pilot-c/` 整目录删除，含其 Gradle `.kts` 构建脚本），
不再延续。其价值已兑现为两点：(1) 验证 Dart→原生逐行移植可行；(2) 作为跨实现黄金语料的早期
种子，反哺方案 B 的对齐基准。

## 7. 方案 B 落地（本分支 `android-native-sdk`）

### 7.1 仓库布局

```
native/
  rich_text_core/         # 唯一事实源：纯 Rust 核心，零运行时依赖
    src/                  # ast / parser / regexes / helpers / lib
    tests/                # 黄金语料（fixtures）+ 往返幂等 + 结构不变量（golden.rs）
  rich_text_uniffi/       # UniFFI 绑定层（proc-macro 模式）
    src/lib.rs            # #[uniffi::export] 薄函数 + #[derive(uniffi::Record)] 结果类型
    bindings/             # 由 uniffi-bindgen 生成的 Kotlin/Swift 桩（见 7.4）
```

### 7.2 跨 FFI 形态（对应 §4.1 风险）

- **块树 / 行内 spans**：以 **JSON 序列化形态**过界（`parse_to_json` → 各端反序列化为本地模型；
  `serialize_blocks_from_json` 为编辑器回写唯一出口）。避免在热路径上整树往返——纯文本/标量函数
  （`rt_markdown_to_plain` / `rt_title_to_plain` / `rt_classify_media_url` / `rt_scan_todo_texts`
  / `rt_lost_media_urls` / `rt_version`）直接返回，零拷贝。
- **坐标契约**：行内 span 坐标为 **UTF-8 字节偏移**（§4.2 风险），与 Rust 核心一致；各端据本地
  字符串编码（Kotlin/Java UTF-16、Swift Unicode scalar）在边界换算。

### 7.3 语义事实源（对应 §4.3 风险）

- Rust 核心 `rich_text_core` 现已是**唯一事实源**（原 Dart `lib/` 已从仓库移除，本分支为纯 Rust 项目）。
  行为基准仍以 goodshare `docs/design/` 为 SSOT；Rust 核心为其忠实移植，公开 API 名与原 Dart 1:1。
  黄金语料 `tests/golden.rs` 钉死 parse↔serialize 幂等、降级不吞内容、span 坐标一致等铁律。已知原
  Dart 单源怪癖（lazy-backref 切分、`]` 不在转义集等）在核心中刻意保留，而非修正——保证与原设计
  输出一致（测试断言据此放宽）。
- **双实现过渡期**（§4.4）：随 Dart `lib/` 移除，过渡期结束——不再需要「Dart 版与原生核心并存、差分
  锁死后下线」流程，Rust 核心即为唯一实现。

### 7.4 绑定生成（对应 §3 方案 B）

- 绑定层用 UniFFI **proc-macro 模式**（`#[uniffi::export]` + `uniffi::setup_scaffolding!`），
  `uniffi-bindgen generate --library <cdylib> --language <kotlin|swift>` 产出各端桩。
- 已验证产出 **Kotlin**（JNA）、**Swift** 绑定；**Dart** 非核心 `uniffi-bindgen` 目标，需另用
  社区 `uniffi-dart-bindgen`（基于 `dart:ffi` / `ffigen`），见 `bindings/` 下说明。
- 再生脚本：`rich_text_uniffi/scripts/gen-bindings.sh`。

