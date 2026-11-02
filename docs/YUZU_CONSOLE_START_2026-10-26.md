# Console 在 Yuzu 模式下直接启动游戏（2026-10-26）

## 用户裁定与结果

用户报告 Console 启动后无游戏窗口，核对确认旧流程在创建窗口前等待 Yuzu 入房。用户询问该机制后明确授权“调整它”。本次将点击“开始游戏”的含义恢复为直接运行游戏；使用本地无线之前再连接服务器。

实施位置为当前整合分支 yuzu-dev，工作目录 D:/pcsmash-yuzu-dev-20261024。未提交合并、未合入稳定 yuzu、未发布；README.md 与 runtime/nvn_vk_m4.c 两项未解决冲突保持原样。

## 改动

- tools/console/src/game.rs：抽出控制通道握手/启动函数，在所有连接方式下主动发送 start。保留原生启动握手，以支持进程控制和现有诊断。
- tools/console/src/main.rs：net_poll 只读取状态并处理既有重启请求，去掉 Yuzu 入房成功才放行启动的副作用。
- tools/console/ui/app.js、index.html、connection.js：说明游戏先开窗口，在本地无线之前连接服务器；UI 的 gate 字段始终为 false，保留字段兼容性。
- tests：新增未入房且无 UI poll 的真实管道回归、真实游戏启动测试，以及未连接时 LDN/socket 拒绝、入房后原进程初始化成功的回归。Console 管道夹具补齐空的插件 SDK 导出查询模拟接口，以匹配前一阶段网络兼容层依赖。

网络协议、原生 LDN/socket 连接检查、游戏宿主 EXE 与第三方插件没有为本次启动调整修改。

## 验证

证据根目录：build/console-start-20261026。

| 检查 | 结果 | 证据 |
| --- | --- | --- |
| Console 常规单元测试 | 28 通过；外部环境测试按要求跳过 | rust-tests.log |
| UI 连接方式/进程状态 | 通过；Yuzu gate=false | ui-tests.log |
| 真实原生 Console 管道 | 三种方式与正常退出通过 | bridge-tests.log |
| Rust 生产握手函数 + 真实管道 | 未入房/无 UI poll 启动及现有重启两项通过 | real-pipe-tests.log |
| Yuzu 原生后端 | 未连接拒绝使用 LDN/socket；同进程入房后成功初始化，其他既有网络回归通过 | backend-tests.log |
| 真实游戏与 GPU | 直接调用 Console 生产 game::start，Yuzu 模式，零 join、渲染前零 UI poll；可见 1280×720 窗口，加载画面截图；started=true / state=0；退出码 0 | real-game-test.log、visible-windows.json、real-console/real-console-result.json |
| Release Console | 构建成功，并替换独立 Online Deluxe 试用目录的 Console | release-build.log、deployment.json |

真实游戏验证使用隔离的配置、存档和日志，复用已有宿主、完整原样插件和诊断 Vanilla 配置。可见窗口记录包含测试编译时间，因此不作为启动性能测量。截图为加载画面（frame_120.bmp），不能据此宣称已实测加入真实多人会话。加载期间 watchdog 报告过短暂停顿并自行恢复，会话最终 result 为 warning；退出码为 0，无 dump。

## 试用入口

D:/pcsmash-yuzu-dev-20261024/build/online-deluxe-20261025/review/StartOnlineDeluxe.cmd

选择 Yuzu 后点击“开始游戏”即可打开游戏，无需先连接服务器。需要联机时在 Console 联机页连接服务器，连接成功后再进入游戏的本地无线。服务器不可达不会再阻塞游戏窗口。

本次不改变第六阶段的其他限制：原有 F8 延迟扩展关闭，渲染保持 Vanilla；LessLag 等动态渲染功能仍待独立兼容方案，真实对端的网络减延迟效果和用户实机验收尚未完成。
