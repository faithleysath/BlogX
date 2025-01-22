# BlogX

> 极简之美：专注本质的静态博客生成器

![PyPI - Downloads](https://img.shields.io/pypi/dm/blogx)

## 简介

欢迎来到 BlogX，一个以极简主义理念打造的静态博客生成器，它将简单性和功能性完美结合。BlogX 以“专注做好一件事”为核心哲学，专注于将 Markdown 文件转换为 HTML，让您的博客体验变得轻松而高效。对于重视简洁、直接博客体验的开发者来说，它是完美的工具。

## 原理

具体来说，BlogX 在构建的过程中做了以下几个步骤：
1. 拷贝静态资源文件到输出目录
2. 复制src目录结构到输出目录
3. 将Markdown文件转换为HTML文件

## 特点

BlogX 的特点如下：
- **极简 CSS 和 JS**：BlogX 主题设计时尽可能减少 CSS 和 JavaScript 的使用，确保即使在禁用 JavaScript 的情况下，您的博客也能保持良好的显示效果（甚至包括$\LaTeX$公式）。
- **低学习曲线**：BlogX 直观易用，易于理解，无论是新手还是经验丰富的开发者都能快速上手。
- **单一职责**：BlogX 专注于一个任务：将 Markdown 文件转换为 HTML 并管理静态文件，让您自由组织网站结构。
- **文章保护功能**：BlogX 新增了文章保护功能，您可以通过将 Markdown 文件命名为 .protect.md 来保护文章内容。保护的文章会生成加密的 HTML 文件，只有通过解密才能查看内容。同时支持强大的防复制、反截屏功能。

BlogX 不会做以下几件事：
- **Tag标签与Category分类**：BlogX 不提供 Tag 标签和 Category 分类功能。当然，您可以自行手动维护这些信息。
- **Index首页**：BlogX 不会自动生成首页，这意味着您需要自行编写`index.md`文件，以展示您的博客列表。
- **评论系统**：BlogX 不会集成评论系统，您可以使用第三方评论系统，如 Disqus。

除此以外，您可以自由定制您的博客目录结构，BlogX 不会强制您使用特定的目录结构。

## 快速开始

1. 安装 BlogX

```bash
pip install blogx
```

2. 初始化博客

```bash
blogx init
```

3. 查看目录结构

```
C:.
├─src
│  │  index.md
│  │
│  ├─haha
│  │  │  index.md
│  │  │
│  │  └─nok
│  │          dule.md
│  │
│  ├─some-article
│  │      hello.md
│  │
│  ├─static
│  │  └─img
│  │          1.bmp
│  │
│  └─_global
│          BLOGNAME
│          footer.md
│          header.md
│          sidebar.md
│
└─theme
    │  template.html
    │
    └─static
        ├─css
        └─js
```

其中`src`目录存放博客源文件，`theme`目录存放主题文件。`_global`目录存放全局文件，该目录不会被转换为HTML文件并复制到输出目录。

| 文件名 | 作用 |
| --- | --- |
| BLOGNAME | 博客名称 |
| footer.md | 页脚 |
| header.md | 页眉 |
| sidebar.md | 侧边栏 |

4. 构建博客

```bash
blogx build
```

5. 查看输出目录

```
C:.
│  index.html
│  template.html
│
├─haha
│  │  index.html
│  │
│  └─nok
│          dule.html
│
├─some-article
│      hello.html
│
└─static
    ├─css
    ├─img
    │      1.bmp
    └─js
```

6. 实时预览博客

```bash
blogx serve
```

该命令会在本地启动一个HTTP服务器，并实时监测文件变化，自动刷新页面。

7. 部署博客

```bash
blogx deploy
```

请确保该目录是一个Github Pages仓库，BlogX会自动生成action文件并推送到Github仓库。

## 贡献
我们欢迎对 BlogX 的贡献。无论您是想要添加新功能的开发者，还是发现漏洞的用户，您的贡献都是宝贵的。在提交拉取请求之前，请阅读我们的贡献指南。

## 许可证
BlogX 是开源的，并在 MIT 许可证 下可用。您可以自由使用、修改和分发 BlogX。

## 截图
![BlogX](example.png)