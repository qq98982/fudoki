# Fudoki React Rewrite Progress Handoff

## 当前目标

本分支正在把旧版静态前端重写为：

- React 前端
- Rust 后端
- 本地 SQLite 持久化
- 本地优先，不做登录/同步

当前分支：`feat/react-frontend-rewrite`

---

## 当前已经完成

### 1. 后端持久层已经落地

Rust 现在已经接入 SQLite，本地持久化已经不是设计稿，而是可运行状态。

已完成：

- 文档表、设置表、分析缓存表、远程 TTS 缓存表
- SQLite 初始化和建表
- 文档 CRUD API
- 设置读写 API
- 旧浏览器 `localStorage` 数据导入 API
- 分析缓存 API
- 远程 TTS 持久缓存元数据 + 音频文件落盘

核心文件：

- `src/app.rs`
- `src/models.rs`
- `src/storage/db.rs`
- `src/storage/documents_repo.rs`
- `src/storage/settings_repo.rs`
- `src/storage/analysis_cache_repo.rs`
- `src/storage/tts_cache_repo.rs`
- `src/tts.rs`

### 2. React 前端主工作台已经可用

React 前端已经替换掉 Vite 默认页，具备基本产品壳：

- 左侧简化文档列表
- 中间编辑区
- 右侧分析 / 词典 / TTS / 设置面板
- Rust 直接服务 `frontend/dist`

核心文件：

- `frontend/src/App.tsx`
- `frontend/src/features/documents/DocumentRail.tsx`
- `frontend/src/features/editor/EditorPane.tsx`
- `frontend/src/features/inspector/InspectorPanel.tsx`
- `frontend/src/lib/api.ts`
- `frontend/src/index.css`

### 3. 下方分析区已经恢复到可用状态

当前已经恢复：

- 编辑区下方常驻分析带
- 使用 Rust / Sudachi 返回的 token 直接渲染
- 自动分析：输入停顿后自动刷新
- 手动分析：`Analyze` 按钮可强制刷新
- 每行右侧播放按钮
- 点击 token 联动右侧词典
- 右下角悬浮快捷按钮

核心文件：

- `frontend/src/features/analysis/AnalysisStrip.tsx`
- `frontend/src/features/analysis/analysisTokens.ts`
- `frontend/src/App.tsx`

### 4. 已经收口的显示规则

分析带当前已经按旧版要求过滤这些内容：

- Markdown 标记，例如 `#`
- 括号本身，例如 `(` `)` `（` `）`
- 括号中的整段说明内容，例如 `（契約条項）`
- `空白` token
- 纯空格 token

普通标点当前策略：

- `。`、`、`、`！`、`？` 等只显示字面
- 不显示注音
- 不显示罗马字
- 不显示词性
- 不作为可点击 token

这部分逻辑在：

- `frontend/src/features/analysis/analysisTokens.ts`

### 5. 已修复的重要 bug

#### 切换文档后 `Stop playback` 失效

根因：

- 旧实现没有统一的 playback session 概念
- 远程 TTS 请求在飞行中时，切换文档后点 Stop 只能停掉“当前对象”，挡不住旧请求返回后继续播放

当前修复方式：

- 引入统一 playback session
- Stop 会直接作废当前 session
- 旧的 remote TTS 响应即使晚到，也不会再启动播放
- system TTS 和 remote TTS 都走统一停止路径

主要改动：

- `frontend/src/App.tsx`
- `frontend/src/lib/systemTts.ts`

---

## 当前还没做完的部分

### 1. 词块视觉还没有完全贴近旧版

现在功能已经恢复，但样式还是“新版本的近似实现”，还没完全贴近旧版截图。

还可以继续做：

- 词块尺寸和间距更像旧版
- 注音/罗马字/词面的垂直节奏更稳
- 颜色条更接近旧版视觉语言
- hover / active / playing 状态更细

主要文件：

- `frontend/src/index.css`
- `frontend/src/features/analysis/AnalysisStrip.tsx`

### 2. 词级播放按钮还没恢复

目前已经恢复：

- 全文播放
- 每行播放

但还没恢复：

- 每个 token 自己的播放按钮

建议做法：

- 不要把播放按钮永远显示在每个 token 上，界面会太密
- 更好的做法是 hover 或 active 时显示

