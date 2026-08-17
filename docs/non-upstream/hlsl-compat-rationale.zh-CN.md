# hlsl-compat:cbuffer vec4 对齐后处理(NON-UPSTREAM EXTENSION)

> 本文档论证的是 dxil-spirv-rs 的**非上游扩展** `non_upstream::hlsl_compat`,
> 对应 crate feature `non-upstream-hlsl-compat`(默认关闭)。
> 它不是上游 dxil-spirv / dxbc-spirv 的功能,也未获得上游认可。
> 设计动机、复现步骤、根因分析与验证过程全部记录于此。

---

## 1. 背景与动机

dxil-spirv-rs 的主用例是:把 D3D11/D3D12 捕获的着色器(DXBC 或 DXIL)转成
SPIR-V,再交给 SPIRV-Cross 反编译回可读的 HLSL / GLSL / MSL 供人阅读。

实测(渲染调试器,Unity Universal 3D Sample):

| 阶段 | D3D12(105 个着色器) | D3D11(62 个着色器) |
|---|---|---|
| DXIL/DXBC → SPIR-V 转换 | 全部成功 | 全部成功 |
| HLSL 反编译失败率 | **74%(78 个)** | **92%(57 个)** |
| GLSL / MSL / JSON / 汇编 | 全部成功 | 全部成功 |

失败错误全部是同一类:

```
The SPIR-V operation is unsupported: cbuffer ID N (name: _9_11), member index 0
(name: _m0) cannot be expressed with either HLSL packing layout or packoffset.
```

GLSL/MSL 对同一批着色器全部成功,说明问题**只存在于 SPIR-V → HLSL 这一环**。

## 2. 现象:stride-4 标量 cbuffer 视图

失败着色器的 SPIR-V 里,部分 cbuffer 被上游输出成**紧凑标量数组**:

```text
struct { _m0 @ 0: float[536] (ArrayStride 4) }   ← 源 DXBC 声明是 134 个 vec4(134×4=536)
```

同一 shader 里另一个 cbuffer 却是:

```text
struct { _m0 @ 0: float4[134] (ArrayStride 16) }
```

stride-4 的 `float[N]` 在 GLSL(std430/std140 宽松布局)下完全合法,但
spirv-cross2 的 HLSL 后端把 cbuffer 建模为 **16 字节 vec4 寄存器**:
它计算出的 HLSL packed array stride 是 16,与 SPIR-V 的 4 不符,于是拒绝
(宁报错也不生成非法 HLSL——这是正确行为)。

## 3. 复现(仓库内即可)

上游测试套件里的 `alloca-opts/float4-array-load.dxil` 稳定复现完全相同的错误
(连 `_9_11` 命名都一致):

```text
cargo run -p hlsl-compat-inspect -- repro tests/shaders/alloca-opts/float4-array-load.dxil
```

输出:

```text
HLSL compile before: UnsupportedSpirv("cbuffer ID 11 (name: _9_11), member index 0
  (name: _m0) cannot be expressed with either HLSL packing layout or packoffset.")
HLSL compile after:  OK
rewritten views: 1 (skipped: 0)
```

`dump` 命令可以查看 shader 的 cbuffer 双视图结构:

```text
cargo run -p hlsl-compat-inspect -- dump tests/shaders/alloca-opts/float4-array-load.dxil
```

```text
Variable #11 struct#9 binding=(0,0):  _m0 @ 0: float32[24] stride=4
Variable #17 struct#15 binding=(0,0): _m0 @ 0: float32x4[6] stride=16
```

**同一个 cbuffer 在模块里同时存在两个视图**:标量视图(全部被动态标量索引访问)
和 vec4 视图(静态整向量访问),绑定完全相同。

## 4. 根因分析(全部基于本仓库 vendored 源码的实证)

### 4.1 标量视图从哪来

不是 type-propagation pass 的错。关键事实:

