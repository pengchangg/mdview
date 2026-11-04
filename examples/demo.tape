# 录 README 用的演示动图：装好 vhs 后在仓库根跑 `vhs examples/demo.tape`
# 产物 examples/demo.gif（1 MB 上下；要塞进 README 首屏就再压一遍）。
# 键序：备一份兄弟文件 → 打开 → 滚动 → 搜 mermaid → 开大纲 → ] [ 换文件 → 退出。
Set Width 1100
Set Height 640
Set FontSize 15
Set FontFamily "JetBrainsMono Nerd Font"
Set Padding 24
Set FPS 30
Set Theme "Dark"

Output examples/demo.gif

# 多文件要至少两份才看得出状态栏的 2/2
Type "cp examples/demo.md examples/appendix.md && mdview examples/demo.md"
Enter
Sleep 1.5

# 滚动
Type "jjjj"
Sleep 0.3
Type "dd"
Sleep 0.8
Type "G"
Sleep 0.8
Type "g"
Sleep 0.3
Type "g"
Sleep 0.5

# 搜索：命中跨软换行也亮
Type "/mermaid"
Sleep 1
Type "nn"
Sleep 0.8
Escape
Sleep 0.3

# 大纲
Type "o"
Sleep 0.6
Type "jj"
Sleep 0.6
Enter
Sleep 0.8

# 多文件：状态栏计数 + ] [ 环绕
Type "]"
Sleep 0.8
Type "["
Sleep 0.8

Type "q"
Sleep 0.4
Type "rm examples/appendix.md"
Enter
Sleep 0.4
