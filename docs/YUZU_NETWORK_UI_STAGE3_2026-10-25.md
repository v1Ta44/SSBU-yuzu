# Yuzu 联机第三阶段：F1 / Console 设置、状态与重启（2026-10-25）

后续验收记录（2026-10-25）：用户已明确确认当前阶段独立功能测试版“实机验收通过”。具体测试范围及原始结果见 YUZU_LIVE_VALIDATION_2026-10-25.md；下文阶段完成时的待验收描述为历史记录。

状态：用户已批准的本阶段实现与局部验证完成，等待用户验收。开发分支仍为 yuzu-dev；yuzu 合并尚未提交，剩余 7 个其他领域冲突。完整游戏、真人联机尚未验收。

## 本阶段结果

1. F1 与 Console 共用 graphics.ini 的 online_mode，支持 Yuzu、关闭联机、上游大厅、局域网以及创建/加入房间码。Console 开始页保存连接方式，联机页提供相应设置；F1 打开时重新读取保存值。
2. “当前连接”取自运行中进程的真实后端，“下次启动”取自保存设置。运行中切换设置不会改变当前连接标签；实际状态、人数和延迟通过既有统计接口显示。
3. Yuzu 使用原有服务器房间控制；无论从哪个启动入口进入，Console 的 Yuzu 会话均在成功加入服务器房间后放行游戏。上游和关闭联机直接放行；冒烟诊断强制关闭联机。离线游玩提示引导选择关闭联机。
4. Yuzu 回滚适配仍不开展。选择 Yuzu 时禁用回滚控件和本次进程的联网回滚参数，保留用户保存的上游回滚设置；切回上游后可继续使用。
5. Console 与 Console 启动的 F1 重启共用流程：请求旧游戏退出，等旧进程结束并关闭旧管道后，重新读取设置，启动新进程与新通信管道。重复请求合并处理。退出失败不擅自开启第二个游戏；没有活动 Console 管道时 F1 给出手动重启提示。
6. 重启后不沿用旧房间身份；Yuzu 要重新加入服务器房间。旧日志读取器按会话隔离，避免写入新进程的日志区。
7. 启动判断和运行时使用同一个已解析配置路径，保留显式环境参数优先级。读取/修改配置保留未知字段与注释，输入校验失败时不写入文件。
8. Console 根据 state.json 选择 13.0.4 / 13.0.5 文件目录和房间准入所用版本信息；专用 Yuzu 启动器同时传递版本信息。该修正不代表游戏及其他个人扩展已通过 13.0.5 实机验收。

## 改动范围

- runtime/menu.cpp：F1 连接选项、重读设置、真实后端标签、回滚可用性及 Console 重启协作。
- runtime/console_bridge.c：真实连接统计、重启请求、退出命令及准入版本信息。
- tools/console/src/network.rs（新增）：共享连接设置、参数解析、程序能力验证与启动策略。
- tools/console/src/game.rs、main.rs：按真实后端放行、状态、版本选择、旧进程退出后的重新启动与会话隔离。
- tools/console/ui/connection.js（新增）、app.js、index.html：共用连接选择、当前/下次状态、对应控件与重启交互。
- scripts/yuzu_play.py：本阶段仅补充 SSBU_GAME_VERSION；其余启动整合已在第二阶段完成。
- tests/console_network_bridge.c、test_console_network_bridge.py（新增）：原生进程与真实管道夹具和复现入口。
- tests/console_connection_view.cjs、test_console_ui_preview.py、fixtures/console_preview.js（新增）：连接视图判断和隔离浏览器预览。预览替身随仓库保存，不依赖其他工作目录中的本地文件。
- tests/network_protocol.c、test_network_build_launch.py：共享原生夹具入口、统计替身及专用启动器版本断言。

## 实际验证

