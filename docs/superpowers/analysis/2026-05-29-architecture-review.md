# Fudoki 项目系统架构深度评审与设计对齐审计报告

本报告针对 Fudoki 项目（基于 **Rust Axum 后端 + SQLite 3 持久化 + React 19 / TypeScript / Vite 前端**）进行全面的系统架构审计。通过比对设计规约（Design Specs）与 feat/react-frontend-rewrite 分支的最新实现，对系统架构的一致性、边界完整性、状态管理合理性、API 安全性以及持久化并发保障进行多维度评估。

---

## 1. 架构概览与核心模式

Fudoki 采用了典型的“轻量级前端 + 高性能本地后端”微型桌面服务架构：
- **前端控制层与表示层**：React 19 辅以 `Vite` 构建。利用 **Zustand** 进行轻量交互状态的分发，利用 **TanStack Query**（React Query）管理服务端异步数据（包括文档列表、字典数据、TTS 提供商等）的生存周期与同步。
- **后端服务与数据层**：基于 Rust **Axum** 框架提供 RESTful Web API。核心分词与语音分析依赖 **SudachiTokenizer** 以及自定义的四级英文分词处理器；数据持久化与底层双缓存（分析缓存与 TTS 音频缓存）系统均基于 **SQLite 3** 构建。

---

## 2. 规约对齐性审查：设计承诺 vs 当前实现

经审计，新分支已成功弥补了先前版本（2026-05-15）的大部分历史遗留差距，但在部分边缘场景和异步同步细节上暴露出两个重大的架构/体验漏洞。

### 2.1 成功对齐的设计要点 ✅
- **编辑器 CodeMirror 6 集成**：成功废弃了 raw `<textarea>`，采用 `@uiw/react-codemirror` 进行了重构，并实现了 `EditorView.lineWrapping` 行包裹，提供了理想的选区意识与扩展基础。
- **分析调用的结构特征签名门控机制**：在 `App.tsx` 中实现了 `computeStructureSignature(text)` 函数，通过输出结构特征签名 `${lineCount}:${sentenceBreaks}` 判定文本骨架。只有在用户输入破坏或重构结构签名时（即非纯词汇编辑场景），才在 1200ms 防抖后触发 `/api/analyze` 耗时请求，成功避免了全量重绘和不必要的 API 洪峰。
- **TTS 缓存 Scope 失效机制**：前端成功传递了基于配置计算出的 `cache_scope_version`（由 `provider:model:voice:format` 组合而成），后端在 `tts_cache_repo.rs` 中通过比对此版本并在变更时自动触发 `clear_all()`，完成了持久化 TTS 缓存文件的自动清理与失效。
- **四级英文 Token 处理器**：在 `src/english.rs` 中按照规约精确实现了缩写音译（Tier 1）、内置覆盖项（Tier 2，如 React、Docker）、外来词音译字典匹配（Tier 3，约180个常见借词）以及未知英文透传（Tier 4），并辅以相应的置信度权重判定。
- **文档复制与分析缓存容量限制**：成功增加了 `POST /api/documents/:id/duplicate` 复制路由，并在分析缓存中设计了 30 天 TTL 与 512 阈值 LRU 淘汰机制，保证了本地数据库尺寸收敛。

### 2.2 🔴 【重大架构缺陷】保存冲突时的静默数据丢失（Silent Data Loss）
* **规约承诺**：设计中要求实施基于乐观并发锁（Revision-based Concurrency Control）的冲突防护，当版本不一致时，向用户抛出冲突保存失败，保护数据不被覆盖。
* **实现现状**：在 `frontend/src/App.tsx#L325-337` 中，前端在捕获到版本冲突错误（HTTP 409）时，其 Catch 块采取的策略为：
  ```typescript
  if (message.includes('409') || message.includes('document_revision_conflict')) {
    const latest = await documentsQuery.refetch()
    const freshDocument = latest.data?.documents.find((document) => document.id === activeDocument.id)
    if (freshDocument) {
      setDraft(freshDocument.content)       // 🔴 漏洞所在：直接覆盖本地草稿
      setDraftRevision(freshDocument.revision)
    }
    setSaveState('conflict')
    return
  }
  ```
* **潜在风险**：**这是一个极其严重的数据安全故障。** 当发生修改冲突时，系统会立即拉取服务端内容并**强行静默覆盖**用户在当前编辑器中正在输入并修改的内容，这会导致用户本地未经保存的工作成果瞬间且不可逆地丢失，且未提供任何“保留本地”或“手动比对差异”的冲突解决机制。

