# BlogX

BlogX 是一个 Rust 编写的 HTML+CSS-first 静态站点编译器。它面向个人博客、知识库和长期维护的文本网站：内容用文件树组织，页面由 Markdown 生成，主题由普通 HTML 模板和静态资源组成。

核心模型很小：一个 Markdown 文件就是一个页面，`content/` 编译到 `public/`。BlogX 不自动生成首页、归档、标签页、分类页或导航树；这些页面都由作者作为普通内容维护。它只负责把内容稳定、快速、可预测地编译成静态 HTML。

当前 `main` 是 Rust 重写版本，旧 Python 版本已归档在 `archive/python-main` 分支。Rust 版不兼容旧 Python 包、旧字符串替换模板、旧 `src/_global` 布局和旧 `.protect.md` 行为。

## 特性

- Rust 单二进制 CLI，无 Node/Vite/Tailwind/Sass 构建链要求。
- Markdown 基于 `comrak`，支持 CommonMark、表格、脚注、任务列表、标题锚点和围栏代码块。
- 构建期 KaTeX 数学公式渲染，默认主题自带本地 KaTeX CSS、字体和 copy-tex 增强脚本。
- 构建期 `syntect` 代码高亮，行号和横向滚动由静态 HTML/CSS 完成。
- MiniJinja 主题系统，显式模板上下文和 `asset_url` / `url_for` / `markdown` helper。
- `.blogx/cache/` 持久缓存、KaTeX 表达式缓存、并行页面渲染和增量静态资源复制。
- 主题资源复制、可选 fingerprint、CSS `url(...)` 重写和旧主题资源清理。
- 默认主题不依赖外部 CDN 或在线字体，禁用 JavaScript 时仍可读、可导航。
- 内置 `serve` 预览、文件监听、`sitemap.xml` 和 `robots.txt` 生成。

## 安装

通过 npm 安装预编译二进制：

```bash
npm install -g @faithleysath/blogx
```

从源码安装当前仓库版本：

```bash
cargo install --path .
```

开发时可以直接运行：

```bash
cargo run -- --help
cargo run -- init my-site
```

当前仓库版本可用：

```bash
blogx --version
```

## 快速开始

```bash
blogx init my-site
cd my-site
blogx build --profile
blogx serve --host 127.0.0.1 --port 8000
```

生成的项目结构：

```text
blogx.toml
content/
  index.md
  about.md
  notes/
    hello.md
  _partials/
    header.md
    footer.md
    sidebar.md
  _drafts/
    unfinished.md
theme/
  theme.toml
  layouts/
  partials/
  assets/
public/
```

规则很直接：

- `content/**/*.md` 生成页面。
- `content/` 下的非 Markdown 文件作为静态资源复制。
- 任意以 `_` 开头的目录是私有输入，不会直接发布。
- `theme/` 提供布局、partial 和主题静态资源。
- `public/` 是完整输出目录。

## CLI

```bash
blogx init <name>
blogx build [--clean] [--no-cache] [--jobs <n>] [--profile]
blogx serve [--host <host>] [--port <port>] [--jobs <n>]
blogx theme sync [--dest <path>] [--prune]
blogx clean
```

常用 flag：

- `build --clean`：构建前清空输出目录。
- `build --no-cache`：本次构建禁用页面和资源缓存。
- `build --jobs <n>`：指定并行渲染线程数。
- `build --profile`：输出构建阶段耗时。
- `serve --host <host>`：指定预览服务监听地址。
- `serve --port <port>`：指定预览服务端口。
- `theme sync`：把 BlogX 内置默认主题同步到当前项目的 `paths.theme`。
- `theme sync --dest <path>`：同步到指定目录。
- `theme sync --prune`：同步后删除目标主题目录里默认主题不存在的额外文件。

旧版 `deploy` 命令没有进入 Rust 重写版。部署建议交给 GitHub Actions、rsync、对象存储 CLI 或其他外部发布流程。

## 内容模型

Front matter 可选。当前页面级字段包括：

```yaml
---
title: "About"
layout: "page.html"
draft: false
---
```

URL 模式由 `blogx.toml` 的 `[theme] url_mode` 控制：

- `html`：`about.md` 输出为 `about.html`
- `clean`：`about.md` 输出为 `about/index.html`，公开链接为 `about/`

BlogX 会改写指向本地 Markdown 页面的链接，例如 `[About](about.md)`。外部链接、锚点、绝对路径、URI scheme 链接和非 Markdown 链接会保持原样。

内容 partial 位于 `content/_partials/`。Markdown partial 会渲染成 HTML，HTML partial 会原样传入模板，key 由相对路径决定：

- `content/_partials/sidebar.md` -> `partials.sidebar`
- `content/_partials/sections/sidebar.md` -> `partials["sections.sidebar"]`
- `content/_partials/nav/index.html` -> `partials.nav`

未被模板访问的 partial 会产生 warning，方便清理废弃内容。

## 主题系统

主题是一个普通目录：

```text
theme/
  theme.toml
  layouts/
    page.html
  partials/
    head.html
    header.html
    footer.html
    nav.html
  assets/
    css/
    js/
    fonts/
```