| 项目 | 本轮结果 | 边界 |
| --- | --- | --- |
| Console Rust 常规测试 | 20 通过，0 失败，3 默认忽略 | 其中 1 项重启测试另行显式执行通过；另外 2 项原有 mod 安装测试不在本阶段验收范围 |
| Rust 真实管道重启测试 | 显式执行 1 项通过 | 覆盖 Yuzu / 上游 / 关闭联机，等待旧子进程、重新建管道、重复请求及诊断限制；使用隔离原生子进程 |
| 原生 Console 桥 | 3 种模式各启动两次，全部通过 | 真实命名管道、模式信息、放行、错误模式加入拒绝、重启标志、退出响应及退出码；不连接真人服务器 |
| F1 编译 | SSBU_ENABLE_YUZU=0 和 1 均通过 | 使用仓库指定的 ImGui 1.91.9b；只编译菜单对象，未在完整游戏中运行 F1 |
| 浏览器中的实际 Console 前端 | 6 种场景通过并留图 | Yuzu / 上游 / 关闭 / 已保存待重启，以及开始页；后台为隔离替身 |
| 前端语法与连接判断 | 通过 | 当前进程优先、Yuzu 等待、诊断及重启条件 |
| 构建启动及既有开关回归 | 26 通过，0 失败、0 跳过 | 新构建启动 14 + mod 开关 5 + 可选功能 5 + 诊断启动 2 |
| 原生网络核心 | 33 项通过 | 模式、配置路径、SDK 接口与收发、回滚隔离 |
| 隔离宿主实际编译链接与运行 | 通过；重复构建重编译数 0 | 使用真实依赖与宿主构建流程，游戏加载器由网络夹具替代；完整游戏未构建 |

证据保存在 build/network-ui-20261024/，包括 rust-tests.log、rust-restart-integration.log、bridge-build.log、bridge-run.log、menu-0/1.log 和 .obj、ui-preview-results.json、六张预览图及各项回归日志。目录沿用开始工作时的日期；本报告记录的是 2026-10-25 完成的最终复查。

原生核心和宿主证据分别见 build/network-protocol-20261024/results.json 与 build/network-build-20261024/host-build-results.json。

stage3-review.patch 提供本阶段已有文件相对阶段开始快照的差异及本阶段新增文件；scripts/yuzu_play.py 和 test_network_build_launch.py 的单行版本补充另见本报告改动范围，避免将其第二阶段改动误标为本阶段。

## 复现主要检查

```powershell
python tests/test_console_network_bridge.py --clang <clang.exe> --deps <依赖缓存>
node tests/console_connection_view.cjs
python tests/test_console_ui_preview.py
cargo test --manifest-path tools/console/Cargo.toml --offline --bin ssbu-console -- --test-threads=1
# 指定上一步生成的 console_network_bridge.exe 后，显式运行真实子进程测试：
$env:SSBU_TEST_BRIDGE_CHILD='<工作目录>\build\network-ui-20261024\console_network_bridge.exe'
cargo test --manifest-path tools/console/Cargo.toml --offline --bin ssbu-console restart_tests -- --ignored --test-threads=1
```

Rust 构建还需已有的更新器可执行文件；可用 SSBU_UPDATER_EXE 指定。浏览器预览需要 Windows Edge。编译器、依赖和编译缓存允许通过现有环境变量指定，不覆盖稳定游戏程序或配置。

## 分支与验收边界

main 仍为 fea28c18b785372b5024ece0a2ea52e860b1fa04；稳定 yuzu 仍为 b360a183a323d860892cf3cc92f5b89f54a437c1。原 D:/pcsmash 工作目录中的 AGENTS.md 修改和未跟踪文档保留。本阶段未提交合并、未发布、未推送、未更改第三方 mod。

剩余冲突：README.md、runtime/nnfs.c、runtime/nnhid.c、runtime/nnmisc.c、runtime/nvn_vk_m4.c、tests/oe_focus_test.c、tests/test_oe_focus.py。它们会阻止完整整合构建，不能据本阶段局部测试宣称游戏可发布。

## 下一项提案，尚未实施

建议处理“焦点 / 输入 / 退出”四个冲突文件：runtime/nnmisc.c、runtime/nnhid.c、tests/oe_focus_test.c、tests/test_oe_focus.py。

- 保留个人版默认后台继续运行；显式开启焦点通知时保留上游修复后的窗口焦点处理，并保留 Linux/macOS 条件分支。
- 保留上游窗口关闭 / 退出逻辑、GameCube 扳机数据和 F1 手柄控制；将个人输入诊断记录放在完整手柄数据写入之后，避免二者互相覆盖。
- 合并默认、显式开启 / 关闭、启动焦点等既有测试，进行局部编译与针对性检查。

需经用户裁定后才开始此下一项；文件系统、图形、文档和稳定版合入继续另行处理。