- `handleDclConstantBuffer`(dxbc-spirv/dxbc/dxbc_resources.cpp)创建的 cbuffer
  IR 类型**永远是** `Type(eUnknown, 4)[N]`(vec4 基);
- `setupArrayType()`(ir/passes/ir_pass_propagate_resource_types.cpp)输出
  `Type(scalarType, vectorSize=4)[N]`,即 **vec4 数组**,不会产出 `float[N]`;
- **DXIL 路径硬编码 `structuredCbv = false`**(dxil-spirv/bc/module_dxbc_ir.cpp),
  即 DXIL 的 cbuffer 一律走 vec4 数组路径。

stride-4 标量视图来自两个**独立的 cbuffer 拷贝提升机制**:

| 机制 | 位置 | 开关 |
|---|---|---|
| DXBC:`CleanupScratchPass::promoteScratchCbvCopy`(scratch → cbuffer 标量访问) | dxbc-spirv/ir/passes/ir_pass_scratch.cpp | 内部 `resolveCbvCopy`,默认 true,未暴露 |
| DXIL:alloca → CBV punchthrough(`analyze_alloca_cbv_forwarding_*`、`emit_gep_as_cbuffer_scalar_offset`) | dxil-spirv/opcodes/ | **无开关** |

触发模式:shader 把 cbuffer 数据拷进局部数组(alloca / 可索引临时寄存器),再
**动态索引**局部数组。此时上游把访问「提升/转发」为对 cbuffer 的**标量粒度**
访问,并产生一个 stride-4 的标量视图 buffer 与该 cbuffer 的规范 vec4 视图并存
(双视图,同一 DescriptorSet/Binding)。

### 4.2 spirv-cross2 为什么拒绝

spirv-cross2 HLSL 后端(`spirv_hlsl.cpp::emit_hlsl_cbuffer`)对每个 cbuffer 调用
`buffer_is_packing_standard(type, BufferPackingHLSLCbufferPackOffset)`
(spirv_glsl.cpp)。该模型把 HLSL cbuffer 视为 vec4 寄存器序列:

- 对 `float[N]` 数组,按 HLSL 打包规则计算出的 packed array stride 是 16
  (`(4 + 15) & ~15`,vec4 对齐),与 SPIR-V 的 ArrayStride 4 不符 → 拒绝。

这与「HLSL 其实能表达 `float arr[536]`(stride 4)」并不矛盾:spirv-cross2 的
打包模型是保守的,无法在一般情况下证明标量数组布局与 HLSL 编译器打包一致,
于是拒绝而非冒险。

## 5. 为什么在 rs 层做(而不是别处)

| 方案 | 评估 |
|---|---|
| **rs 层后处理 pass(本方案)** | 产物级修复,不碰 vendored C++;默认关闭、独立模块;惠及所有下游;回归测试直接复用 810 shader 套件 |
| 改上游 C++ | 真正根因,但两个机制一个无开关、一个有未暴露开关;关闭会改变所有消费方输出(Vulkan 鲁棒性场景);需要 fork 或上游 PR,周期不可控 |
| 改 spirv-cross2 HLSL 后端 | 单独不成立:双视图(同一 binding 两个 buffer)在 HLSL 里仍会产生重复 `register(b0)`;且需要 fork 维护 |
| spirv-tools 优化 pass | 不存在「cbuffer 布局规范化」类 pass |
| 应用层 | 每个下游重复造轮子;测试基建要从零搭 |

结论:**在 dxil-spirv crate 内做纯 SPIR-V 后处理,是当前约束下最合理的位置**。

## 6. 设计

### 6.1 API 与隔离

- feature:`non-upstream-hlsl-compat`(默认关闭;关闭时 `non_upstream` 模块不存在)
- 模块:`dxil_spirv::non_upstream::hlsl_compat`
- 入口:`vec4_align_cbuffers(&[u32]) -> Result<NormalizeOutput, HlslCompatError>`
- 错误类型独立(`HlslCompatError`),不污染上游 `dxil_spirv::Error`

### 6.2 算法