### 2.3 🔴 【重构遗留 Bug】文档切换时的缓存穿透且无兜底分析缺陷
* **设计意图**：用户在切换文档时，系统自动拉取 SQLite 中缓存的分析结果，如果有则渲染，无则重新触发后台自动分析，以展现分词与拼音。
* **实现现状**：在切换文档时，`syncedDocumentIdRef.current` 变更，触发同步 Effect：
  1. 设置 `suppressNextAutoAnalyzeRef.current = true`（临时阻止了自动分析 Effect 的定时器触发）。
  2. 异步调用 `loadCachedAnalysis(documentId, revision, content)` 尝试从后端 `/analysis` 获取缓存。
  3. **Bug 机制**：如果此时该文档的缓存未命中（例如首次冷启动、缓存到期被 LRU 淘汰、或之前分析因报错未生成），`loadCachedAnalysis` 将在 `!cached` 判断中重置分析状态并提前 return：
     ```typescript
     if (!cached) {
       setAnalysis(null)
       setAnalysisState('idle') // 🔴 置为 idle 并返回
       return
     }
     ```
     此时由于 `suppressNextAutoAnalyzeRef` 已在渲染时被单次消耗，**没有任何后台的兜底重新分词机制会被唤起**。导致此文档将维持在无任何分词标记（Idle）的状态，必须用户重新打字或点击手动的“Analyze”按钮。

---

## 3. 依赖方向与模块边界审查

### 3.1 后端依赖倒置（DIP）的偏离 🟡
* **合理方面**：后端模块整体架构较为规范：
  `main.rs / app.rs` (接入路由层) ➡️ `analyzer.rs / dictionary.rs` (核心逻辑层) ➡️ `storage/` (Rusqlite 持久化实现层)。
  像 `english.rs` 与 `number_reading.rs` 等辅助库纯粹解耦，没有任何数据库依赖，极其便于做本地单元测试。
* **缺陷表现**：核心 TTS 领域服务 `src/tts.rs` 直接导入并强耦合了持久化模块的 `crate::storage::db::AppDatabase` 和 `crate::storage::tts_cache_repo::TtsCacheRepository`。
* **重构建议**：此设计背离了**控制反转/依赖倒置（Dependency Inversion Principle）**。应当在逻辑层 `src/tts.rs` 中定义抽象的 Cache 行为 Trait：
  ```rust
  pub trait TtsCache {
      fn get(&self, key: &TtsCacheKey) -> Result<Option<SynthesizedSpeech>>;
      fn insert(&self, key: &TtsCacheKey, speech: &SynthesizedSpeech) -> Result<()>;
  }
  ```
  在后端应用组装时，由 `storage/` 下的具体实现库装配进 TTS 逻辑中。这样不仅提高了领域模型的内聚性，还极大地便利了在测试阶段对 TTS 的 API 请求进行 Mock。

### 3.2 前端 App.tsx 单体协调器（Monolithic Coordinator）的膨胀 🟡
* **现状分析**：前端使用 `features/` 进行组件物理分块非常规范，大部分 feature 组件为表现型受控组件（Controlled Presentational Components），测试隔离性好。但 `frontend/src/App.tsx` 作为整个主页的胶水层，已经膨胀至 1000+ 行代码，强行杂糅了：
  - 5 个以上的 React useState 和 6 个以上的 React useRef。
  - 4 个以上复杂的 TanStack Query (`useQuery` / `useMutation`)。
  - 本地音频播放控制的复杂 session lifecycle 维护。
  - 大量的并发保存控制和 i18n 桥接。
* **重构建议**：这严重违反了**单一职责原则（SRP）**。随着 Markdown 升级和多文档编辑的引入，App 组件将成为灾难。建议将网络同步、编辑器草稿管理以及音频播放控制（Playback Session）分别抽离至独立的 React Hook（如 `useDocumentSync`，`usePlaybackSession`），让 `App.tsx` 降噪为纯粹的布局组合器。

---

## 4. 状态管理模式评审

### 4.1 双状态库方案的成功之处 ✅
- **Zustand** 专注于纯前端轻量级响应式交互状态（例如搜索框文字、当前选中的分词 Token、活动的检查器面板 Tab 等），其更新在内存中同步完成，无冗余模板代码，渲染效率极高。
- **TanStack Query** 精准捕获网络异步状态（Documents 与 Settings），借助其 Stale-While-Revalidate 特性管理客户端的缓存有效期，自动在后台静默刷新健康指标或设置项。

