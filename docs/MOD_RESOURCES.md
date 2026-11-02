# 模组加载与 Console 管理

本开发分支于 2026-10-25 按用户裁定改用上游 Skyline → ARCropolis → 资源模组。旧稳定 yuzu 0.2.0 的个人虚拟 data.arc 后端不再参与本开发分支。原实现与历史测试仍可从稳定分支和实施前源码快照查看。

## 加载

完整插件转换、采集挂钩及重新生成游戏代码的步骤见 [上游 SKYLINE.md](SKYLINE.md)。Console 的管理功能与加载器是否准备完成独立；看到插件文件或开关成功，不能说明资源已在游戏内生效。

Console 默认将 `<构建或包根目录>/sd` 作为 SD，并向游戏显式传入 `SSBU_SD`。可在启动设置的 `[env]` 保存另一个 SSBU_SD；相对路径以游戏程序目录为基准。单次启动覆盖不能切换到另一个未在管理页显示的 SD。`SSBU_SKYLINE=0` 可暂时关闭上游插件入口。

```text
sd/
  atmosphere/contents/01006A800016E000/romfs/skyline/plugins/libarcropolis.nro
  ultimate/mods/<启用包>/
  .console-mods/disabled/<停用包>/
  .console-mods/trash/<时间-包名>/
  .console-mods/staging/<导入暂存>/
config/console-mods.json
```

启用/停用实际移动完整包，使它进入或离开 ARCropolis 扫描目录。回收区和暂存区在扫描目录以外。别名仅存入 Console 配置，不改包内 config.json、模型、贴图、音频或 plugin.nro。手工放入启用目录的包也会显示。原生加载器自己的工作区/预设仍由原生工具控制，Console 不重写它们；因此目录内有包也不等于原生预设已选中它。

## 管理操作

- 查看、别名、ZIP/文件夹导入、启停、打开文件夹、软删除；游戏运行时只能查看和打开。
- 导入先复制到扫描范围外，再完整移入目标目录。重名包会拒绝导入；不会覆盖既有包。
- 路径越界、ZIP 链接、重解析目录和递归导入自身暂存区被拒绝。保存配置失败时会恢复已移动的包；失败暂存保留便于排查。
- 导入时保留并备份旧 config/mods.json；对同名包迁移别名和启用选择。不会自动搬动原工作目录的包，不迁移旧优先级或换槽。
- 暂不提供优先级和虚拟换槽；它们原来由个人后端实现，当前没有经过验证的原生对应操作。

`--with-mods` 已退役并明确报错；`--without-mods` 只作为旧命令的兼容参数，不关闭 Skyline。旧 SSBU_RESOURCE_MODS / SSBU_MOD_PROFILE / SSBU_MOD_CACHE / SSBU_MOD_REGION 设置会阻止 Console 启动并提示移除，避免误以为旧开关还控制新加载器。

管理配置使用独立 schema 1；较新或损坏的配置不会被静默覆盖。构建记录继续绑定程序哈希，resource_mods=false 表示旧资源后端已移除，skyline_loader=true 表示保留上游插件入口，均不承诺某个第三方插件已经完成适配。

## 验证

常规管理回归包含真实目录启停、导入、软删除、原文件字节一致、迁移、失败回退、ZIP 越界和目录链接检查。还可用 SSBU_TEST_MOD_ORIGINAL 与 SSBU_TEST_MOD_EVIDENCE 指定原包及全新诊断目录，执行 Rust 的 real_original_package_lifecycle 忽略测试，生成每个文件的 SHA-256 证据。实际插件与画面验证详见本次阶段报告。
