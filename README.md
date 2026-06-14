# DesktopOrganizer

桌面图标自动整理工具 — 零持久性、零后台进程、事件驱动。

## 功能

- **智能分类** — 180+ 关键词类别匹配，`.exe` 按功能细分
- **多显示器** — 每屏独立布局配置
- **九宫格区域** — 默认 3×3 网格，支持子区域嵌套
- **形状排列引擎** — 13 种排列形状
- **撤销/重做** — 最多 50 层历史
- **多语言** — 中文、English、한국어、日本語、हिन्दी
- **托盘运行** — 最小化到系统托盘，事件驱动自动整理（零 CPU 空闲）
- **拖拽缩放** — 区域大小和位置自由调整

## 技术栈

- **Rust** + **eframe/egui** 0.31 + **windows-rs** 0.58
- Win32 Shell API 直接操作桌面 ListView
- `ReadDirectoryChangesW` 事件驱动桌面监控
- `CreateMutexW` 单实例检查

## 构建

```bash
cargo build --release           # 编译
tools\inject_icon.bat            # 注入图标（需要 rcedit）
```

产物：`target/release/desktop-organizer.exe`

## 使用

1. 运行 `desktop-organizer.exe`
2. 右键托盘图标 → 选择语言
3. 关闭窗口时选择「最小化到托盘」或「退出」
4. 桌面文件变动自动触发整理

## 许可

MIT
