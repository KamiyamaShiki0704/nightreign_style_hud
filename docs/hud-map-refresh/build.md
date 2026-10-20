---
id: BUG-Hud-Map-Refresh
type: build
version: 1.0
updated: 2026-09-24
based_on: [spec.md@1.0, design.md@1.0]
---

# 构建与验证

自动结果：**PARTIAL**。71 项离线测试、Release 构建及二进制检查通过。
用户退出游戏后已备份并安装修复版，安装哈希与产物一致。
人工结果：**PASS（用户反馈）**。2026-09-24 用户在安装后回复“正常了”，本轮 HUD 可见性与隐士刷新清理问题验收关闭。
该反馈为整体确认，没有逐项报告每个 Probe 的过程；不扩展为复仇者内存释放或长期稳定性验收。

## Revision / 修改范围

- HUD 基线 `215a2158bf32e5027dc9167368ca68bcee705368` + 当前未提交更改。
- KB 基线 `9511e3e8137fd2cda9fca74bb07a4b688fe89087`。
- fsrs pin 仍为 `3c8c1d7633a99309fb004c9f894ea10b7967d0e0`。
- 保留之前送葬者方向与复仇者清理变更；本轮只修改本工程 HUD/加载/隐士刷新代码及其文档、fixture。
- Summon 只作为上轮来源，未修改；没有修改 HKS/参数、公开发布或提交 Git。

## 根因与修复

复仇者清理会话将 helper 的逐帧字节误当作加载状态；render 又将会话不可用作为全局 HUD 隐藏条件。
正常地图因此隐藏，而真正加载时旧字段归零、1.5秒保护到期反而出现 HUD。
新 `loading_state.rs` 使用经过本机反汇编及正常/传送对照确认的原生请求字节，并核对版本和四处指纹。
HUD 与清理共用此读取，但清理会话不可用不再直接隐藏 HUD。长加载请求未结束就继续隐藏。

隐士问题源于原刷新入口只重置复仇者。现同时清空隐士槽位、就绪魔法、锁定目标、伤害元素地址缓存和输出计时。
新玩家有效后、读取活动效果前清掉旧的吸收及混合魔法输出；普通菜单不触发刷新。

## 自动与运行证据

最初执行 `cargo test -p nightreign-style-hud --locked --offline map_refresh_regression_tests -- --nocapture`：
两个测试分别因正常HUD被清理会话隐藏、刷新保留满槽而失败（0 passed / 2 failed；本轮早期工具输出）。
修复后最终执行：

| 检查 | 结果 | 证据 |
| --- | --- | --- |
| 全量 locked/offline tests | 71 passed / 0 failed | `target/delivery-hud-map-refresh/tests.log` |
| locked/offline release build | PASS，无警告 | `target/delivery-hud-map-refresh/build.log` |
| 原生加载布局fixture | PASS；四处修改分别拒绝 | `tests/fixtures/ww271-loading-request.bin`（相对 crate） |
| AMD64 / 无动态 CRT 导入 | PASS | `target/delivery-hud-map-refresh/binary-check.json` |
| `git diff --check` | PASS | 交付时执行，无空白错误 |
| 安装与备份哈希 | PASS | `target/delivery-hud-map-refresh/delivery.json` |
| 正常地图只读样本 | FE=3、实际请求=0，旧flags=[1,1,0,0] | `research/normal-gameplay.json` |
| 传送只读样本 | 实际请求为1约3.8秒、结束归0 | `research/teleport-sequence-3.json` |
| 加载后重新定位单例 | FE=3、实际请求=0、玩家有效 | `research/normal-after-teleport.json` |

采样限制及不能使用的旧地址字段见 design.md。采样没有写游戏内存或调用游戏函数。
七项新增测试覆盖 SPEC-HMR-01..04 的可离线部分；原生效果消除、真正画面及吸收交互仍为人工项。
这不是 UE 工程，无 FrameRecorder 报告。

## 本地交付

- 工程产物：`target/delivery-hud-map-refresh/nightreign_style_hud.dll`
- 大小：6,745,600 bytes
- SHA256：`2C4DD102341C84D59B48988134255E5A82AB43BF23C6B0F4706B22A290D5721E`
- 安装目标：`F:\GoldenAge\_GoldenAge\dll\nightreign\nightreign_style_hud.dll`
- 2026-09-24 21:43:56（Asia/Shanghai）确认游戏进程退出，备份并安装；安装后 SHA256 与上方产物一致。
- 旧版备份：`F:\GoldenAge\_GoldenAge\dll\nightreign\nightreign_style_hud.pre-hud-map-refresh-20260924-214356.dll.bak`
- 备份 SHA256：`D234C7D9CF1504D02869734E9643391EEE184E13DD165EF86593697693E1AA50`。

## 人工验收

本轮验收入口为：正常地图 HUD 出现；传送加载全程隐藏、结束恢复；隐士带1–3个元素及就绪魔法传送后清空，重新吸收正常；普通菜单开关保留槽位。
用户安装后反馈“正常了”，按上述两个报障的整体修复确认记录，未取得逐项操作记录。
上轮复仇者死亡/60秒到期退场/换角释放与反复召唤仍待专项验收，需结合清理日志判断。
