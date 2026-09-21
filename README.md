# Glyph

一个基于 **Tauri 2 + Vue 3** 的跨平台桌面效率工具箱。左侧活动栏在多个工具之间切换,数据全部本地处理,不上传云端。当前版本 **v0.2.2**。

## 功能 / 工具

在 `src/tools.ts` 的工具箱注册表中声明,新增工具 = 在这里加一项 + 建对应组件。当前包含:

| 工具 | 说明 |
| --- | --- |
| **Markdown** | Markdown 阅读 / 编辑 / 实时预览。基于 markdown-it + highlight.js,支持大纲、任务列表、锚点、代码高亮;可内联本地 / 远程 / base64 图片;右侧文件信息面板可显隐;编码自动检测。 |
| **截图** | 全屏 / 区域截图,支持全局快捷键,截图结果以内联图片进入其它工具。 |
| **OCR 识别** | 图片文字识别。按平台分派:Windows/Linux 走本地推理,macOS 走原生 Vision。 |
| **开发者工具** | 面向开发的小工具集合,含 **HTTP 请求**(Postman 式:方法、URL、Headers、Body、历史记录)与**人民币大写**转换等。 |
| **PDF** | PDF 查看 / 处理(纯 Rust 渲染,无需外部 dll)。 |
| **文件编码** | 文本文件的编码检测与转换(GBK / UTF-8 等)。 |
| **图片工具箱** | 图片格式转换、压缩等本地处理。 |

> UI 提供中 / 英双语(`src/locales/`),跟随系统语言,可手动切换。

## 技术栈

- **桌面框架**:Tauri 2(Rust 后端 + 系统 WebView)
- **前端**:Vue 3.5(`<script setup>` + TypeScript)、Vite 8(rolldown)、Pinia、vue-i18n
- **编辑器 / 渲染**:CodeMirror 6、markdown-it、highlight.js
- **后端能力**:`src-tauri/` 下的 Rust 命令(文件 IO、编码、截图、OCR、PDF、图像处理等)

## 环境要求

- **Node**:`^22.18.0 || >=24.12.0`(见 `package.json` 的 `engines`)
- **pnpm**(仓库使用 pnpm 管理依赖,含 `pnpm-lock.yaml` / `pnpm-workspace.yaml`)
- **Rust**(stable)+ 各平台 Tauri 前置依赖(Windows 需 WebView2 / MSVC;Linux 需 `webkit2gtk` 等;详见 [Tauri 官方平台指南](https://tauri.app/start/prerequisites/))

## 开发

```sh
pnpm install

# 前端热更新(仅浏览器,不含 Tauri 宿主)
pnpm dev

# 完整桌面应用开发(启动 Vite + Tauri 宿主)
pnpm exec tauri dev
```

## 构建与打包

```sh
# 前端生产构建:type-check(vue-tsc) + vite build
pnpm build

# 打桌面安装包(Windows 示例,产物在 src-tauri/target/release/bundle/)
pnpm exec tauri build --bundles nsis
```

> `--bundles` 的合法取值随平台而变(Windows: `nsis` / `msi`;macOS: `dmg` / `app`;Linux: `deb` / `rpm` / `appimage`),不要传 `all`。

## 其它脚本

```sh
pnpm type-check   # vue-tsc 类型检查
pnpm test:unit    # Vitest 单元测试
pnpm lint         # oxlint + eslint(--fix)
pnpm format       # Prettier 格式化 src/
```

## 发版约定

- 发版通过**推送新的递增语义化 tag**(如 `v0.2.1` → `v0.2.2`)触发 GitHub Actions 三端 CI(`release.yml` 的 `on.push.tags: 'v*'`);**不复用、不 force 重指旧 tag**。
- tag 版本需与安装包内版本一致:同步 `src-tauri/Cargo.toml` 与 `src-tauri/tauri.conf.json` 的 `version`(`tauri.conf.json` 的 version 决定产物文件名,如 `Glyph_0.2.2_x64-setup.exe`)。`Cargo.lock` 的 glyph 版本由 cargo 构建时自动同步。

## 项目结构

```
.
├── src/                    # Vue 前端
│   ├── components/         # 通用组件 + tools/ 下各工具组件
│   ├── stores/             # Pinia(settings / persistent)
│   ├── locales/            # 中 / 英文案
│   └── tools.ts            # 工具箱注册表
├── src-tauri/              # Tauri(Rust)后端
│   ├── src/                # commands / markdown / ocr / pdf / screenshot 等模块
│   ├── capabilities/       # 权限与 fs scope
│   └── tauri.conf.json     # Tauri 配置(CSP、打包目标等)
└── README.md
```

## License

MIT
