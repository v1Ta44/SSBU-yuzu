# Yuzu 网络构建与启动接入（2026-10-24）

## 授权与完成范围

依据用户对“网络的构建与启动接入”的明确批准执行。本阶段将第一阶段网络核心纳入源代码构建、构建特性记录和命令行启动；没有修改 F1/Console 界面，没有进行 Yuzu 回滚适配，没有处理 mod、渲染或焦点运行时代码中的剩余冲突。

分支为 yuzu-dev，工作目录 D:/pcsmash-yuzu-dev-20261024。main 仍为 fea28c1，稳定 yuzu 仍为 b360a18。尚处于合并过程中，本阶段没有生成合并提交或发布。

## 构建行为

- tools/lifter/build.py 保留上游的 host / gamedll 分工，以及 Linux/macOS、ImGui、FFmpeg 和现有优化设置；sdkdata.c、nro_registry.c、sdklift_gen.c 仍由游戏 DLL 持有，不重复编入运行程序。
- 新增 --with-yuzu / --without-yuzu。个人 Windows 源码构建默认纳入 Yuzu 扩展；其他平台默认关闭，目前显式启用会报明不支持。仅表示构建选择已隔离，不代表本轮已经验证 Linux/macOS 完整构建。
- 启用时纳入 network_protocol.c、Yuzu 房间/LDN/套接字源文件及原有网络控制入口；为 C 和 C++ 同时设置 SSBU_ENABLE_YUZU，静态编译链接 ENet/Zstandard。实际链接验证发现并补齐了所需的 advapi32 系统库。
- setup 将网络依赖缓存路径显式传给子构建，默认读取仓库 deps；不因构建目录位于其他位置而误用相邻工作目录的依赖缓存。
- 改变网络构建选项会使 link/check 重新执行；改变实际 C/C++ 编译选项会使对象文件重新编译。重复调用不累加网络头文件参数。
- 保留已有 --with-mods / --without-mods 和 --with-latency-slider / --without-latency-slider 的开关、构建及 13.0.4 保护逻辑，只完成与上游构建骨架的机械衔接。对应运行时冲突和功能验收仍待裁定。
- 程序 .build.json 在现有 schema_version=2 上增加 yuzu_network 与 network_abi 字段，并绑定 EXE 的 SHA256；现有便携打包器仍能读取原有字段。没有改动更新器或便携包产品流程。
- 开启 Yuzu 时拒绝使用 --runtime 直接替换运行程序；目前须从源码构建宿主。拒绝发生在解包/复制前，--runtime-any-tree 也不能绕过。需要上游原生预编译路径时可明确 --without-yuzu；这不意味着其余个人扩展已获得预编译包兼容验收。

## 启动行为

- 新增 scripts/network_launch.py，按照原生网络核心的配置路径和优先级判断连接方式，实际程序的特性记录决定是否允许 Yuzu。
- scripts/run.py 在处理完所有 --env 后再确定连接模式，避免已保存的回滚选项覆盖显式连接选择；普通连接为 yuzu / upstream / off。
- Yuzu 或关闭联机模式中，本次进程的联网回滚参数被关闭；不会改写 graphics.ini 中保存的回滚选项。本机 loop 诊断继续保留。
- 上游模式按原有条件启用回滚，并保留显式 SSBU_RB_NET=0 的优先级。
- scripts/yuzu_play.py 明确请求 Yuzu；即使配置上次保存的是 upstream/off，也不会误连上游。验证程序确实具备 Yuzu 构建记录，并按 state.json 选择游戏版本目录，不再将启动文件路径固定为 13.0.4。这是路径修正，不扩大其他扩展对 13.0.5 的验证范围。
- 复制/刷新测试 EXE 时同时复制其特性记录；无记录、已替换或不支持的 EXE 不会冒充支持 Yuzu。
- runtime/boot.c / hostrt.c 的网络启动、退出引用受构建开关保护，关闭扩展时不再无条件要求 Yuzu 网络符号。

## 处理的冲突

完成 tools/lifter/build.py、scripts/setup.py、scripts/run.py 三个脚本的冲突处理；保留两边已有选项和上游新增构建机制。剩余 7 个文本冲突：

- README.md
- runtime/nnfs.c
- runtime/nnhid.c
- runtime/nnmisc.c
- runtime/nvn_vk_m4.c
- tests/oe_focus_test.c
- tests/test_oe_focus.py

## 实际验证

| 验证 | 结果与边界 |
| --- | --- |
| 构建/启动/既有开关及诊断回归 | 26 项通过，0 失败、0 跳过。包括 14 项新构建启动检查、5 项 mod 构建开关检查、5 项可选扩展检查和 2 项诊断启动检查。 |
| 网络核心 | 33 项原生检查通过：连接选择、配置路径、SDK 注册与开房/成员/收发、回滚隔离。 |
| 标准源码复制 | 使用 setup.step_stage 将当前源码复制到独立诊断构建目录，网络新文件及构建助手齐全。 |
| 实际编译链接 | 使用复制后的 build.py host 流程，真实编译网络源码及 ENet/Zstandard，使用上游游戏导入库生成、SDL3 和正常链接参数，成功生成网络测试程序。 |
| 增量构建 | 对同一隔离程序第二次调用宿主构建，重新编译源码数为 0。 |
| 程序记录与运行 | 生成的特性记录通过实际 EXE SHA256 验证，测试程序通过游戏 SDK 接口完成本机协议收发。 |
| Python 与差异检查 | 修改脚本可编译，批准范围文件无残留冲突标记；差异空白检查通过。 |

实际编译链接使用的 main 为独立网络夹具，并在该夹具目录内省略游戏加载器；未编译游戏生成代码或渲染器。运行时的游戏/渲染依赖由明确的测试替身提供，不能据此宣称完整游戏编译、真人 Yuzu 联机或上游真人联机通过。原游戏文件、第三方 mod、用户配置和存档均未用于本阶段验证。

既有测试夹具已补上新构建能力记录、上游上下文字段和完成构建的输出文件；保留其原本的行为断言，并增加 Yuzu 连接/回滚参数断言。

## 证据与复现

- build/network-build-20261024/final-regression.json：26 项回归，0 失败/错误/跳过。
- build/network-build-20261024/final-regression.log：逐项输出。
- build/network-build-20261024/host-build-results.json：实际宿主构建、增量与执行结果，以及独立夹具目录。
- 对应夹具目录中的 build.log、runtime.log、运行程序及 .build.json。
- build/network-protocol-20261024/results.json：33 项网络核心检查。

```text
python tests/test_network_build_launch.py
python tests/test_network_host_build.py
python tests/test_network_protocol.py --clang <clang.exe> --deps <已有依赖缓存目录>
```

前两项原生构建验证可通过 RECOMP_CLANG 指定编译器、SSBU_NETWORK_DEPS 指定已有依赖缓存。测试文件的默认路径为本仓库 deps；不写入稳定构建目录。

## 待裁定事项

下一项建议为 F1/Console 的连接方式、实际状态显示和重启协作；需要先提交交互与改动方案，经用户裁定后再实施。完整游戏构建仍受剩余运行时冲突影响，必须在相应方案获批后逐项解决。Yuzu 回滚适配继续排除在本轮范围之外。
