# 恢复自有减延迟（2026-10-26）

> 最新验收：用户明确确认“新面板和减延迟都实机验收通过”。本文下方当时的待验收描述保留为历史记录，现已关闭相应待办；不将实机使用验收写成未经测量的端到端延迟毫秒数据。

用户裁定：停止 Online Deluxe 插件适配，使用原个人版的减延迟。开发在 D:/pcsmash-yuzu-dev-20261024、yuzu-dev 进行；本记录不表示用户实机验收、合入稳定 yuzu 或发布。

## 当前使用

入口：build/own-delay-20261026/review/StartReview.cmd。此前 build/online-deluxe-20261025/review/StartOnlineDeluxe.cmd 已改为转入新入口，原文件备份于 build/own-delay-20261026/previous-entry。旧已验收 build/skyline-console-20261025/review/StartReview.cmd 的程序未改。

游戏前台 F8 展开 / 收起原有面板，支持 Auto 与手动 0～25 帧。仅当前实际选择 Yuzu、已建立本地无线且有 2～8 个节点时，沿已验收函数 0x71016ccae0 在原计算后调整对象 +65 字节；不改变游戏速度和渲染时序。恢复自动后停止后续覆盖，干净对照需重进会话。复制原个人版 config/online-delay.ini 的 3 帧选择到独立验证目录，下次操作在新目录保存。

资源仍采用上游 Skyline → 原样 ARCropolis 4.0.8；本地 Console 管理资源包。新 SD、存档、配置、日志独立，第三方 NRO、配置、模型、贴图、音频均未修改。Online Deluxe / ssbusync / 该试验的其余插件不在当前注册表中。

Console 保留直接开窗和运行中入房，后台焦点修复保留；runtime/nvn_vk_m4.c 继续等于 main，SHA256 为 6442802c4845d1b7aeede4ff9b031464d3c12018596037ec6b54f6a508d9ef04。不适配 Yuzu 回滚。

## 代码范围

- fighter_sdk/hook_host.c：重新启用既有独立加载开关；延迟 DLL 及默认配置路径使用 Unicode Windows API，长度检查，保留显式配置与缺失扩展处理。
- runtime/yuzu_net.c：ssbu_net_ldn_snapshot 在非 Yuzu 模式返回不活动的上下文，避免旧状态驱动手动减延迟；既有 ABI 不变。
- tests/network_protocol.c：增加非 Yuzu 模式的陈旧会话隔离检查；补齐测试夹具对已存在宿主 SDK 查找接口的声明实现。
- tests/net_delay_frontend_bridge.cpp：查找属于本次测试窗口的面板，避免误抓用户正在运行的其他个人版面板。实际面板产品代码没有为此改变。
- tests/test_delay_loader.py：真实加载器在中文目录、默认 / 显式配置、关闭功能、扩展缺失条件下的测试。
- 原 extensions/net_delay 算法、适配函数和前端产品代码复用，没有改写。

未部署的 ssbusync GetBacktrace 适配从 runtime/nnmisc.c 撤回；现场调查和试构建仅保留在 build/local-wireless-crash-20261026，标记停止。

## 验证结果

| 项目 | 结果 |
| --- | --- |
| 有 Yuzu + 减延迟宿主、无 Yuzu + 无减延迟宿主 | 均成功构建，能力标记和产物哈希已核对 |
| 原游戏等待计算器对照 | 2125 组通过，1887 次手动写入，寄存器 / 标志 / 栈保持，仅目标字节变化；无对端、掉线、换会话等门槛通过 |
| 独立核心 | 1088 组判定、16000 次并发操作通过 |
| 设置 | 27 次保存恢复，实际适配器保存、启动参数、非法值与指纹拒绝通过 |
| 协议 | 33 组配置 / SDK / 本机 ENet 对照通过，含新加非 Yuzu 陈旧状态隔离 |
| Unicode 延迟加载 | 中文 DLL / 配置路径、显式配置、禁用构建、缺失扩展通过 |
| 面板 | 真实 WebView2 页面 → 私有管道 → 核心，显隐三轮、隐藏启动、绑定窗口、失去可见性与关闭通过 |
| 完整游戏第一次导航 | 正常启动与退出，ARC / 延迟与资源加载成功；旧方向保持时间过长使选择越出角色区，停在规则页，未记为对战通过 |
| 完整游戏缩短诊断按键 | 同一正式宿主、不改产品逻辑，成功对战 120 帧，退出 0；仅加载一个原生插件 ARC，原等待适配指纹通过，面板实际连接并完成页面初始化 |

证据在 build/own-delay-20261026/validation.json、host-*-build.log、sessions/production-battle、sessions/short-holds-battle，以及 build/latency-slider/native-test、module-test 和 build/network-protocol-20261024。实际资源包读取可见游戏日志 fslog，原包复制后逐文件哈希复核。

验收更新（2026-10-26）：用户确认实机对端联机测试已通过，真实跨机器联机已确认；手动等待值应用及主观手感尚未确认。用户另报告 F8 无法呼出面板，现已修正隐藏启动标志并补充全屏层级与实际游戏检查，见 YUZU_F8_FULLSCREEN_FIX_2026-10-26.md；物理按键仍待用户复测。本机假服务器、离线对战和计算器对照不能替代真实跨机联网，也未测得端到端按键延迟数据。

仍只剩 README.md 合并冲突；未创建合并提交、未变更 main / 稳定 yuzu、未推送或发布。
