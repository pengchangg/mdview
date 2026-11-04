# 录 README 首屏那张演示动图。
# 前置：cargo build --release，并把 target/release 放进 PATH（VHS 起的是真 shell，
#       它只认 PATH 里的 mdview）；还要 ttyd / tmux / ffmpeg 与一个中日韩字体，
#       否则 demo.md 的中文会变成豆腐块。
# 产物 examples/demo.gif，控制在 2 MB 上下（README 首屏用）。
Set Width 1100
Set Height 620
Set FontSize 15
Set FontFamily "JetBrainsMono Nerd Font"
Set Padding 20
Set Framerate 20
Set Theme "OneHalfDark"

Output "examples/demo.gif"

# 多文件要至少两份才看得出状态栏的 2/2
Type "cp examples/demo.md examples/appendix.md && mdview examples/demo.md"
Enter
Sleep 1.6

# 滚动：jj、半页、跳尾、再回顶
Type "jjjj"
Sleep 0.3
Type "dd"
Sleep 0.9
Type "G"
Sleep 0.9
Type "gg"
Sleep 0.5

# 搜索：/ 边打边亮，Enter 保留查询，再用 n 跳下一个命中
Type "/mermaid"
Sleep 1.0
Enter
Sleep 0.4
Type "nn"
Sleep 0.9
Escape
Sleep 0.3

# 大纲：o 开、j 选、Enter 跳
Type "o"
Sleep 0.6
Type "jj"
Sleep 0.6
Enter
Sleep 0.9

# 同目录翻页：状态栏的 1/2 ↔ 2/2
Type "]"
Sleep 0.9
Type "["
Sleep 0.9

Type "q"
Sleep 0.4
Type "rm examples/appendix.md"
Enter
Sleep 0.4
