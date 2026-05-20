# BlogX

BlogX 是一个用 Rust 编写的 HTML+CSS 优先静态站点编译器。

它保留一个很小的心智模型：文件树就是网站，一个 Markdown 文件就是一个页面。首页、目录页、导航、归档、标签页和分类页都由作者作为普通内容维护；BlogX 只负责渲染页面、复制静态资源、应用模板，并生成 `sitemap.xml` 和 `robots.txt` 这类机器可读的站点基础文件。

这个 Rust 版本是一次破坏性重写。它不兼容旧 Python 包、不保留旧字符串替换模板、不保留旧 `src/_global` 布局，也不保留旧 `.protect.md` 行为。旧 Python 版本已归档在 `archive/python-main` 分支。

## 状态

当前 `main` 分支包含 `docs/rust-rewrite-decisions.md` 描述的 Rust 重写版本。实现已经可以用于本地构建、预览和发布前验收；兼容策略以新的 Rust 模型为准，而不是无迁移复用旧 BlogX 项目。

## 安装

从仓库源码安装：

```bash
cargo install --path .
```

开发时常用命令：

```bash
cargo run -- --help
cargo run -- init my-site
```

## 快速开始

```bash
blogx init my-site
cd my-site
blogx build --profile
blogx serve --host 127.0.0.1 --port 8000
```

默认项目结构：

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
  _drafts/
    unfinished.md
theme/
  theme.toml
  layouts/
  partials/
  assets/
public/
```

`content/` 会编译到 `public/`。Markdown 文件会生成页面，非 Markdown 文件会作为静态资源复制。任何名称以 `_` 开头的目录都是私有输入，不会直接发布。

## 命令

```bash
blogx init <name>
blogx build
blogx build --clean
blogx build --no-cache
blogx build --jobs 8
blogx build --profile
blogx serve
blogx clean
```

旧版 `deploy` 命令没有进入第一版 Rust 重写。

## 内容模型

Front matter 是可选的。第一版只支持页面级渲染元数据：

```yaml
---
title: "About"
layout: "page.html"
draft: false
---
```

支持两种 URL 模式：

- `html`：`about.md` 输出为 `about.html`
- `clean`：`about.md` 输出为 `about/index.html`，公开链接为 `about/`

BlogX 只会改写指向本地 Markdown 页面的链接，例如 `[About](about.md)`。外部链接、锚点、绝对路径和非 Markdown 链接会保持原样。

## 渲染能力

Markdown 渲染基于 `comrak`，支持 CommonMark、表格、脚注、任务列表、标题锚点、围栏代码块，以及构建期数学公式渲染。

数学公式在构建期渲染为 KaTeX 兼容 HTML。默认主题自带本地 KaTeX CSS 和字体，不需要浏览器端 JavaScript 才能显示公式。

代码块使用 `syntect` 在构建期高亮。行号是静态 HTML/CSS，不依赖客户端脚本。

## 主题系统

主题使用 `theme/layouts/` 下的 MiniJinja 模板。

模板接收显式上下文对象：

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

内容 partial 位于 `content/_partials/`。Markdown partial 会渲染为 HTML，HTML partial 会原样传入模板，key 由相对路径确定。例如：

- `content/_partials/sidebar.md` 暴露为 `partials.sidebar`
- `content/_partials/sections/sidebar.md` 暴露为 `partials["sections.sidebar"]`
- `content/_partials/nav/index.html` 暴露为 `partials.nav`

如果 partial 没有被任何页面模板访问，构建会给出 unused partial warning。

## 默认主题

当前默认主题是 Rust 重写后的新主题，不是旧 Python 版本模板的原样迁移。

新主题的目标是满足 HTML+CSS-first 约束：

- 不依赖外部 CDN 完成核心渲染
- 不依赖外部字体完成核心渲染
- 使用本地 KaTeX CSS 和字体
- 使用本地 syntax CSS
- 语义化 HTML
- skip link
- 清晰的 focus 样式
- 响应式布局
- 支持 `prefers-reduced-motion`
- JavaScript 只用于复制代码按钮等渐进增强

旧默认模板仍可在 `archive/python-main` 分支查看。它依赖 Google Fonts、BootCDN、Font Awesome、jQuery、fancybox、toastr 和旧 protect 脚本；这些能力没有按原样带入 Rust 版默认主题。

## No-JS 原则

生成的网站在禁用 JavaScript 时仍必须可读、可导航。默认主题用 HTML 和 CSS 完成阅读、导航、响应式布局、数学公式、代码高亮和行号。JavaScript 只用于渐进增强，例如代码复制按钮。

## 性能与缓存

BlogX 使用编译器式构建流水线，包含并行页面渲染、`.blogx/cache/` 持久缓存、KaTeX 表达式缓存、静态资源增量复制、theme asset 清理和 orphan output 清理。

使用 profile 查看构建耗时：

```bash
blogx build --profile
```

profile 会显示 source scan、template load、partial render、Markdown、KaTeX、代码高亮、layout、asset copy、site infrastructure、cache 和 total build 时间。

## 开发与验收

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
bash scripts/no-js-smoke.sh
bash scripts/accessibility-smoke.sh
bash scripts/browser-acceptance.sh
bash scripts/serve-watch-smoke.sh
bash scripts/release-artifact-smoke.sh
```

发布前手工验收清单见 `docs/manual-acceptance.md`。

约束实现核对见 `docs/constraint-compliance-checklist.md`。
