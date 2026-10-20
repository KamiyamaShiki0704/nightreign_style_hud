---
id: BUG-Hud-Map-Refresh
type: design
version: 1.0
updated: 2026-09-24
based_on: [spec.md@1.0]
---

# 加载判断与元素清理

目标工程为 nightreign_style_hud；沿用原 fsrs pin，不修改 Summon 或依赖缓存。
这是 ER 原生 DLL，不使用 UE / FrameRecorderProbe。

## 根因与原生证据

`CSNowLoadingHelper::is_loading_screen_active()` 把 helper+0xec..0xef 的未知字节按任一非零解释成加载。
本机 WW 2.7.1.0 的正常地图样本恰为 `[1,1,0,0]`，原生 FE HUD 为 Default(3)，玩家有效。
此前 render 又把复仇者清理会话不可用直接作为全局隐藏条件，导致正常地图 HUD 被隐藏。
加载中旧字节变成全零，原有 1.5 秒保护期过后反而允许绘制。

EXE SHA256：`1a3547101327f65d0c76da2f9190ac0aa66871ea42bae2aecc61e11a8b597891`。
见 `research/native-loading-proof.txt`：

- RVA 0x2a2880 写 helper+0xec=1；0x2a2ba1 将其复制到 +0xed 再清零。二者是逐帧锁存值，不能当加载标志。
- RVA 0x861d70 对 LoadingScreenData+0x10 写 word=1；0x861d80 清零。
- RVA 0x767e4f 检查 MenuMan+0x730；随后 0x767e6a 将 MenuMan+0x720 传给加载窗口创建过程。
- +0x08 是加载模式，不是“是否加载”；不使用该字节或旧依赖中未经验证的 timer 名称。

`research/normal-gameplay.json` 为用户确认正常地图的样本；
`research/teleport-sequence-3.json` 捕获 counter 2→3：50.290s 请求为1，54.105s归0，持续约3.8秒。
`research/normal-after-teleport.json` 用重新解析的单例确认加载结束后 HUD=3、请求=0、玩家有效。
第三组采样旧脚本缓存了 FE/World 单例地址，重建后这些字段不可信（出现92/169）；不将其用于加载后判定。
加载 helper/MenuMan 在这次采样未变；结束状态另取新样本交叉验证。脚本已改为循环内重新取单例。
第一、二组只有正常/传送后数据，不作为加载期间证据。

## 实现映射

- SPEC-HMR-01：render 仍调用会话观察以使旧绑定失效，但不再以其返回值控制全局 HUD。
- SPEC-HMR-02：`loading_state::screen_active` 读取已验证的 MenuMan+0x730；WW版本、结构偏移及四处原生指纹全部通过才允许读取。HUD 和被动清理共用该读取。未知版本返回 None；清理会话拒绝继续，HUD保留原有 FE/计数保护。
- HUD gate 合并实际加载请求、原生界面隐藏和计数变化后的1.5秒保护；请求为1期间保护不会因时间到而解除。
- SPEC-HMR-03：`begin_map_refresh` 清空元素槽、就绪魔法、锁定目标及全局伤害元素地址缓存、旧输出计时，记录待清理标志。会话代次变化也使这些缓存失效。新玩家有效后、采集活动效果前清理原有吸收/混合魔法输出。
- SPEC-HMR-04：普通 FE 菜单隐藏不产生刷新事件；复仇者家族HP记录不变。

## 自动 Probe / 人工边界

`cargo test -p nightreign-style-hud --locked --offline` 执行真实 render 使用的状态 gate、ChargeState刷新入口，以及原生指纹fixture比对。
六项 map_refresh_regression_tests 覆盖正常、长加载、首次观察已在加载、普通菜单、满槽就绪魔法、部分槽/缓存清除与重新填充。
原生字节fixture从上述本机EXE提取，额外验证每个指纹被改写时均拒绝。
已有复仇者测试继续覆盖家族进度保留和代次失效。

不在离线测试中伪造 PlayerIns 调用游戏函数。原生输出实际消除、渲染、吸收交互及清理稳定性由游戏内验收；离线通过不等于最终PASS。
