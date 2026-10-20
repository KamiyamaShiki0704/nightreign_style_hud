---
id: FEAT-Revenant-Passive-Retirement
type: design
version: 1.0
updated: 2026-09-23
based_on: [spec.md@1.0]
---

# 实现映射

只读来源：Summon revision `01491edfa709545786d0fe795ab2fb2458dca4e1` 的
`src/chr_lifecycle.rs`、`src/retirement.rs` 与原生字节 fixtures。
HUD 基线 `215a2158bf32e5027dc9167368ca68bcee705368`；KB revision
`9511e3e8137fd2cda9fca74bb07a4b688fe89087`。保留原工作区未提交的送葬者方向改动。

- 将 Summon 的原生生命周期与引用排空模块及 fixtures 固定复制到 HUD crate；不引入运行时 Summon DLL 依赖，不升级全工作区 fsrs。
- `release_revenant_debug_soul` 先隐藏并停用，向独立队列转移 handle、实例地址与世界/玩家/加载代次。队列不依赖当前角色，切换后仍由 FrameEnd 处理。
- 帧末接受原始 ordinal 168 和 Summon 实测 `0x900000a8`；同一 HUD 的 render 与释放使用互斥门，FrameEnd 用 try_lock 避免等待渲染线程。
- 限 WW2.7.1.0，保留 Summon 原生函数指纹、所有权检查、六个任务注销、原生槽位脱离、两个安静帧末、析构 flag=0 和 owner deallocate。
- 每次帧末独立检查加载计数、加载标志、世界及玩家；发生变化时丢弃旧跟踪并隔离已脱离对象，禁止旧上下文重试。渲染侧通过代次变化清空旧魂灵绑定。
- 存活维护不重置已绑定单位的死亡状态；初次绑定仍执行初始化。保持 60 秒和 1 秒退场等待。
- 专用 `nightreign_revenant_cleanup.log` 输出 staged / detached / draining / destroyed / quarantined 与拒绝原因，正常 HUD 的关闭日志行为不变。

P01/P02/P03 对应退场、死亡、角色切换调用和独立队列；P04 对应槽位脱离与队列排空；P05 对应加载代次隔离；P06 对应继承的 fixture/分配器/DrainGate 测试及新增适配测试。
这是 ER DLL，不使用 UE 地图、PIE 或 UECLI。游戏内 P01–P05 等待人工操作与日志，离线检查不声明运行时通过。
