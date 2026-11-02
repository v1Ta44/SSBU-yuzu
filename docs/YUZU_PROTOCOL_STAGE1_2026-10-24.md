# Yuzu 协议接入：第一阶段（2026-10-24）

## 本次裁定与范围

用户要求以 main 为基础逐项整合，所有改动须先经裁定。本阶段依据“先做 yuzu 联机，暂时不做回滚对 yuzu 的适配”执行，只处理网络核心。F1/Console 界面、构建脚本的其他冲突、mod、图形、焦点和账户规则继续等待各项裁定。

当前分支 yuzu-dev；基线 main fea28c1；个人来源 yuzu b360a18。稳定 yuzu、main 和原工作目录保持原状。此阶段不是可发布或已完成人工联机验收的游戏版本。

## 已实现

- 游戏的最终网络注册统一由 nnldn_register 负责。nnnet_register_final 完成 SDK 基础注册后只调用该入口，不再另行注册一整套 Yuzu 网络接口。
- 上游与 Yuzu 共用 nnldn.c 中的接口名称/别名表。network_abi.h 定义 43 个操作；Yuzu 适配器按操作提供函数，不再维护第二份游戏符号名称表。
- 保留原有 Yuzu 房间协议、LDN 会话、ENet 和压缩数据处理。虚拟套接字内部仍使用已有 Yuzu 实现，尚未把两边的收发队列和等待机制改写成一套。
- 连接选择复用上游 online_mode 配置项，支持 yuzu、off 及现有 lan/host/join/lobby 等模式。SSBU_ONLINE 显式覆盖；无 online_mode 时兼容旧 online_server 配置。配置在进程首次读取时固定，更改后须重启。
- 启用个人扩展的构建在没有连接配置时默认 Yuzu；明确关闭或已有上游配置得到保留。非法模式会关闭联网并记录错误，不会自动改连另一种服务器。
- Yuzu 模式阻止上游大厅传输启动；上游/关闭模式拒绝 Yuzu join/自动连接，避免两套网络同时运行。
- 上游查询房间角色、人数和统计信息时，可以读取 Yuzu 的实际游戏会话。人数取游戏本地无线房间，不把服务器房间里的所有成员当成同一场对局玩家。显示线程使用尝试加锁，不等待房间锁；未取得锁时返回空闲统计。
- Yuzu/关闭模式阻止启动上游联网回滚。保留上游模式原有入口及本机 loop 诊断，没有增加回滚握手、输入帧同步或跨 Yuzu 回滚能力。

## 文件与用途

| 文件 | 改动 |
| --- | --- |
| runtime/network_protocol.h / .c | 连接配置选择、Yuzu 会话信息与统计适配。 |
| runtime/network_abi.h | 共用操作定义，避免重复维护游戏符号名称。 |
| runtime/nnnet.c | 处理本文件网络注册冲突，保留统一入口。 |
| runtime/nnldn.c | 单份接口名称表，根据选定协议绑定操作；转接房间状态。 |
| runtime/nnnet_yuzu.inc | 从自行注册 SDK 改为按操作提供适配函数。 |
| runtime/net_relay.c | 隔离上游传输，接入统一统计。 |
| runtime/yuzu_net.c / yuzu_net_internal.h | 隔离 Yuzu join/自动连接，提供无等待统计读取和 RTT。 |
| runtime/rb_net.c | 对未支持的连接方式禁止启动联网回滚；没有适配 Yuzu 回滚。 |
| tests/network_protocol.c / test_network_protocol.py | 使用真实注册入口和 SDK 适配器，通过本机独立 ENet 房间夹具验证收发。 |

## 本轮验证

- 33 项新原生检查全部通过：模式解析、注释/空白/BOM、旧配置兼容、环境覆盖、配置路径优先级、进程内选择固定、三种连接选择、回滚隔离，以及 SDK 开房/成员加入/发送/接收/关闭。
- 新夹具实际编译链接 nnnet.c、nnldn.c、network_protocol.c、yuzu_net.c、yuzu_ldn.c、yuzu_socket.c、rb_net.c；通过 SDK 注册的函数调用完成数据收发。游戏、渲染和上游传输由有界测试替身提供，误调用游戏/渲染服务会使测试失败，因此不把本轮结果扩展为完整游戏或上游真人联机通过。
- 原有 test_yuzu_backend.py 全部通过：入房固定报文字节、身份与头像、LDN 搜索、房主和加入者、成员事件、压缩数据、畸形包、旧描述符失效、断线唤醒、日志级别/限流/脱敏。
- 五个网络编译单元分别在 SSBU_ENABLE_YUZU=0 和 1 下编译，共 10/10 通过。
- 没有修改第三方 mod、存档或用户配置。测试的配置、程序与日志位于新工作目录 build/network-protocol-20261024 和 build/logs，依赖沿用原目录缓存。

复现新检查：

```text
python tests/test_network_protocol.py --clang <clang.exe> --deps <已有 ENet/Zstandard 缓存目录>
```

本轮证据：build/network-protocol-20261024/results.json、compilation.json、各项日志，以及 build/logs/yuzu_backend_test.log。

## 待后续裁定与验收

1. 构建接入：构建脚本须纳入 network_protocol.c、已有 Yuzu 源文件和 ENet/Zstandard，并为个人 Windows 构建设置 SSBU_ENABLE_YUZU=1。该宏默认 0，确保未启用扩展的上游/其他平台构建保持原有路径。目前 tools/lifter/build.py 的既有冲突仍待处理，不能声称常规完整构建已接入。
2. F1/Console 的连接方式入口、状态文字、重启与配置协作尚未修改；尤其不能把现有 F1 菜单对 yuzu 值的显示当成已经适配。
3. 其余合并冲突解决且构建完成后，在独立配置/存档目录进行完整游戏验证，再做跨机器 Yuzu/PC 真人联机验收。
4. 两边虚拟套接字内部实现的进一步合并另列处理项；本阶段只共享注册和操作边界，保留已验证的 Yuzu 数据收发语义。

本阶段不完成整个合并、不发布、不推送，也不合入稳定 yuzu。
