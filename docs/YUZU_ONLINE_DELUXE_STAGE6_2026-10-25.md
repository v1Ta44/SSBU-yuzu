# 第六阶段：原生 Online Deluxe 接入试验（2026-10-25）

状态：2026-10-26 用户明确停止 Online Deluxe 插件适配，恢复自有减延迟。此试验不再推进；原入口已转到 build/own-delay-20261026/review/StartReview.cmd。下文仅为历史试验记录，不代表当前使用方案；恢复结果见 YUZU_OWN_DELAY_RESTORE_2026-10-26.md。

## 用户裁定与测试入口

用户要求：先停用原有减延迟，尝试原生插件；同时核对 PC 移植版是否已经降低原版渲染时序造成的固有延迟。旧 YUZU_DELAY_STAGE6_PROPOSAL_2026-10-25.md 的 F8 接回方案被本裁定替代。

独立入口：build/online-deluxe-20261025/review/StartOnlineDeluxe.cmd。沿用现有 Console 与已验收宿主基线，仅叠加下述插件兼容修复。旧 build/skyline-console-20261025/review/StartReview.cmd 对应 EXE 的 SHA256 仍为 5723d80e72ea14c0a5d1caf84488f7b50ac2785568603bba43afb06fa76d34ea，没有替换。

测试入口使用独立 SD、配置、日志和存档，共享只读原版 RomFS / 已编译模块及既有私有着色器缓存。SD 位于 build/online-deluxe-20261025/sd-vanilla，附原样“游戏加载动画”包验证资源共存，没有复制或搬走用户的其他启用包。旧宿主延迟扩展编译标记 latency_slider=false；启动入口另设 SSBU_MODS=0，旧 F8 不启用。

## 插件版本与原包

本机游戏是 13.0.4。先检查并记录当前 v1.4.3 后，发布说明确认 v1.4.0 开始为 13.0.5 更新地址；smashline 1.6.7 也明确只适用 13.0.5，PIA 1.3.0 为 13.0.5 更新。新版最初实际加载后在分辨率挂钩处崩溃，日志保留；随后改用完整、相互匹配的 13.0.4 官方原包，不手改偏移或插件文件。

| 组件 | 本次试用版本 |
| --- | --- |
| ssbu-online-deluxe | v1.3.0，原包附带 ssbusync / nx_over |
| smashline | v1.6.6 |
| ssbu-pia-interface | v1.2.0，与 v1.3.0 发布说明匹配 |
| nro-hook | v0.4.0 |
| imgui-smash | v1.0.0 |
| ARCropolis | 沿用已验收 4.0.8 原包 |

全部 8 个 NRO 原文件未改动，哈希见 all-plugin-sha256.json、plugin-integrity.json、dependency-manifest-1304.json。Skyline 工具对派生的转译代码应用既有模拟器检查兼容和解释器恢复入口；这不修改原始 NRO。各发布 JSON、下载 ZIP 和版本源码快照均保存在诊断目录，未运行第三方自动安装脚本。

## 兼容层改动

1. runtime/nnnet_c4a.inc：补注册 nnsocketGetAddrInfo / nnsocketFreeAddrInfo，直接复用已有 BSD 布局实现；没有扩大 DNS 或报文行为。自测分派支持仅插件使用、主游戏未导入的注册名称，增加真实数值地址查询/释放检查。
2. runtime/nnhid.c：提供插件查询的独立键盘/鼠标未连接状态，宿主键盘仍按既有 Npad 路线使用；提供大小正确的 TouchScreenState<1>，避免误用主游戏的 <16> 接口写越界。新增缓冲区边界自测。没有新增插件键鼠操作支持的承诺。
3. runtime/skyline.c：LookupSymbol 可查到宿主已经实现的 SDK 导出，例如 nvnBootstrapLoader；复用主游戏既有导入地址，保留挂钩链。缺失名称仍返回 NotFound。此项修复了 ssbusync 在 vsync_history.rs 的 Option 解包错误退出。
4. tests/test_skyline_lookup.py：验证已加载模块优先、主游戏导出、主游戏 SDK 导入地址身份、仅插件使用的宿主导出与缺失符号失败。

以上是运行时兼容层；没有改第三方模型、配置、纹理、音频或 NRO。新增 TOML 是独立诊断 SD 下的可选测试配置，不是修改作者附带文件。修复前快照 before-compat-sources.zip；仅本次变更的补丁 compatibility-only.patch。

