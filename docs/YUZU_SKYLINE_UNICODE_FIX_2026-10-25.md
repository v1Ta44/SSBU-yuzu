# 中文 mod 路径修复与复验（2026-10-25）

状态：用户于 2026-10-25 明确确认“验收通过”。本次中文路径修复及对应第五阶段验收已通过。工作目录 D:/pcsmash-yuzu-dev-20261024，分支 yuzu-dev；没有提交合并、合入稳定 yuzu 或发布。

## 原因和改动

插件枚举出来的是 UTF-8 中文名字，原宿主却用 Windows ANSI 接口解释后续路径。在本机 GBK 代码页下，44 个真实包的目录存在性查询全部失败，插件无法进入目录。修复统一 nn::fs / Skyline 的路径为 UTF-8，仅在 Windows 文件系统边界转换成 UTF-16 并调用 Wide API。无需改包名或改包内文件。

- runtime/fs_host.h：集中严格编码转换、目录/文件操作、枚举及系统路径读取。
- runtime/nnfs.c：挂载根、读写/重命名/删除、存档日志和资源遍历使用相同编码；保持上游 FSA 分派和存档提交/恢复算法。fs_user_root 对既有 album 调用者保留 ACP ABI，账号名字逻辑保持已裁定实现。
- runtime/nnfsdir.c：统一枚举名称与下一层文件访问编码，包括存档工作副本隐藏检查。
- runtime/skyline.c：SD 环境路径、插件目录扫描、注册表/日志打开使用同一转换；插件目录通过 InitOnce 初始化。
- tests/fs_unicode.c、tests/test_fs_unicode.py：调用真实 SDK 文件系统处理器，使用独立临时目录检查中文、英文、日文和表情字符。

本次修复只涉及上述六个源/测试文件。图形、延迟滑块、Yuzu 回滚没有接入。保留 README.md 和 runtime/nvn_vk_m4.c 两个既有合并冲突。

## 验证证据

| 项目 | 结果 | 证据（相对 build/skyline-unicode-20261025） |
| --- | --- | --- |
| 修复前运行同组回归 | 67 项中 26 项失败，复现非 ASCII 路径问题 | fs-checks-before.log |
| 修复后运行同组回归 | 67 项，0 失败；覆盖嵌套目录、枚举、创建/读写、重命名、删除、存档提交/回滚、既有恢复自测、非法 UTF-8 | fs-checks.log |
| 有 / 无 Yuzu 宿主 | 两种构建均成功 | host-build.log、host-build-no-yuzu.log |
| 两种宿主完整 SDK 自测 | 文件系统与目录专项通过；整体仍有既有 erfcf 5 ulp 一项失败，与修复前基线相同，不能记为全通过 | ssbu_native_unicode.exe.sdk.log、ssbu_native_unicode_no_yuzu.exe.sdk.log |
| 全部原包独立副本实机扫描 | 44/44 个包均进入；独立 SD 根目录同时含中文、日文及表情字符；正常管道退出，exit 0 | all44-scan-results.json、sessions/all44-unicode-menu-background/result.json |
| 实际资源替换 | 用户“游戏加载动画”的自定义图标已显示 | sessions/all44-unicode-menu-background/shots/all44-loading.png、loading-animation-detail.png |
| 角色模型实机对战 | 使用更新后的原验收 EXE，在原样“Ukon替换火狐”中文包中读取 c00 模型/纹理，头像和角色模型实际显示；游戏确认火狐 kind 7 对马里奥 kind 0，运行 120 个正式对战帧后正常退出，exit 0 | fox-model-results.json、sessions/fox-representative-v4-battle/game.log、shots/fox-replacement.png（同 session） |
| 原包完整性 | 源文件和独立全量副本的 2122 个文件 SHA256 与测试前一致 | original-mods-sha256.json、originals-after-all44.json、final-mod-integrity.json |

全量副本是独立文件，不是硬链接；验证缓存、配置和存档放在诊断目录。没有修改第三方 config.json、模型、纹理、音频或 NRO。

角色验证前的脚本尝试保留在 fox-representative-battle / v2 / v3 session：前两次停在菜单，v3 虽完成对局但诊断参数以数字构造了不存在的 UI 名称，实际为马里奥，不计作火狐验证；v4 改用原生角色名后，游戏记录、实际资源打开及画面一致。未为测试改变运行时代码。

## 验收版

继续使用 build/skyline-console-20261025/review/StartReview.cmd。运行程序及配套构建记录、调试信息已更新；此前版本保存在 before-review-update。Console、用户 SD 设置和原始包保持原样。

本次 EXE SHA256：5723d80e72ea14c0a5d1caf84488f7b50ac2785568603bba43afb06fa76d34ea。源验证程序与验收入口程序哈希一致；部署清单见 review-update.json。四个运行时源文件与构建副本逐字节一致，源码哈希见 final-source-sha256.json。

源码修复前快照：before-fix-sources.zip；仅本次修复的可审查补丁：unicode-fix.patch。

## 范围与后续验收

“44 个包都能进入”是目录读取结论，不等于全部包的所有资源已兼容。全量扫描后，ARCropolis 对部分配置、扩展/新增资源路径索引及两处文件大小修补发出警告；保留原始日志，不修改包或隐藏警告。已确认加载动画和火狐 c00 替换；尚未逐一验证 44 个包的所有角色/槽位。警告摘录见 arcropolis-warnings.txt。其余独立兼容问题需按实际症状另列处理。

2026-10-25 用户明确回复“验收通过”，据此记录本次修复版验收通过。此次确认对应原生资源加载、Console 管理及中文路径修复的当前实现；保留上述自动验证边界，不推定所有第三方包、角色和槽位均已逐一兼容。后续合并项仍须先提交方案供用户裁定；本次验收不自动授权合入稳定分支、版本更新或发布。
