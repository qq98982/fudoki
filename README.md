# Fudoki (フドキ)

> 日本語を「見える化」する Web ツール（テキスト解析＆音声読み上げ）
>
> An interactive Japanese text analysis and speech synthesis web app
>
> 让日语结构可视化的 Web 工具（文本分析与语音朗读）

![Screenshot](static/fudoki.png)

---

## English

### Overview
Fudoki is now a local-first React frontend served by a Rust backend. Rust handles Sudachi-based analysis, dictionary lookup, remote TTS, and SQLite persistence for documents and settings.

### Features
- React workspace with a simplified multi-document rail, editor surface, and inspector panel.
- Local SQLite persistence for documents and settings.
- Text analysis: Rust backend segmentation, POS tags, and reading metadata.
- Dictionary: JMdict integration via Rust API.
- TTS: system/browser speech plus OpenAI-compatible remote speech.
- Legacy browser-data import from the previous frontend on first launch in the same browser.
- Multilingual UI and theme toggles.

### Usage
Online: https://fudoki.iamcheyan.com

Local:
```bash
./run.sh
```

Preferred launchers:

Linux/macOS:
```bash
./run.sh
```

Windows PowerShell:
```powershell
.\run.ps1
```

Windows Command Prompt:
```bat
run.bat
```

Launcher behavior:
- checks required tools
- runs `git lfs pull` if `resources/system.dic` is missing
- runs `npm --prefix frontend install` when frontend dependencies are missing
- runs `npm --prefix frontend run build`
- runs `cargo build`
- starts the server with `cargo run`
- then open http://127.0.0.1:8000

LAN sharing:
- Fudoki listens on `127.0.0.1:8000` by default, so only the current machine can connect.
- To allow other machines on the same local network, start it with `FUDOKI_BIND_ADDR=0.0.0.0:8000`.
- You can also persist that setting by adding `FUDOKI_BIND_ADDR=0.0.0.0:8000` to your local `.env`.
- Linux/macOS: `FUDOKI_BIND_ADDR=0.0.0.0:8000 ./run.sh`
- Windows PowerShell: `$env:FUDOKI_BIND_ADDR="0.0.0.0:8000"; .\run.ps1`
- Windows Command Prompt: `set FUDOKI_BIND_ADDR=0.0.0.0:8000 && run.bat`
- Then open `http://<this-computer-lan-ip>:8000` from another machine.
- On Windows, allow inbound access to port 8000 if Windows Firewall prompts or blocks it.

### Online TTS (OpenAI-compatible)
By default, Fudoki can use the browser's built-in Web Speech API ("system" TTS). To enable remote/online TTS, configure a local `.env` file (see `.env.example`) and pick the `openai-compatible` provider in Settings.

Relevant environment variables (defaults shown in `.env.example`):
```env
FUDOKI_TTS_DEFAULT_PROVIDER=openai-compatible
FUDOKI_TTS_OPENAI_BASE_URL=https://one-dev.felo.me/v1
FUDOKI_TTS_OPENAI_API_KEY=your_api_key_here
FUDOKI_TTS_OPENAI_MODEL=gpt-4o-mini-tts
FUDOKI_TTS_OPENAI_MODEL_OPTIONS=gpt-4o-mini-tts,gpt-audio-1.5,gpt-realtime-1.5
FUDOKI_TTS_OPENAI_VOICE=marin
FUDOKI_TTS_OPENAI_VOICE_OPTIONS=marin,cedar,alloy
```

When `FUDOKI_TTS_OPENAI_MODEL_OPTIONS` or `FUDOKI_TTS_OPENAI_VOICE_OPTIONS` are present, Fudoki uses those lists to populate the remote model and remote voice dropdowns in Settings. The `.env` values `FUDOKI_TTS_OPENAI_MODEL` and `FUDOKI_TTS_OPENAI_VOICE` are used as defaults; if a saved browser selection is no longer in the allowed list, Fudoki falls back to the `.env` default and then to the first valid option.

### Backend APIs
- `GET /api/health`
- `POST /api/analyze`
- `GET /api/dictionary?term=...`

### Part-of-Speech Colors
|  | POS |
|---|---|
| 🟢 | Noun |
| 🔵 | Verb |
| 🟠 | Adjective |
| 🟣 | Adverb |
| 🔴 | Particle |
| 🟡 | Interjection |

### Development
```
fudoki/
├── Cargo.toml
├── frontend/
│   ├── src/
│   └── dist/
├── src/
│   ├── main.rs
│   └── ...
├── resources/
│   ├── sudachi.json
│   └── system.dic
├── static/
│   └── ...
└── README.md
```
- Frontend source lives under `frontend/`.
- Built frontend assets are served from `frontend/dist/`.
- Rust persists local data in SQLite and serves the app and APIs.

### License and Third-party
- MIT License
- Kuromoji.js — Apache License 2.0
- JMdict — Creative Commons Attribution-ShareAlike 3.0

