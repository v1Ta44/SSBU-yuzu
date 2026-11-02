# F8 隐藏启动与全屏面板修正（2026-10-26）

验收更新：2026-10-26 用户确认“除去全屏状态下仍然看不到延迟窗口，其他验收通过”。后台失焦、房间等待与窗口模式 F8 通过；全屏面板仍未通过，原可见标志与置顶层级检查不等于实际画面验收。

用户报告 F8 无法呼出自有减延迟窗口，并要求全屏显示一起修复。

## 原因和修改

当前 StartReview.cmd / 实机日志传入 SSBU_NET_DELAY_UI=hidden，适配器据此以 --hidden 启动前端。面板却在每次显示判断中持续要求 !app.hidden。因此展开状态虽会切换，原生 HWND 和 WebView 仍然隐藏；窗口和全屏都会受影响。

此前前端检查使用 --integration-test，它会将 hidden 清零，绕过了真实启动参数。本次新增不带该覆盖选项的 --hidden 回归；旧已部署程序在展开后的可见性断言稳定失败。

extensions/net_delay/ui/win32_panel.cpp 将该标志明确为 start_hidden，显示只取决于展开状态及游戏是否处于前台。收起时保留返回游戏焦点的行为。游戏失焦 / 最小化仍会隐藏面板，不留下点击区域。

上游 runtime/display.c / gfx_options.c 的 fullscreen 是无边框置顶窗口，没有切换显示模式。本次明确让面板跟随游戏的置顶层级，回到窗口模式后恢复普通层级；不更改游戏全屏实现。

## 检查和边界

- 真实 --hidden 参数、无 --integration-test 覆盖：旧版展开失败；修正版连续三轮展开 / 收起均使 HWND 与 WebView 一起显示 / 隐藏。
- 以离屏窗口模拟上游无边框置顶全屏与窗口切换，检查面板的层级和位置；原管道夹具继续覆盖移动、改变大小、隐藏恢复、真实网页选值通信及游戏窗口关闭。
- 1088 组后端决策、16000 次并发选择 / 查询、27 轮设置保存恢复通过，CSS / 字体复用、缺失页面报错及页面预览检查通过。
- 正式游戏检查使用真实游戏窗口，以安全窗口消息切换“窗口 → 全屏 → 窗口”，通过同一面板展开处理器检查可见性：三个阶段 HWND 与 WebView 均可见，面板与游戏置顶层级一致；每次收起后均不可见。没有向用户桌面注入 F8 或其他键盘操作，因此用户实际物理按键仍需复测。

证据目录 build/f8-fix-20261026：window-modes-results.json、before-window-modes.log、fixed-window-modes.log、host-final.log、production-run.log。完整游戏记录位于 build/background-room-fix-20261026/sessions/f8-final-battle。

已备份并更新同一试用入口。正式游戏完成 120 帧对战并正常退出。最终宿主 SHA256：a784406793053c8ce6408684a315dae0702c56d505d23d37ff86a6d172c44551；面板 SHA256：fb54bb480d9b2829f73cefcae6744fcbb18eff8d7671f836c432124142bb32d8。程序能力记录与扩展指纹一致，完整部署校验见 deployment-final.json。沿用 build/own-delay-20261026/review/StartReview.cmd；初始版面板备份在 trial-before，最终覆盖前的成套备份在 trial-before-final。减延迟算法、保存的帧数、游戏 DLL、ARC 和第三方包保持原样。

影片与房间失焦修正仍一起包含在宿主中。实机对端联机已获用户确认；真实切窗和物理 F8 / 全屏面板操作尚待本次用户验收。没有合入稳定 yuzu、更新版本或发布。