## 静态转译与实际运行

使用官方插件转译工具处理 8 个 NRO。除主游戏挂钩外，还为 lua2cpp_common 生成 smashline 所需挂钩；保留无挂钩的基线输入不变。原始日志有 5 条“把原指令写回原值”的记录，工具将其误计为源码变化；诊断派生列表只省略这些无变化写入，保留完整原日志及 unchanged-site-writes.json，没有跳过真实补丁检查或改动转译工具。

- 主游戏：44 个对象重新编译，1166 个对象链接；所有相应源文件先验证“无挂钩重新生成”与基线逐字相同。
- 公共角色模块：8 个对象重新编译，117 个对象链接，14312 个入口。
- 尚有 smashline 的两个 SDK 区域挂钩未激活（sdk+0x51e5dc、sdk+0x51ec68）。本次普通对战未因此失败，但不能据此声称任意 smashline 插件 / 异常路径已兼容。

## 已完成的验证

| 验证 | 真实结果 | 证据（相对 build/online-deluxe-20261025） |
| --- | --- | --- |
| 有 / 无 Yuzu 宿主 | 构建成功，旧延迟均禁用 | host-build.log、host-no-yuzu.log |
| Skyline 查找回归 | 所列优先级 / 地址身份 / 错误处理检查通过 | symbol-lookup-tests.log |
| 最终试用 EXE 完整 SDK 自测 | 新增网络别名、HID 边界检查通过；仍有既有 erfcf 5 ulp 一项失败，整体不记为全通过 | sdk-final.log、sdk-final-result.json |
| 默认渲染实际对战 | 8 个插件完成初始化；两名马里奥对局，120 个正式对战帧后正常退出，exit 0 | sessions/active1304-input-battle/result.json、game.log |
| 最终入口 + 显式 Vanilla + 资源包 | 原样加载动画资源共存；通过原生测试按键展开插件浮窗，屏幕显示 Player Info / FPS / FrameTime；120 个正式对战帧正常退出，exit 0 | sessions/render-vanilla-battle/result.json、game.log、shots/online-deluxe-battle.png |
| LessLag 单独对照 | 失败，exit 4；进入对战时 3 处渲染代码改写未激活并出现访问异常，不计为可用或降低延迟 | sessions/render-lesslag-battle/game.log、result.json |
| 原包完整性 / 原验收版保护 | 原始及各测试副本 NRO、加载动画文件哈希一致；旧验收 EXE 未改动 | final-validation.json、review-staging.json、all-plugin-sha256.json |

自动操作脚本也保留失败尝试：插件会增加 Npad 查询，原按轮询次数计时的手柄脚本按键持续时间不足；延长诊断按键后顺利选人进对战。该调整仅在 build 诊断脚本内，没有改动正常游戏的输入时序。

## 对“PC 已天然降低渲染延迟”的核对

结论：代码证据支持“已经减少一部分”，暂不能支持“原版固有延迟已全部消除”，也不能照搬作者在主机/模拟器环境中的减少 3～4 帧宣传值。

- PC 运行时 display_l1_pipe_default() 默认返回 1。nvn_gl.c 的 l1_pipe_after_submit 在本帧提交后直接安排呈现，并在这里控制游戏等待；最终实机日志确认 pipe ON。这已改写原主机等待到后续 present 的路径。
- 游戏内部仍有帧索引逻辑。例如原版 0x710386ab4c 仍将索引加 2，PC 提前送出纹理不等于游戏逻辑、渲染缓存、输入采样与屏幕扫描都没有延迟。
- 原插件的 Vanilla = 三缓冲 / TwoBehind / vsync；LessLag = 双缓冲 / OneBehind / 关闭 vsync 并启用渲染改写；LLUltra 进一步使用 Immediate。它通过 ssbusync 动态切换指令和执行顺序，不能当作 PC swapchain 的同名设置直接套用。
- LessLag 实测触发了 0x710386fcec（间接调用→立即返回值）、0x710374c054（条件跳转→无条件跳转）、0x710374b554（调用→NOP）等动态写入。本次静态游戏库只编入启动状态，运行中这些指令切换未生效；改变部分寄存器/缓冲配置却继续执行旧指令可能造成不一致。失败证据证明当前缺少此项兼容，不能证明 LessLag 算法在 PC 没有效果。
- 本次 L1 统计是宿主轮询到呈现的诊断量。插件增加了轮询，不能拿它与旧程序的数字直接相减来宣称端到端收益；没有高速相机/输入光电测试，也没有真实对端网络对照。

