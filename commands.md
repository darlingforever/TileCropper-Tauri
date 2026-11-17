# TileCropper-Tauri 常用命令速查

> 所有命令在 `D:\code\TileCropper-Tauri` 目录下执行（PowerShell 或终端）

## 日常开发

```powershell
# 启动开发模式（前端 HMR + Rust 热重载）
cd D:\code\TileCropper-Tauri; npm run tauri dev

# 仅启动前端（Vite 开发服务器，浏览器里调 UI）
npm run dev

# Rust 代码检查（不生成二进制，最快验证编译）
cd src-tauri; cargo check; cd ..

# 前端类型检查
npx vue-tsc --noEmit

# 前端生产构建（输出到 dist/）
npm run build
```

## 打包发布

```powershell
# 生成 release 安装包（NSIS .exe 安装程序）
npm run tauri build

# 只编译 release 二进制（不打包安装程序，更快）
cd src-tauri; cargo build --release; cd ..

# 产物位置：
#   安装包：src-tauri/target/release/bundle/nsis/*.exe
#   可执行文件：src-tauri/target/release/tilecropper.exe
```

## 清理缓存

```powershell
# 清掉 Rust 编译缓存（释放 ~6.8GB，不影响功能，下次编译需 ~12 秒重建）
cd src-tauri; cargo clean; cd ..

# 只清 release 产物（保留 debug 缓存，省得 dev 重新编译）
cd src-tauri; cargo clean --release; cd ..

# 清掉前端构建产物（dist/，几十 KB 无所谓）
rm -rf dist

# 清掉前端依赖重装（解决 npm 相关疑难杂症）
rm -rf node_modules package-lock.json; npm install
```

## 图标管理

```powershell
# 从源图生成各尺寸图标（首次搭建或换图标时用）
# 准备一张 1024x1024 的 PNG 放到 src-tauri/icons/icon.png
npx tauri icon src-tauri/icons/icon.png
```

## Rust 工具链

```powershell
# 查看 Rust 版本
rustc --version

# 更新 Rust 工具链
rustup update

# Rust 代码格式化
cd src-tauri; cargo fmt; cd ..

# Rust 代码检查（更严格的 lint）
cd src-tauri; cargo clippy; cd ..
```

## 调试技巧

```powershell
# 开发模式下看 Rust 后端日志（println! 输出在终端里）
# tauri dev 启动的终端会直接显示 Rust 的 println! 和 eprintln!

# 前端调试：在 tauri dev 的窗口里按 F12 打开 DevTools
# 或在代码里加 console.log，输出在 DevTools Console

# 查看 release exe 大小
ls -lh src-tauri/target/release/tilecropper.exe

# 查看 target 各部分占用
du -sh src-tauri/target/debug/deps src-tauri/target/debug/build
```

## 常见问题

```powershell
# PowerShell 禁止运行脚本 → 解除限制
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser -Force

# npm 命令找不到 → 确认 Node.js 在 PATH
node --version

# cargo 命令找不到 → 确认 Rust 在 PATH
# Rust 默认装在 %USERPROFILE%\.cargo\bin
$env:PATH += ";$env:USERPROFILE\.cargo\bin"
```

## 目录结构速览

```
TileCropper-Tauri/
├── src/                    # 前端源码（Vue 3 + TS）
│   ├── components/         #   Vue 组件
│   ├── composables/        #   Canvas/拖拽/几何逻辑
│   ├── stores/             #   Pinia 状态管理
│   ├── api.ts              #   Tauri IPC 封装
│   ├── types.ts            #   前后端共享类型
│   └── styles.css          #   全局样式
├── src-tauri/              # Rust 后端
│   ├── src/                #   Rust 源码（9 个模块）
│   ├── icons/              #   应用图标
│   ├── Cargo.toml          #   Rust 依赖声明
│   ├── tauri.conf.json     #   Tauri 配置（窗口/CSP/打包）
│   └── target/             #   编译缓存（可删，~6.8GB）
├── dist/                   # 前端构建产物（可删）
├── node_modules/           # 前端依赖（可删重装）
├── package.json            # 前端依赖 + 脚本
├── vite.config.ts          # Vite 构建配置
└── commands.md             # ← 本文件
```
```

</content>
