---
id: FEAT-Revenant-Passive-Retirement
type: build
version: 1.0
updated: 2026-09-23
based_on: [spec.md@1.0, design.md@1.0]
---

# 构建与本地交付

自动结果：**PARTIAL**。64 项离线测试、Release 编译、DLL 校验通过；游戏运行验收尚未执行。
当前 Spec 的 SPEC-RPR-01 至 05 均有对应代码路径，运行时行为仍属于人工项，不能据此宣称已最终通过。

## Revision 与边界

- HUD 基线：`215a2158bf32e5027dc9167368ca68bcee705368` + 本次未提交改动。
- Summon 只读参考：`01491edfa709545786d0fe795ab2fb2458dca4e1`，本轮未修改。
- KB：`9511e3e8137fd2cda9fca74bb07a4b688fe89087`。
- fsrs 保持 `3c8c1d7633a99309fb004c9f894ea10b7967d0e0`；Cargo.lock 只增加本 crate 已有 workspace Windows 依赖关系。
- 保留进入任务时已有的送葬者方向、CSV、README 和测试变更；本地 DLL 继续包含那些变更。
- 本轮没有 Git 提交/推送、公开 Release、参数表或 HKS 变更。

## 实现

- `src/chr_lifecycle.rs` 和 `src/retirement.rs` 完整复用 Summon；SHA256 分别为
  `815B2C91F6ED8E99268615E4A9D695E2C3E85BD52E5BEFFFB11FE2456CCD6BD4`、
  `C42BA68FEBCC50BEDBCCD72A650D4CD5420DE3A3E5E6E7E444990C455B02FECD`，与源文件相同。
- `src/revenant_cleanup.rs`：独立队列、FrameEnd 编码、渲染访问同步、WW2.7.1.0 gate、加载代次隔离、未绑定取消请求接管、日志。
- `src/state.rs`：死亡/退场/换角请求移交队列；不再在死亡检查前复活已绑定魂灵；实例地址及创建前指针校验；旧代次绑定失效。
- `src/hud.rs`、`src/runtime.rs`、`src/lib.rs`：安装、模块路径、渲染同步与加载观察。
- 新增 `src/revenant_cleanup_tests.rs` 与三个原生 fixture。上述 src 路径均相对 `examples/nightreign-style-hud/`。

## 自动验证

| 检查 | 结果 | 证明边界 |
| --- | --- | --- |
| `cargo test -p nightreign-style-hud --locked --offline` | 64 passed / 0 failed | 原有 53 项 + 11 项生命周期、布局、代次及死亡判定检查；不执行游戏内释放 |
| `cargo build -p nightreign-style-hud --release --locked --offline` | PASS，无编译警告 | 优化 DLL 成功构建 |
| 原生模块/三个字节 fixture 与 Summon 对比 | 完全相同 | 复用已验证原生代码；HUD 接入仍待运行验证 |
| PE 和导入表 | AMD64，无动态 MSVC/UCRT 导入 | 二进制形态正确 |
| 安装后 SHA256 | 与构建产物一致 | 确认本地安装的是本轮构建 |
| `git diff --check` | PASS | 无新增空白错误 |

证据：`target/delivery-revenant-retirement/build.log`、`binary-check.json`、`delivery.json`。
这是 ER DLL，未运行 UE FrameRecorderProbe；当前无本轮游戏运行日志。

## 本地交付

2026-09-23 02:50（Asia/Shanghai）确认游戏未运行，备份并安装：

- DLL：`F:\GoldenAge\_GoldenAge\dll\nightreign\nightreign_style_hud.dll`
- 旧版备份：`F:\GoldenAge\_GoldenAge\dll\nightreign\nightreign_style_hud.pre-revenant-retirement-20260923-025006.dll.bak`
- 工程副本：`target/delivery-revenant-retirement/nightreign_style_hud.dll`
- 新 DLL：6,744,576 bytes；SHA256 `D234C7D9CF1504D02869734E9643391EEE184E13DD165EF86593697693E1AA50`。
- 旧 DLL：SHA256 `0EE756B2339044DC0B281A21BC8B42B6AF744F38FF1774A81EF2EA46FD61B742`。

没有改动加载器或独立精密镜头。复仇者被动无需另外加载 Summon DLL，也无需新增 SpEffect/配置。

## 人工验收入口

启动游戏后读取 DLL 同目录 `nightreign_revenant_cleanup.log`。
依次测试存活 60 秒并退场、击杀被动魂灵、切换角色、累计至少 20 次生成/清理、清理中传送或返回菜单。
正常顺序为 `phase=staged` → `phase=detached` → `phase=draining` →
`outcome=destroyed retirement=frame-end-drained`。只有最后一条证明释放调用已完成。
切图可出现 `phase=quarantined`，不能将它统计为已释放。
若出现 `phase=refused` 或一直没有 destroyed，保留日志用于诊断，不能视为通过。

未证明：HUD 集成后的游戏内释放、动画表现、长期反复召唤与过图稳定性。
范围外：模型缓存/子弹释放、旧版游戏兼容、所有联机情况。
最终 PASS/FAIL 由用户验收给出。
