# 图标

`logo.png`（1254×1254，带透明通道）是唯一的源文件，采用「微光几何」：黑色圆角底形与三条白色斜线。
使用黑白高对比，方便在输入源菜单和状态栏的小尺寸下辨识；`candidates/` 保留 Logo 探索方案。

`apps/macos/scripts/bundle.sh` 打包时用 `sips` + `iconutil` 生成 `Glimmer.icns`（应用图标），生成物不进仓库。

`menu-icon.pdf` 是输入法菜单（输入源列表、菜单栏）用的图标：16×16pt 纯黑矢量，三条斜线镂空。
系统只把「只有黑色 + 透明」的图当模板图，菜单高亮时才会反白、深色模式才会自动变色，所以不能直接用黑底白线的位图。
它由 `gen-menu-icon.py` 从 `logo.png` 量出的几何常数生成；改了 `logo.png` 要重新量圆角方块与斜线的像素坐标写进脚本，再跑一次：

```bash
python3 assets/icon/gen-menu-icon.py
```

Windows 共用 `apps/windows/tsf/resources/glimmer.ico`，由 TSF DLL 内嵌，Server 与 Settings 构建时也引用它。
更新 `logo.png` 后，用 ImageMagick 同步生成含 16、32、48、64、128、256px 的 ICO：

```bash
magick assets/icon/logo.png -define icon:auto-resize=256,128,64,48,32,16 apps/windows/tsf/resources/glimmer.ico
```

`windows/mode-{zh,en,caps}.svg` 是 Windows 任务栏语言栏上的「中 / 英 / A」图标源文件（纯 alpha，DLL 按任务栏主题填色）；
改了以后跑 `windows/render-mode-icons.sh` 重新导出 `apps/windows/tsf/resources/mode/*.alpha`（16 / 20 / 24 / 32 四档 DPI）。
