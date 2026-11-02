# 第五阶段：原生资源加载与 Console 管理（2026-10-25）

状态更新：用户于 2026-10-25 明确确认修复版“验收通过”，第五阶段当前实现已通过用户验收。此前英文名两包的独立对照有效，但新放入的 44 个中文名包均未加载；已按用户“修复”授权修正文件系统 UTF-8 / ANSI 编码衔接错误，44 个包均能进入，加载动画与火狐 c00 角色替换均已实机确认。原验收失败记录保留；详见 YUZU_SKYLINE_UNICODE_FIX_2026-10-25.md。开发目录 D:/pcsmash-yuzu-dev-20261024，分支 yuzu-dev；未生成合并提交。

## 本次结果

采用上游 Skyline → ARCropolis → 原生资源包。Console 保留列表、ZIP/文件夹导入、别名、启用/停用、打开目录与软删除，撤出个人虚拟 data.arc 引擎及其 DLL/API/构建脚本和旧引擎测试。旧稳定 yuzu 与原工作目录的资料保留；实施前源码快照位于 build/skyline-console-20261025/before-sources.zip。

nnfs.c 恢复上游文件系统、FSA 文件系统分派、存档日志和账号关闭接口，仅按既有裁定衔接统一名字，并为无 Yuzu 构建保留明确名字来源，禁止回退 Windows 用户名。本项 nnfs 冲突已解决；剩余 README.md 和 runtime/nvn_vk_m4.c 两个冲突。

## 管理行为

- 默认 SD 是 Console 构建/包根目录的 sd，游戏启动显式传入相同 SSBU_SD。保存的自定义 SD 同时影响管理和启动；不同的单次启动覆盖会明确报错。
- 启用包在 sd/ultimate/mods；停用、回收、暂存分别放在扫描范围外的 sd/.console-mods。启停移动整包，别名存入 config/console-mods.json，不修改包内文件。
- 原生工作区/预设仍由 ARCropolis 自己控制，Console 不改写它们。本次实机使用插件默认生成的工作区，文件启停已真实生效；已有自定义预设仍需在原生工具确认选择。
- 导入拒绝重名、越界、链接、重解析目录及包含自身暂存区的源文件夹；分批移动失败会恢复；配置写入失败时回退位置。失败暂存保留以便恢复。
- 显式导入时备份旧 config/mods.json，仅迁移可用的别名/启用选择；不会自动搬动原目录或迁移旧优先级/换槽。
- 所有修改与游戏启动共用锁，重启路径也经过同一入口；运行中只允许查看/打开。旧换槽/优先级控件移除，旧字段请求明确拒绝。
- 管理页独立于游戏后端 DLL；插件存在仅显示“待验证”，不把文件存在当成挂钩或兼容已通过。含 plugin.nro 的包提示仍需转换和挂钩验证。

## 构建与上游流程

构建不再编译个人资源后端；--with-mods 明确报退役，--without-mods 保留为兼容参数，关闭原生插件使用 SSBU_SKYLINE=0。Console 遇到旧 SSBU_RESOURCE_MODS / SSBU_MOD_PROFILE / SSBU_MOD_CACHE / SSBU_MOD_REGION 会提示移除，避免静默忽略设置。

程序哈希绑定与网络 ABI 记录保留。resource_mods=false 表示旧后端已撤出，skyline_loader=true 仅记录上游入口能力。打包侧不再收集旧 DLL，拒绝旧后端构建，准备原生 SD 管理目录；本轮没有制作或发布可分发版本。

实际运行发现 tools/lifter/relift_sites.py 仍按旧拆分方式只链接 shim，生成的游戏库缺少当前宿主导入的 game_data_bind。已使它复用 build.py 的 compile_game_data，将相同的 SDK/数据对象放回生成库。修复后重新生成库有 1166 个对象、112890 个游戏函数和当前数据接口，实机正常启动。源码未改动 main 分支。

## 实机证据

诊断根目录：build/skyline-console-20261025。复用上次已验收的独立主游戏对象与私有着色器缓存；图形测试副本保持上游基线，未处理工作分支的图形冲突。

