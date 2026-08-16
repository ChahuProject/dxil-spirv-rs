# 文档

这里是 **dxil-spirv-rs** 的文档中心 — [dxil-spirv](https://github.com/HansKristian-Work/dxil-spirv)（DXIL/DXBC → SPIR-V）的安全 Rust 绑定。

[English](README.md) | [中文](README.zh-CN.md)

## 文档地图

| 文档 | 受众 | 内容 |
|---|---|---|
| [usage.md](usage.zh-CN.md) | **使用者** — 消费该 crate 的人 | 安装、快速开始、完整 API 讲解、重映射配置、错误处理、限制 |
| [platform-support.md](platform-support.zh-CN.md) | 使用者 + 维护者 | 支持的 OS/架构矩阵、构建要求、DXC 测试框架注意事项 |
| [architecture.md](architecture.zh-CN.md) | **开发者** — 修改该 crate 的人 | `-sys`/安全层拆分、FFI 边界、build.rs 链接闭包、bindgen 规则、跨平台坑（即"踩坑实录"） |
| [testing.md](testing.zh-CN.md) | 开发者 | 测试套件架构、829 个着色器覆盖声明、类别分解、回归基线机制 |
| [ci.md](ci.zh-CN.md) | 开发者 | CI job、平台策略、DXC 处理、缓存策略及其塑造它的坑 |
| [contributing.md](contributing.zh-CN.md) | 贡献者 | 如何贡献、代码规范、AI 维护政策、下文定义的文档标准 |
| [changelog.md](changelog.zh-CN.md) | 所有人 | 变更了什么、为什么 — 项目从首个提交到 100% 测试覆盖的演进 |

## 文档标准（如何共享文档）

任何人 — 人类或 AI — 在添加或编辑项目文档时都必须遵守以下规则。它们保证文档可发现、可审查、不冲突。

### 位置与命名

1. **所有项目文档都放在 `docs/` 中。** 顶层除 `README.md`（门面）外不允许有其他 `*.md` 文件。不要把文档放进源码目录 — 代码注释描述代码，`docs/` 描述项目。
2. **一题一文件。** 按受众和关注点拆分（见上方地图）。出现新主题就新增 `docs/<topic>.md`；文件超过约 400 行就拆分。
3. **kebab-case 文件名，不带数字后缀** — `platform-support.md`，不是 `platform_support.md` 或 `support_v2.md`。旧文件被*删除*或重命名，绝不复制出 "v2"。
4. **图片和资源放在 `docs/assets/`**，用相对路径引用。仓库根目录不放二进制文件，链接不用绝对路径。
5. **每个 `docs/*.md` 都必须能从本索引到达。** 本索引是入口；文档没列在这里就不算可发现。
6. **交叉引用使用相对路径**（`[testing.md](testing.md)`），不用绝对 URL，保证文档在 GitHub、crates.io 和离线环境都能用。

### 双语约定

7. **每个给人看的文档都有中英两个版本**：英文 `<name>.md`，中文 `<name>.zh-CN.md`。技能文件（`.agents/skills/`）除外。
8. **两个版本顶部都有语言切换链接**：形如 `[English](...) | [中文](...)` 的切换行，英文文件指向对应 `.zh-CN.md`，中文文件指向对应英文文件。翻译必须与英文版保持同步 — 改英文必改中文，反之亦然。
9. **翻译要求**：代码块、命令、路径、commit hash、API 名称保持原样；技术术语（DXIL、SPIR-V、bindgen、CMake、Cargo、FFI、RAII）保留英文；正文翻译为简体中文。

### 内容规则

10. **认清受众。** 文档要么*给使用者*，要么*给开发者* — 只写给一类人，另一类用链接指过去。`usage.md` 从不解释 C++ 核心如何链接；`architecture.md` 从不重复讲解如何加依赖。
11. **覆盖事实必须带数字。** 像"所有上游测试均已覆盖"这样的断言必须附证据：*829/829 个着色器，由 `test_completeness_check` 强制*。数字过期就更新 — 过期的数字比没有数字更糟。
12. **代码示例必须可运行。** 每个片段都应能从全新 crate 直接复制粘贴。优先用 crate 里的 `no_run` doctest，而不是纯文字片段。
13. **坑记录在它咬人的地方。** 跨平台坑（链接标志、bindgen 符号性、DXC 可用性）属于解释受影响子系统的文档 — 见 [architecture.md](architecture.zh-CN.md) 的踩坑实录和 [ci.md](ci.zh-CN.md) 的 CI 历史。没有理由就不要删坑条目；它们是花钱买的教训。
14. **变更记录条目是事实性的，不是宣传。** 说明改了什么、为什么；引用引入它的提交。不要有 AI 风格的废话。

### AI 维护政策（同样适用于文档）

15. 本项目由 **AI 维护**：AI 生成和 AI 编辑的代码与文档被明确欢迎（完整政策见 [contributing.md](contributing.zh-CN.md)）。AI 贡献者必须遵守与人类完全相同的标准 — 包括本文档。
16. AI 编辑的文档必须说明数字和断言的依据（例如"在提交 X 上运行 `cargo test` 验证"）。没有验证路径的断言是缺陷，不是观点。

## 快速定位

```
README.md                 ← 门面（30 秒概览 + 链接）
docs/                     ← 其余一切，按受众组织
  usage.md                ← 给使用者
  platform-support.md     ← 给使用者 + 维护者
  architecture.md         ← 给开发者
  testing.md              ← 给开发者
  ci.md                   ← 给开发者
  contributing.md         ← 给贡献者
  changelog.md            ← 给所有人
.agents/skills/           ← AI 维护技能（引用这些文档）
```