### 4.2 【反模式】基于副作用（Effects）的本地状态双向强制同步 🟡
* **设计问题**：在 `App.tsx` 中，`draft`（编辑器当前缓冲区）和 `draftRevision` 使用了本地的 React State 管理。当 TanStack Query 的文档源 `activeDocument` 因网络拉取、列表变更而发生刷新时，代码通过一个庞大的 `useEffect` 副作用去硬性修改本地 draft 状态，完成数据流“降级”同步。
* **风险评估**：这是 React 开发中的典型**同步反模式**。由于 Effect 运行在渲染之后，会导致发生多轮不必要的双重渲染（Double Render），并在网络抖动、快速切换文档、或异步自动保存冲突时产生多条竞争数据流，导致本地 `draft` 的一致性难以被精确控制。
* **重构建议**：应将编辑器的 Draft 编辑草稿直接移入单独的 Zustand 切片（Slice）中，通过 Zustand 驱动单向状态流更新，由编辑器自下而上触发更新，摆脱在渲染树顶层用复杂的 `useEffect` 副作用去捕获网络状态变更的沉重设计。

---

## 5. API 契约一致性审查

### 5.1 数据契约一致性评估 ✅
Rusqlite 存储层通过 Serde 将 Rust 的下划线（Snake Case）映射与前端 TypeScript 定义的驼峰（Camel Case）实现了良好的双向兼容。所有的核心数据格式（如 `DocumentRecord` / `AnalyzeResponse`）都在底层具有严密的一对一映射，在正常网络调用中未发生类型偏离或未匹配字段导致的运行时错误。

### 5.2 🟡 【设计缺陷】类型不安全的 API 异常判定逻辑
* **问题所在**：后端虽然定义了非常严整的 JSON 异常信封（Envelope）：
  ```rust
  pub struct ApiErrorResponse {
      pub error: ApiErrorBody,
  }
  ```
  但在前端 `api.ts` 的底层封装 `requestJson` 拦截中，对于错误状态码一律提取了 `response.text()` 并直接封装为通用 String 形式的异常投掷出来：
  ```typescript
  if (!response.ok) {
    const text = await response.text()
    throw new Error(text || `Request failed: ${response.status}`)
  }
  ```
  导致上层的 React 组件无法获取类型安全的错误代码（Error Code），而必须依靠极端脆弱的“**字符串模糊子串匹配**”来实现保存冲突的流程分支：
  ```typescript
  if (message.includes('409') || message.includes('document_revision_conflict')) { ... }
  ```
* **安全隐患**：如果后端有任何微小的文案微调，或因为网络代理返回了非标准的带有 409 字样的 HTML 页面，前端的字符串模糊匹配将会立刻误判，具有高度的不确定性。
* **重构建议**：前端 `requestJson` 应在 `!response.ok` 时，尝试将响应体的 JSON 解析为 `ApiErrorResponse` 实体，若成功则向下游抛出包含明确 `code` 和 `message` 字段的自定义子类 `ApiError`，由前端业务逻辑直接通过 `error.code === 'document_revision_conflict'` 进行类型安全的判定。

---

## 6. 持久化层设计与并发机制审查

### 6.1 杰出的 SQLite 多线程并发配置（Pragmas） ✅
在 `/src/storage/db.rs` 初始化逻辑中，系统对所有的 Rusqlite 数据库连接执行了极为优异的本地高吞吐配置：
- `journal_mode = WAL`（开启预写日志模式）：使 SQLite 实现了真正的读写并发（读操作不会阻塞写操作，反之亦然），避免了本地读分析缓存时导致用户无法保存文档的致命体验。
- `foreign_keys = ON`：强制外键物理完整性。
- `busy_timeout(5s)`：在写入锁竞争时提供了长达 5 秒的自动超时重试缓冲，大幅减少了高并发写入时的 `Database locked` 报错。

