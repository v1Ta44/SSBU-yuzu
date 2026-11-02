# 构建与 Console 收尾修正（2026-10-26）

用户已批准 YUZU_BUILD_CONSOLE_STAGE8_PROPOSAL_2026-10-26.md 的三项修正。本轮在 D:/pcsmash-yuzu-dev-20261024 的 yuzu-dev 实施，当前试用入口已更新。未合入稳定 yuzu、未更新版本、未发布；README.md 冲突保持待处理。

## 完成的修正

1. runtime/gfx_options.c：F1 写入时更新所有有效同名设置行，使第一处 / 最后一处优先的读取者都看到同一新值。保留其他行的原始字节和顺序、注释与未知字段；缺失项追加。多次替换时安全增长输出缓冲，检查长度溢出，匹配时不越过短行；读取 / 分配失败时不覆盖原文件，继续采用临时文件替换方式保存。不修改原有读取优先级。
2. scripts/setup.py：共用 check_runtime_features 在读取 / 解包运行时包前拒绝源构建专用扩展；step_link 对已缓存的预编译运行时也再次检查。Yuzu 或自有减延迟需要源码宿主时给出明确错误及“去掉 --runtime / 显式关闭扩展”的操作提示。纯上游预编译安装和默认开关保持原行为。BUILDING.md 补充参数组合说明。
3. tests/network_protocol.c、tests/console_network_bridge.c：只保留一份共用 extra_import_lookup；不存在的插件专用导出返回 NULL。移除包含方重复定义，原生管道测试恢复直接编译；增加缺失导出的检查。没有修改生产符号查找逻辑。

新 tests/test_gfx_ini.py 编译整个真实 gfx_options.c，以独立进程重新读取配置验证“保存后重启”的行为。tests/test_optional_features.py 增加在文件操作前拒绝无效组合、缓存链接入口不绕过检查，以及正常上游包仍可用的检查。

## 验证结果

| 项目 | 结果 |
| --- | --- |
| 修复前设置反例 | 8 项中 5 项失败，稳定覆盖重复行；原始日志 gfx-before.log |
| 修复前构建组合反例 | 提前拒绝测试失败，旧流程确实触及包路径；原始日志 features-before.log |
| 修复后设置写入 | 8 项全部通过，含重复图形 / 联机项、重启读回、空文件、短行、保留原字节、末行无换行、2000 次长值扩写与 1000 次缩写 |
| 可选功能检查 | 7 项通过，含提前拒绝、缓存入口、合法上游运行时和源构建开关 |
| 资源构建 / 网络构建启动 | 3 项 / 14 项通过 |
| 正常 staging 的原生宿主夹具 | 编译、能力记录、SDK 网络调用和增量不重复编译通过 |
| 原生协议整合 | 33 项通过 |
| 原始 Console 原生管道测试 | 直接编译运行通过，不再需要诊断副本；三种协议、启动、重启请求、正常关闭和新会话检查通过 |
| Console 常规 Rust 检查 | 28 项通过、5 项按条件忽略 |
| Rust 实际管道启动 / 重启 | 两项通过，使用本轮直接编译的原始原生管道测试程序 |
| 当前分支正式宿主 | 正常 staging 591 个文件后，有 Yuzu + 自有减延迟 61 个宿主文件构建成功；无 Yuzu + 无减延迟 55 个宿主文件构建成功 |
| 完整游戏验证 | 使用前者、匹配的现有本地游戏 DLL，原样 ARCropolis 4.0.8 和自有减延迟加载；进入正式对战并达到 120 帧后正常退出 0，截图已检查 |

游戏 DLL、原样 NRO 和自有扩展算法没有为这三项修正改动。本轮复用匹配的既有游戏与角色模块编译结果，不表述为全部游戏从零转译 / 编译通过；真人跨机器联机和实际输入延迟仍待验收。

## 试用入口与回退

仍使用 build/own-delay-20261026/review/StartReview.cmd，原 Online Deluxe 入口的跳转也继续有效。只更新同一试用目录的宿主及能力记录 / 调试文件，保留配置、存档、资源、F8 选择及独立扩展文件。Console 本体代码和程序无需因本轮重新构建。

新版宿主 SHA256：7c638a7f30658222f2631707b3af65350736ee9908baee54b6ad63569a584c59。
旧宿主 SHA256：deb9218d31e70275fd3b589f2919b889af68ef4f3a0ca1eef4038fcde158bb3b。
旧文件完整备份：build/build-console-fix-20261026/trial-before。

证据：build/build-console-fix-20261026/validation.json、checks.json、stage.log、host-*.log、各专项日志、sessions/short-holds-battle。three-fixes.patch 与 implemented-sources.zip 记录本轮实际修正及源码。构建记录、EXE、扩展指纹和游戏 DLL 的一致性在部署前后均核对。

08 / 09 的本轮修正及本机回归已完成；最终版清洁构建、整套用户验收及打包更新流程仍按后续清单推进，不将本次通过扩大为发布完成。main 和稳定 yuzu 保持原提交。