因此试用入口的可选诊断配置将菜单、离线与在线渲染全部设为 Vanilla，关闭 overclocker；原样 NRO 保留。LessLag / LLUltra / FPS Boost 的完整动态指令兼容与渲染时序比较需要单独方案，先交用户裁定，不借本次测试直接改未裁定的图形合并冲突。

## 用户试用与未完成范围

从独立 StartOnlineDeluxe.cmd 打开 Console，选择 Yuzu 并按既有流程连接，进入游戏的本地无线会话。普通手柄 ZL + ZR + 十字键下展开浮窗；全信息模式中选 NetLatency 并用十字键左右调整，或在联机选人画面按插件原生方式调整等待帧数。保持 NetProfile / 渲染为 Vanilla，不切换 LessLag / LLUltra 或 FPS Boost。

用同一对端、同一网络和场景对照 Auto 与手动值，结合掉帧/同步情况判断。当前离线浮窗的网络栏显示 --- 是预期，不能当成设置已经应用。尚未记录实际多人会话中的 SET ONLINE LATENCY 与有效值，也未证明比旧减延迟算法更快。此项等待真实联机验证，未标记验收通过。


## Console 启动无窗口的反馈核对（历史，2026-10-26 已调整）

用户报告从 Console 启动后没有窗口。检查独立试用入口的三份用户会话（20261025-155332、155459、170118）确认：当前连接模式为 yuzu，均只记录到 console bridge 的入房前等待，没有收到 start，没有进入游戏映射/插件初始化，也没有插件异常记录。170118 会话还明确记录 Console 先断开连接，因此游戏不启动。其他会话的退出码 1 本身不能用来判定插件崩溃。

代码对应：game.rs 在 Yuzu 模式不立即发送 start；main.rs 的 net_poll 在房间状态达到 3 后才发送 start；runtime/boot.c 在创建窗口前等待 console_bridge_start。因此实际顺序是“开始游戏（先等待）→ Console 联机页连接服务器 → 入房成功后窗口出现 → 游戏内本地无线”。已补充独立试用说明；没有擅自绕过已裁定的 Yuzu 入房等待，没有更改用户保存的联机方式。机器可读证据位于 build/online-deluxe-20261025/console-start-investigation/results.json。


## 2026-10-26 用户批准后的启动调整

用户明确要求“调整它”后，Console 已改为所有连接方式均在控制通道连上后发送 start，不再等待 Yuzu 服务器入房，net_poll 也不再负责启动。游戏先创建窗口、加载插件；连接服务器与进入本地无线仍按正常联机流程进行。联机底层的未连接检查保留，没有伪造房间状态。

通过 Console 实际 game::start 路径，在隔离配置、存档和日志下使用当前 Online Deluxe 原样插件，保持 Yuzu 模式、零次 join、画面出现前零次 UI poll：检测到可见 1280×720 游戏窗口，截图显示加载画面与原样 mod 动画，状态为 started=true / state=0；正常 shutdown，退出码 0。加载期间出现短暂 watchdog 警告并自行恢复，Console 会话结果保留 warning。常规测试、真实管道启动/重启、连接前拒绝及入房后初始化回归通过。独立试用 Console 与试用说明已更新；上节启动等待的描述仅作为故障历史保留。详见 YUZU_CONSOLE_START_2026-10-26.md。


## 2026-10-26 停止试验

用户实测进入本地无线退出。现场日志显示 Yuzu 服务器已分配地址、LDN Initialize 成功，随后 ssbusync 调用缺失的 nn::diag::GetBacktrace 导致退出。实际 NRO 转译代码表明该调用用于通过精确返回地址识别联机模式；不是仅供报错日志使用。尚未部署的该接口修复已按用户停止适配的裁定撤回，调查/试构建保留于 build/local-wireless-crash-20261026，并标记 ABANDONED。不能记为此插件联机兼容通过。

新的使用入口只加载原样 ARCropolis 和自有宿主减延迟，停止加载 Online Deluxe / ssbusync 及试验所用其他配套插件。历史 NRO、资源、日志均保留；未改第三方包。