1. 官方 ARCropolis 4.1.1 原版曾被尝试，插件明确要求游戏 13.0.5；本机当前游戏是 13.0.4，故改用官方 4.0.8 原版。两份下载原归档与哈希保留，未修改原 NRO。早期试验留下的官方辅助资源同样保持原字节，详见 third-party-unchanged.json。
2. 使用上游 build_plugins.py 转换 4.0.8；采集实际 skyline_sites.log，再按上游 relift_sites.py 生成 50 处所需挂钩。最后日志没有 NOT ACTIVE。
3. 初次冷启动的观察窗口过短，起初只有 main 进入记录；延长观察并采集线程栈后确认初始化会正常返回，偏慢的工作是插件自身的配置/查表初始化。未通过修改原插件或删除资源绕过。
4. 导入原样 Kirby loading icon 和 mkwiiintro。两个包的原路径位于 D:/pcsmash/build/mod-adapt/gamebanana/extracted；选择、存档、ARC 配置、缓存均使用诊断目录。
5. 实机启用时出现粉色卡比加载图标、马车 Wii 开场视频；视频读取 19531461 字节，与原模组 webm 文件大小一致。通过 Console 的实际管理函数停用两包后，恢复原版加载图标和大乱斗开场视频，读取 108813844 字节；停用运行日志没有打开两个包的路径。
6. 再用相同管理函数重新启用，隔离验收目录中的实际游戏程序重新读取 19531461 字节，验证预设/缓存没有使重新启用失效。启用/停用运行均使用真实 GPU 和真实 Console 管道正常退出，退出码 0。
7. 完整包的每个文件哈希在导入、启停、软删除以及实机运行后核对，原包与安装副本一致。ARC 官方文件和额外官方辅助资源也做了字节核对。

可查看：mod-on-off-comparison.png、kirby-loading-enabled.png、mkwii-video-enabled.png；完整 sessions/*/game.log、trace.txt、result.json 和截图均保留。标题截图还显示 Smash 13.0.4 / ARCropolis 4.0.8。

## 检查结果及限制

- Rust Console：28 项常规检查通过；3 个依赖外部实物的忽略入口分别独立运行，包括真实子进程/管道重启、两个原包生命周期、为实机准备的真实启停。早期生命周期测试误假设 SD/回收区只有一个包，已修正为按包识别；未通过清理其他包让断言过关。
- Python：14 项网络构建启动、3 项旧后端退役/精确程序记录、5 项可选功能检查通过。
- 原生联机：33 项协议/设置/SDK 适配检查通过；真实 Console 管道和增量宿主构建通过。
- 有/无 Yuzu 宿主均成功链接，未保留已删除后端依赖；无 Yuzu 不会引用不存在的个人联机名字函数。
- 浏览器实际执行 Console 前端的空闲/运行中两种状态，确认按钮锁定、字面别名显示和换槽控件移除，均通过，截图 mods-manager.png / mods-running.png。
- F1 原生输入诊断在资源加载测试中完成，正常退出通过。此前后台运行的用户验收保留；本轮不重新扩大为真人跨机或手柄验收。
- 完整 SDK 自测整体仍有 1 项失败：erfcf 最大差 5 ulp。已与先前用户验收版对照，失败数据完全一致；有 Yuzu、无 Yuzu 均相同。文件系统挂载/目录/存档日志专项通过。此数学问题本轮未修改，也不把完整 SDK 自测写成全通过。
- 这是功能验收构建，主游戏采用既有未优化编译对象；未验收优化版性能、任意其他第三方插件、大型整合包或完整玩家对局。

## 本机验收入口

双击 build/skyline-console-20261025/review/StartReview.cmd。这是新的 Console 与本轮原生加载程序，独立配置和存档，两个原包已恢复为启用；默认关闭联机，便于观察资源。

可以先启动看卡比图标和马车 Wii 视频，退出后在模组页停用，再启动确认恢复原版。文件仍保存在诊断 SD，关闭不会删除。别名、导入、打开目录、回收可分别检查；修改需先退出游戏。

review 仅供本机验收，依赖原版外部 RomFS 与现有私有着色器缓存，不是可拷贝发布包。图形个人改造、延迟模块、Yuzu 回滚均不纳入本项验收。本项已通过用户验收；下一合并项仍先交用户裁定后实施；合入稳定 yuzu 前仍需用户给出版本号更新方案。

## 主要改动位置

运行时：runtime/nnfs.c、console_bridge.c、nnmisc.c；撤出 mod_resources.c/.h。

管理与界面：tools/console/src/mods.rs、game.rs、main.rs；tools/console/ui/app.js、index.html。

构建：tools/lifter/build.py、relift_sites.py；scripts/setup.py、package.py、release_update.py；撤出旧资源后端工具和旧引擎专项测试。完整退役列表在 retired-files.txt，旧源码有稳定分支和 before-sources.zip。

检查与说明：tests/test_mod_build_options.py、test_optional_features.py、test_console_network_bridge.py、test_network_host_build.py；BUILDING.md、MOD_RESOURCES.md、PACKAGING.md、INTERFACES.md 和本次裁定/清单/报告。阶段单独源码差异见 stage5-review.patch。