对每个「Uniform + Block 装饰、单成员 struct、成员为 32 位 stride-4 标量数组、
offset 0、长度 N 且 N%4==0」的 cbuffer 视图:

1. **双视图配对**:查找同 set/binding、总字节数相同的 vec4 视图(`float4[N/4]`
   stride 16)。找到 → **合并**(访问链重定向到 vec4 视图,删除标量视图变量);
   找不到 → **就地重排**(类型替换为 `float4[N/4]` stride 16)。
2. **访问链改写**:`[member, i]` → `[member, i/4, i%4]`。静态 i 折叠为常量;
   动态 i 在访问链前插入 `OpUDiv`/`OpUMod`。访问链结果类型不变(仍指向标量)。
3. **清理**:删除被合并的变量,及其 entry-point interface 项、装饰(OpDecorate)
   与名字(OpName),不留悬空引用。
4. **跳过规则**(任一命中即整个视图原样保留):N%4!=0、offset 非 0、向量加载
   消费标量数组访问链、被 OpStore/嵌套访问链/非语义调试指令引用、非 32 位
   类型、StorageBuffer 存储类。

布局字节不变(标量数组与 vec4 数组描述同一内存),因此变换是**语义中性**的。

## 7. 验证

### 7.1 全量 810 shader 扫描(`hlsl-compat-inspect scan`)

```text
total scanned: 810
shaders with rewritten cbuffer views: 11
GLSL regressions (ok -> fail): 0          ← 语义中性:pass 对全量零破坏
HLSL failures before: 191
HLSL failures after: 181
HLSL fixed by pass: 10                    ← cbuffer 布局类失败全部修复
```

剩余的 181 个 HLSL 失败属于其他类别(未支持的 builtin、WMMA 等),不在本 pass
范围内。GLSL 零回归是最强证据:对未命中的 shader pass 是逐字节不变的
no-op,对命中的 shader 语义等价(读同一块内存的同样字节)。

### 7.2 测试

- 单元测试(`dxil-spirv/tests/non_upstream_hlsl_compat.rs`,rspirv 构造模块,
  不依赖 C++ 转换):双视图合并、就地重排 + 静态索引拆分、幂等 no-op、非法输入
  拒绝。
- e2e 回归(`dxil-spirv-tests/tests/non_upstream/hlsl_compat_e2e.rs`):10 个目标
  shader 修复断言、alloca-opts 全目录无劣化、干净模块幂等。
- 全量语义中性:见 7.1。

运行方式(默认 `cargo test` 不包含这些测试):

```text
cargo test -p dxil-spirv --features non-upstream-hlsl-compat --test non_upstream_hlsl_compat
cargo test -p dxil-spirv-tests --features non-upstream-hlsl-compat --test non_upstream_hlsl_compat_e2e
```

## 8. 已知限制与未来

- 只处理「单成员 wrapper」形态(成员直接是 stride-4 数组、offset 0)。嵌套
  struct 里的 stride-4 数组、非 16 对齐 offset、多成员 struct 会被跳过。
- 只处理 32 位标量;16 位(min16float,stride 2)与 64 位(stride 8)跳过。
- 动态索引的向量加载(标量数组上)无法安全重写,跳过整个视图。
- 合并模式会删除标量视图变量;其死类型(旧 struct/array/pointer)保留在模块
  中(合法冗余,不影响任何消费方)。
- 未来若 spirv-cross2 暴露其他 HLSL 不兼容形态,在 `non_upstream` 模块内按
  同样模式扩展新函数,不改变本 pass 的契约。

## 9. 与上游的关系

- 上游 dxbc-spirv 的标量视图输出是**合法 std140**;本 pass 是对 HLSL 消费方
  的兼容性修复,不是对上游输出的纠错。
- 不修改 vendored C++(dxil-spirv / dxbc-spirv / dxil-spirv-sys 零改动)。
- 上游同步(`/sync-upstream`)不受影响:feature 关闭时本扩展完全不参与构建。