### 3. 词详情仍然偏简化

现在是：

- 点击 token 后右侧词典联动

还没做的是：

- 旧版那种更丰富的词条详情层
- 更完整的词性/备注/多义项展示
- 更接近旧版的“点词展开”体验

### 4. 数字读音还没补

用户明确希望：

- `第 1 条` 这种数字尽量自动标读音
- 如果有成熟判断方法，优先用成熟方法

目前还没做。

注意事项：

- 前端不要自己分词
- token 边界必须尊重 Rust / Sudachi 返回结果
- 真正要做的是“数字 token 的显示读音增强”，不是重做 tokenizer

建议实现方向：

- 优先在 Rust 端补一层 number reading 规范化
- 对独立数字、`第 + 数字 + 条`、日期等常见场景做增强
- 前端只消费增强后的 `reading`

### 5. 编辑器仍然是 `textarea`

当前是为了先把主链路打通。

后续可以升级到：

- CodeMirror 6

但要注意：

- 不要在升级编辑器前打乱当前自动保存 / 自动分析链路
- 先把现在的交互稳定后再换编辑器内核

---

## 当前测试状态

这轮已经通过的验证命令：

### 仓库根目录

```bash
npm test
```

这会实际执行：

```bash
cargo test && npm run frontend:test && npm run frontend:build
```

### 单独前端

```bash
cd frontend
npm test
npm run build
```

### 单独 Rust

```bash
cargo build
```

---

## 建议的下一步顺序

如果下次继续开发，推荐按下面顺序：

1. 收紧分析带视觉
2. 恢复 token 级播放按钮
3. 丰富 token 详情和词典交互
4. 做数字读音增强
5. 再考虑把编辑器升级到 CodeMirror 6

原因：

- 现在主链路已经通了
- 分析区是用户当前最关注的视觉与交互区域
- 数字读音增强会涉及后端逻辑，最好在 UI 稳定后单独做

---

## 继续开发时必须记住的规则

### 1. 不要在前端重新分词

必须使用 Rust / Sudachi 返回的 token 作为唯一分词边界。

前端只能做：

- 显示过滤
- 显示增强
- 交互联动

不能做：

- 自己重新切 token
- 自己重排词边界

### 2. 括号注释内容继续保持隐藏

像下面这种：

- `（契約条項）`
- `（尊敬）`
- `（谦让）`

都不应该出现在分析带里。

### 3. 标点只显示字面

不要给这些内容加：

- 注音
- 罗马字
- 词性
- 可点击交互

### 4. 播放状态必须继续走 playback session

不要回退成“谁拿到音频谁就直接播放”的写法。

否则：

- 切文档
- Stop
- 异步返回

这些时序问题会再次把 bug 带回来。

---

## 当前关键文件导航

### 后端

- `src/app.rs`
- `src/models.rs`
- `src/tts.rs`
- `src/storage/db.rs`
- `src/storage/documents_repo.rs`
- `src/storage/settings_repo.rs`
- `src/storage/analysis_cache_repo.rs`
- `src/storage/tts_cache_repo.rs`

### 前端

- `frontend/src/App.tsx`
- `frontend/src/lib/api.ts`
- `frontend/src/lib/i18n.ts`
- `frontend/src/lib/systemTts.ts`
- `frontend/src/features/analysis/AnalysisStrip.tsx`
- `frontend/src/features/analysis/analysisTokens.ts`
- `frontend/src/features/editor/EditorPane.tsx`
- `frontend/src/features/inspector/InspectorPanel.tsx`
- `frontend/src/index.css`

### 测试

- `frontend/src/app/App.test.tsx`
- `frontend/src/features/analysis/AnalysisStrip.test.tsx`
- `tests/documents_api.rs`
- `tests/settings_api.rs`
- `tests/legacy_migration_api.rs`
- `tests/analysis_cache_api.rs`
- `tests/tts_speak_api.rs`

---

## 一句话状态

现在不是“刚起步”，而是：

- Rust + SQLite 主链路已经通
- React 工作台已经可用
- 下方分析区已经恢复到能继续精修的阶段

下一次继续开发，应该直接在“分析区视觉/交互收口 + 数字读音增强”上推进，而不是重新搭架子。
