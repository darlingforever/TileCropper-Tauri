# Tile Cropper (Tauri 重构版)

Windows 10 开始菜单磁贴切图工具 —— 从 Electron 迁移到 Tauri 2 + Rust + Vue 3。

## 与原版的区别

| 项目 | 原版 (Electron) | 本版 (Tauri) |
|------|----------------|--------------|
| 安装包体积 | ~150 MB | ~10 MB |
| 后端语言 | Node.js (CommonJS) | Rust |
| 前端框架 | 裸 JS + 手写 DOM | Vue 3 + TypeScript + Pinia |
| 快捷方式创建 | PowerShell (~400ms/次) | Win32 COM 直调 (~5ms) |
| 安全模型 | sandbox:false | Tauri 权限模型 |
| 热更新 | 重启 Electron (~8s) | Vite HMR (~0.3s) |
| 类型安全 | 无 | TypeScript 全链路 |

## 环境要求

### 必装（一次性）

1. **Visual Studio 2022 Build Tools + "Desktop development with C++" workload**
   - 下载：https://aka.ms/vs/17/release/vs_BuildTools.exe
   - 安装时勾选「使用 C++ 的桌面开发」工作负载
   - Rust 编译 Windows 原生二进制依赖 MSVC 的 link.exe

2. **Rust toolchain**
   ```powershell
   irm https://rustup.rs | iex
   ```
   安装后重启终端，确认 `rustc --version` 能跑

### 已有（无需安装）

- Node.js 22+
- Windows SDK 10.0.22621（MakeAppx / Signtool）
- .NET Framework 4.0（编译 C# 启动器用）
- WebView2（Win10/11 自带）

## 快速开始

```bash
# 1. 安装前端依赖
npm install

# 2. 生成 Tauri 图标（从 icon.png 生成各尺寸）
npx tauri icon src-tauri/icons/icon.png

# 3. 开发模式运行（会自动编译 Rust 后端）
npm run tauri dev

# 4. 打包发布版
npm run tauri build
```

首次 `npm run tauri dev` 会编译所有 Rust 依赖（约 3-5 分钟），之后增量编译只需几秒。

## 项目结构

```
TileCropper-Tauri/
├── src/                      # Vue 3 前端
│   ├── main.ts               # 入口
│   ├── App.vue               # 根布局
│   ├── types.ts              # 前后端共享类型（IPC 契约）
│   ├── api.ts                # Tauri IPC 封装
│   ├── stores/tile.ts        # Pinia 状态（替代全局变量）
│   ├── composables/
│   │   ├── useGeometry.ts    # 网格几何纯函数
│   │   ├── useCanvas.ts      # Canvas 绘制 + 裁切
│   │   └── useDrag.ts        # 拖拽/键盘交互
│   └── components/
│       ├── Toolbar.vue       # 工具栏
│       ├── TileCanvas.vue    # 画布
│       ├── PinDialog.vue     # 添加到开始菜单弹窗
│       └── StatusBar.vue     # 状态栏
├── src-tauri/                # Rust 后端
│   └── src/
│       ├── lib.rs            # Tauri 命令入口
│       ├── ico.rs            # ICO 二进制组装（纯 Rust）
│       ├── manifest.rs       # XML 生成（纯 Rust）
│       ├── powershell.rs     # PowerShell 执行器（证书/Appx 用）
│       ├── launcher.rs       # C# 启动器编译
│       ├── shortcut.rs       # 快捷方式（Win32 COM，无 PowerShell）
│       ├── cert.rs           # 自签名证书管理
│       ├── msix.rs           # sparse package 打包签名注册
│       ├── tile.rs           # 主流程编排
│       └── cleanup.rs        # 残留清理
├── package.json
├── vite.config.ts
├── tsconfig.json
└── src-tauri/
    ├── Cargo.toml            # Rust 依赖
    └── tauri.conf.json       # Tauri 配置
```

## 重构要点

### 性能优化

- **快捷方式创建**：原版每次调 `powershell.exe`（冷启动 ~400ms），本版用 `windows` crate 的 `IShellLinkW` COM 接口直接创建（~5ms），提速 80 倍
- **ICO 组装**：原版 JS Buffer 操作，本版 Rust 原生内存操作，零分配
- **XML 生成**：Rust `format!` 宏，编译期检查

### 安全改进

- Electron 的 `sandbox:false` → Tauri 的 `capabilities` 权限模型
- IPC 契约从隐式 → TypeScript 类型定义 + Rust serde 镜像
- CSP 严格化

### 仍需 PowerShell 的操作（不可避免）

以下操作没有好的 Rust 原生替代，仍用 PowerShell：
- `New-SelfSignedCertificate` —— 证书生成（一次性，缓存复用）
- `Import-Certificate` + UAC —— 证书信任（一次性，需管理员）
- `Get-AppxPackage` / `Add-AppxPackage` / `Remove-AppxPackage` —— MSIX 注册

但这些都不是高频操作，对日常使用体验影响小。

## License

MIT