### Contributing and Feedback
Pull requests are welcome. For issues and feature requests, use GitHub Issues: https://github.com/iamcheyan/fudoki/issues

---

## 日本語

### 概要
Fudoki は React フロントエンドを Rust バックエンドから配信するローカル優先アプリです。Rust は Sudachi による解析、辞書参照、リモート TTS、SQLite による文書と設定の永続化を担当します。

### 主な機能
- 簡素化した複数文書リスト、編集領域、インスペクタを備えた React ワークスペース。
- 文書と設定を SQLite に保存するローカル永続化。
- 形態素解析：Rust バックエンドによる分割、品詞、読み情報。
- 辞書：Rust API 経由の JMdict 参照。
- TTS：ブラウザの system 音声と OpenAI 互換の remote 音声。
- 旧フロントエンドのブラウザ内データを初回起動時に移行。
- 多言語 UI とテーマ切替。

### 使い方
オンライン：https://fudoki.iamcheyan.com

ローカル：
```bash
./run.sh
```

推奨ランチャー:

Linux/macOS:
```bash
./run.sh
```

Windows PowerShell:
```powershell
.\run.ps1
```

Windows コマンドプロンプト:
```bat
run.bat
```

ランチャーの動作:
- 必要なコマンドをチェック
- `resources/system.dic` がない場合、`git lfs pull` を実行して必要なアセットを取得
- フロントエンド依存がなければ `npm --prefix frontend install` を実行
- `npm --prefix frontend run build` を実行
- `cargo build` を実行
- `cargo run` でサーバーを起動
- ブラウザで http://127.0.0.1:8000 を開く

LAN 共有:
- Fudoki はデフォルトで `127.0.0.1:8000` を待ち受けるため、現在のマシンからのみ接続できます。
- 同じローカルネットワーク上の別マシンから接続する場合は、`FUDOKI_BIND_ADDR=0.0.0.0:8000` を指定して起動します。
- ローカルの `.env` に `FUDOKI_BIND_ADDR=0.0.0.0:8000` を追加して永続化することもできます。
- Linux/macOS: `FUDOKI_BIND_ADDR=0.0.0.0:8000 ./run.sh`
- Windows PowerShell: `$env:FUDOKI_BIND_ADDR="0.0.0.0:8000"; .\run.ps1`
- Windows コマンドプロンプト: `set FUDOKI_BIND_ADDR=0.0.0.0:8000 && run.bat`
- 別マシンのブラウザで `http://<このマシンのLAN内IP>:8000` を開きます。
- Windows でファイアウォールの確認が出る、または遮断される場合は、8000 番ポートの受信を許可してください。

### 品詞色分け
| 色 | 品詞 |
|---|---|
| 🟢 | 名詞 |
| 🔵 | 動詞 |
| 🟠 | 形容詞 |
| 🟣 | 副詞 |
| 🔴 | 助詞 |
| 🟡 | 感動詞 |

### Markdown サポート

標準的なテキストエリアを **EasyMDE** Markdown エディタに置き換えました。日本語解析機能とは完全に互換性があります：

- **リッチテキスト編集**：ツールバーでクイック書式設定（太字、斜体、見出し、リスト、引用、リンク、画像）
- **ライブプレビュー**：サイドバイサイドの Markdown プレビューモード
- **全画面モード**：集中執筆環境
- **シンタックスハイライト**：視覚的な Markdown 構文サポート
- **シームレスな統合**：Markdown コンテンツで日本語解析が自動的に機能

Markdown 統合の詳細なドキュメントは [MARKDOWN_README.md](./MARKDOWN_README.md) をご覧ください。

### 開発情報
- テーマカラー：`static/styles.css` の CSS 変数を編集。
- JMdict データ：`static/libs/dict/` に配置。

### ライセンスと利用ライブラリ
- MIT License
- Kuromoji.js — Apache License 2.0
- JMdict — Creative Commons Attribution-ShareAlike 3.0

### 貢献・フィードバック
Issue／PR を歓迎します。https://github.com/iamcheyan/fudoki/issues

---

## 中文

### 概述
Fudoki 通过本地 Rust 后端提供前端页面，使用 Sudachi 进行日语分词与词性标注，并通过 Web Speech API 朗读文本。

### 功能
- **Markdown 编辑器**：内置 EasyMDE markdown 编辑器，支持富文本格式，同时保持完整的日语分析能力。
- 文本分析：分词、词性、假名与罗马音。
- 语音合成：按单词/按行/全文播放；语速 0.5–2.0；音色选择。
- 播放控制：暂停/继续为独立按钮；播放中播放按钮显示"停止"图标。
- 即时设置生效：播放中更改语速或音色，会先暂停再在当前段附近按新设置续播；设置持久化到 localStorage。
- 词典：整合 JMdict；点击词卡查看释义。
- 文档：多文档管理、自动保存、快速切换。
- 界面：暗色模式、显示切换、多语言 UI、工具栏可拖拽。
- 移动端：≤480px 时压缩头部语速滑条与音色下拉宽度；左侧按钮、右侧设置。