### 6.2 🔴 【核心并发漏洞】文档保存乐观锁更新的非原子性（Non-atomic Update）
* **漏洞还原**：在文档存储库 `/src/storage/documents_repo.rs#L83-112` 中，更新文档版本控制的逻辑如下：
  ```rust
  pub fn update(&self, id: &str, request: UpdateDocumentRequest) -> Result<DocumentPayload, UpdateDocumentError> {
      // 1. 获取现有文档并检测版本（基于一次连接）
      let existing = self.get(id)?.ok_or(UpdateDocumentError::NotFound)?;
      if existing.revision != request.expected_revision {
          return Err(UpdateDocumentError::Conflict);
      }
      
      ...
      
      // 2. 执行真正的 UPDATE（基于另一次新的连接）
      self.db.with_connection(|conn| {
          conn.execute(
              "UPDATE documents SET title = ... WHERE id = ?1", 
              ...
          )
      })?;
  }
  ```
* **隐患剖析**：**这使乐观锁并发机制在并发调用下完全失效。**
  因为 `self.get()` 和 `conn.execute()` 分别调用了两次 `self.db.with_connection`，而 `with_connection` 每次都会自动从操作系统申请并开启一个**全新且独立**的 SQLite 连接，完成操作后立即关闭。
  如果发生并发请求（比如前端因为防抖或高频保存，或者在多设备同步时，有两个线程同时调用 `update`）：
  - 线程 A 读出当前 Revision = 3，判定无冲突。
  - 线程 B（并发读入）也读出 Revision = 3，判定无冲突。
  - 线程 A 连接抢占，写入成功，Revision 提升为 4。
  - 线程 B 连接抢占，也写入成功，Revision 也强制变更为 4。
  由于读与写之间缺失了悲观锁和原子事务的保护，在并发读写下会出现严重的“**丢失更新（Lost Update）**”数据安全漏洞。
* **重构建议**：整个比对与写入操作必须高度内聚在一个 `rusqlite` 的**原子事务**中执行。我们必须修改 `update` 函数，在同一个 `Connection` 下开启 `transaction()` 并完成 `SELECT` 校验与 `UPDATE` 写入，最后一次性 `commit()` 提交，确保原子性：
  ```rust
  self.db.with_connection(|conn| {
      let tx = conn.transaction()?; // 开启事务
      // 在 tx 事务连接中完成 select ... FOR UPDATE
      // 完成版本校验与更新
      tx.commit()?; // 提交事务
  })
  ```

---

## 7. 架构改进建议与演进路线图

为了让 Fudoki 架构在走向 Markdown 预览与生产级别的高可靠性时更加健壮，我们建议遵循以下三步重构路线图：

### 📈 第一阶段：核心并发与数据安全性修复（P0 - 紧急）
1. **重构乐观锁更新的事务性**：重写 `documents_repo.rs` 的 `update` 方法，保证读校验和写更新完全内聚在一个数据库事务中（原子化）。
2. **安全化冲突解决（消除丢数据漏洞）**：当前端捕获 409 Concurrency Conflict 时，禁止无脑 `setDraft` 覆盖本地草稿，应展示一个侧边滑出抽屉或提示框，标明“服务器版本与您的本地编辑版本存在分歧”，让用户在“保留我的更改（强制覆盖版本并递增）”与“采用最新更改（覆盖并更新）”之间做出选择。
3. **修复文档切换缓存穿透**：修改 `loadCachedAnalysis` 逻辑，在缓存数据未命中的 Catch 或 `!cached` 分支下，异步自动调用 `handleAnalyzeInternal('auto')` 触发冷启动分析，保证任何文档切换均能正常展现语法分析。

### ⚙️ 第二阶段：类型规范化与模块解耦（P1 - 重要）
1. **重构 API 错误类型封装**：在 `api.ts` 中提取 JSON 的 `ApiErrorResponse`，抛出强类型 `ApiError` 异常，替换前端所有的 `message.includes(...)` 脆弱逻辑。
2. **TTS 服务层与存储层解耦（依赖倒置）**：在后端 `src/tts.rs` 中使用 Rust Trait 屏蔽 SQLite `AppDatabase` 和 `TtsCacheRepository` 的物理依赖，使其符合整洁架构标准。

### 🧹 第三阶段：单体降噪与性能调优（P2 - 持续优化）
1. **主页单体拆分**：将 `App.tsx` 的复杂定时器、草稿同步、音频播放逻辑提炼为多个自定义 `React Hooks`，将页面渲染简化为受控的状态展示树。
2. **移入草稿管理切片**：采用 Zustand Slice 将全局的文本 draft 缓冲区交由状态树直接驱动，避免使用副作用 `useEffect` 在本地和服务器端状态之间进行粗暴桥接。