模板使用 MiniJinja。页面模板可以访问：

- `site`
- `page`
- `content`
- `partials`
- `assets`
- `build`

内置 helper：

- `asset_url("css/main.css")`
- `url_for("about.md")`
- `markdown("**small fragment**")`

默认主题是 Rust 重写后的新主题，不是旧 Python 模板代码的逐行迁移，但保留了旧模板的主要 UI 气质：文章和右侧栏布局、红色链接、居中标题、浅色纸面、虚线侧栏、暖色引用块、图片阴影、深色代码块和外链标记。

默认主题的约束：

- 不依赖外部 CDN 完成核心渲染。
- 不依赖在线字体。
- 本地自带 KaTeX CSS 和字体。
- 语义化 HTML、skip link、清晰 focus 样式。
- 响应式布局，宽屏侧栏在右侧，窄屏侧栏落到底部。
- 支持 `prefers-reduced-motion`。
- JavaScript 只做渐进增强，例如图片 lightbox、代码复制提示和 KaTeX copy-tex。

同步内置默认主题：

```bash
blogx theme sync
blogx theme sync --prune
```

## 缓存机制

BlogX 有两套持久缓存。

构建缓存位于：

```text
.blogx/cache/manifest.json
```

它记录 BlogX 版本、渲染指纹、页面输入 hash、输出路径、diagnostics、使用过的 partial、静态资源 hash 和上次输出列表。页面命中缓存需要满足：

- 没有使用 `--no-cache`
- BlogX 版本未变
- `blogx.toml`、`theme/` 和 `content/_partials/` 形成的渲染指纹未变
- 页面内容 hash 未变
- 输出路径未变
- 对应输出文件仍存在

KaTeX 缓存位于：

```text
.blogx/cache/katex.json
```

它按 BlogX 版本、inline/display 模式、KaTeX 配置和公式源码缓存渲染结果。重复出现的公式不会反复调用 KaTeX 渲染。

当前缓存策略偏保守：主题、配置或 partial 变化会让页面缓存整体失效。日常只改文章时，BlogX 会只重渲染变化的页面；改全站布局或默认主题时，会按正确性优先重新渲染相关输出。

## Benchmark

以下数字来自本机参考测试，不是跨机器承诺。测试环境：

```text
BlogX: 0.4.9 release build
CPU: Intel Core i5-14600KF, 20 threads
Command: blogx build --profile --jobs 20
```

真实博客当前规模：

| 场景 | 页面 | 资源 | 总耗时 |
| --- | ---: | ---: | ---: |
| `--clean --no-cache` 冷构建 | 24 rendered | 9 copied | 54ms |

合成 1000 页轻量站点：

| 场景 | 结果 | 总耗时 |
| --- | --- | ---: |
| 冷构建 | 1001 rendered, 0 cached | 88ms |
| 无变化热构建 | 0 rendered, 1001 cached | 18ms |
| 改 2 篇、加 1 篇 | 3 rendered, 999 cached | 20ms |

合成 1000 页重排版站点：

这个用例每篇包含多段 inline/display KaTeX、表格、脚注、任务列表、嵌套列表、引用块和两个代码块，用来压 Markdown、KaTeX 和高亮路径。

| 场景 | 结果 | 总耗时 | Markdown 累计 | KaTeX 累计 | 代码高亮累计 |
| --- | --- | ---: | ---: | ---: | ---: |
| 冷构建 | 1001 rendered, 0 cached | 194ms | 2174ms | 788ms | 50ms |
| 无变化热构建 | 0 rendered, 1001 cached | 15ms | 0ms | 0ms | 0ms |
| 改 2 篇、加 1 篇 | 3 rendered, 999 cached | 42ms | 35ms | 20ms | 8ms |

复现方式：

```bash
cargo build --release
target/release/blogx build --clean --no-cache --profile --jobs 20
target/release/blogx build --profile --jobs 20
```

解释这些数字时要注意：`page render` 是并行 wall time，而 profile 里的 `markdown render`、`katex render`、`code highlighting` 是各页面累计耗时，因此可能大于总耗时。

## 配置概览

`blogx init` 会生成一份完整 `blogx.toml`。核心字段：

```toml
[site]
title = "My Blog"
base_url = "/"

[paths]
content = "content"
output = "public"
theme = "theme"
cache = ".blogx/cache"

[theme]
default_layout = "page.html"
url_mode = "html"

[markdown]
heading_anchors = true
unsafe_html = true

[katex]
throw_on_error = true
error_color = "#cc0000"
trust = false

[code]
theme = "base16-ocean.dark"
line_numbers = true

[assets]
fingerprint = false

[site_infra]
sitemap = true
robots = true
```

## 开发与验收

常规检查：

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

端到端 smoke：

```bash
bash scripts/no-js-smoke.sh
bash scripts/accessibility-smoke.sh
bash scripts/browser-acceptance.sh
bash scripts/serve-watch-smoke.sh
bash scripts/release-artifact-smoke.sh
```

设计约束和验收文档：

- `docs/rust-rewrite-decisions.md`
- `docs/manual-acceptance.md`
- `docs/constraint-compliance-checklist.md`