### 使用
在线版：https://fudoki.iamcheyan.com

本地运行：
```bash
npm install
```

推荐启动脚本：

Linux/macOS:
```bash
./run.sh
```

Windows PowerShell:
```powershell
.\run.ps1
```

Windows 命令提示符:
```bat
run.bat
```

脚本行为：
- 检查所需命令
- 当 `resources/system.dic` 缺失时，执行 `git lfs pull` 拉取所需资源
- 执行 `cargo build`
- 使用 `cargo run` 启动服务
- 浏览器访问 http://127.0.0.1:8000

局域网共享：
- Fudoki 默认监听 `127.0.0.1:8000`，只有当前机器能访问。
- 如果要让同一局域网内的其他机器访问，启动时设置 `FUDOKI_BIND_ADDR=0.0.0.0:8000`。
- 也可以把 `FUDOKI_BIND_ADDR=0.0.0.0:8000` 写进本机 `.env`，以后启动会自动使用。
- Linux/macOS：`FUDOKI_BIND_ADDR=0.0.0.0:8000 ./run.sh`
- Windows PowerShell：`$env:FUDOKI_BIND_ADDR="0.0.0.0:8000"; .\run.ps1`
- Windows 命令提示符：`set FUDOKI_BIND_ADDR=0.0.0.0:8000 && run.bat`
- 然后在其他机器浏览器打开 `http://<这台机器的局域网IP>:8000`。
- 如果 Windows 防火墙提示或拦截，需要允许 8000 端口入站访问。

### 词性颜色
| 颜色 | 词性 |
|---|---|
| 🟢 | 名词 |
| 🔵 | 动词 |
| 🟠 | 形容词 |
| 🟣 | 副词 |
| 🔴 | 助词 |
| 🟡 | 感叹词 |

### Markdown 支持

应用现在内置了 **EasyMDE** markdown 编辑器，替换了标准的 textarea，同时完全保持日语分析功能的兼容性：

- **富文本编辑**：使用工具栏快速格式化（粗体、斜体、标题、列表、引用、链接、图片）
- **实时预览**：并排 markdown 预览模式
- **全屏模式**：专注写作体验
- **语法高亮**：可视化 markdown 语法支持
- **无缝集成**：日语分析功能自动作用于 markdown 内容

有关 markdown 集成的详细文档，请参阅 [MARKDOWN_README.md](./MARKDOWN_README.md)。

### 开发信息
- 主题颜色：编辑 `static/styles.css` 中的 CSS 变量。
- JMdict 数据：放置在 `static/libs/dict/`。

### 许可与第三方
- MIT License
- Kuromoji.js — Apache License 2.0
- JMdict — Creative Commons Attribution-ShareAlike 3.0

### 贡献与反馈
欢迎 Issue／PR：https://github.com/iamcheyan/fudoki/issues

---

## Name Origin / 名称の由来 / 名称由来

### English
Fudoki is named in homage to Japan’s ancient regional gazetteers “Fudoki”.
“Fudo” conveys the atmosphere and character of place and culture; “Ki” means to record.
This app similarly “records the climate of language”—prosody, rhythm, phonology, and grammar—by segmenting text, labeling parts of speech and readings, and reassembling it for spoken output. It is not the book itself, but a calm tool inspired by that spirit of attentive recording.

### 日本語
この名称 **フドキ** は、奈良時代の地誌『**風土記（ふどき）**』へのオマージュです。
日本人が「フドキ」と聞くと、多くの場合この古代の記録書を思い出します。そこには土地、暮らし、風俗、文化が静かに、しかし丹念に記されています。

- 「風土」＝地域や文化の空気感・肌ざわり
- 「記」＝記すこと、記録すること

このアプリは、まさに「言葉の風土」を記録し、見える化するための道具です。文を分解し、品詞や読み、音のリズムを捉え、発音として再構成する――それは『風土記』が土地の景色を一つひとつ書き留めた営みによく似ています。歴史書そのものではなく、その精神への敬意としての命名です。

### 中文
**Fudoki（フドキ）** 的名字向日本奈良时代的古代地志《**风土记**》致敬。

- 「风土」＝地域与文化的气息与肌理
- 「记」＝记录、书写

本应用做的，正是“记录语言的风土”：把句子拆解成词语，标注词性与读音，把语感、节奏、声韵与语法结构重新组合，并以语音方式呈现。这与《风土记》逐条记录土地与民俗的工作在结构上高度一致。它不是历史书本身，而是对那种“安静而细致地记录世界”的精神的致敬——让语言的风土逐步显形。

---

## Appendix (Brand & History)

### Brand
<div align="center">

Made with ❤️ for Japanese language learners worldwide

世界中の日本語学習者のために ❤️ を込めて

为全世界的日语学习者用心打造 ❤️

</div>

### Star History

[![Star History Chart](https://api.star-history.com/svg?repos=iamcheyan/fudoki&type=Date)](https://star-history.com/#iamcheyan/fudoki&Date)
