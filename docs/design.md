下面是一份可以直接丢进仓库 `1024-api-key-vault-program` 的设计文档草案，粒度比较细，也把**产品/商业价值**写进去了。你可以保存成：

> `docs/1024-vault-api-key-design.md`

---

# 1024 Vault & API Key Program 设计文档

*1024-api-key-vault-program*

## 0. 文档目的

本设计文档面向：

* **链上开发者**（Rust / Solana / 1024Chain）
* **量化 / 产品 / 合规 / 商业团队**
* **智能代理 / SDK（quant1024）集成开发者**

目标是定义一套在 1024Chain 上的 **非托管用户金库（Vault）+ 多 API Key 授权体系**，用于支持：

* 1024EX 永续合约交易所的 **保证金托管**
* 1024Quant / quant1024 策略的 **API 自动交易**
* 与 1024 生态其他衍生品协议的 **统一资金层**

Program 名称：`1024-api-key-vault-program`
GitHub 仓库：`https://github.com/.../1024-api-key-vault-program`（占位）

---

## 1. 背景 & 商业目标

### 1.1 问题背景

1024 生态要同时满足：

1. **非托管（Non-Custodial）**

   * 项目方不持有用户私钥
   * 项目方不能单方面动用用户资金
   * 资金由不可升级的智能合约逻辑控制

2. **量化友好 / API 友好**

   * 用户可以像 CEX 一样，为每个策略生成一个 API Key
   * 策略可以 7×24 自动交易、调仓、风险控制
   * API Key 可以随时修改、撤销和限权

3. **统一资金层**

   * 同一份保证金可以在 1024EX、未来的期权/结构性产品等协议中复用
   * 避免用户到处分散资金，提高资金利用率和产品粘性

### 1.2 商业价值

设计 `Vault + API Key` 的直接商业收益：

* ✅ **量化机构 & Pro 用户的必备能力**

  * 多 Key，多策略，多账户风控
  * 每个 Key 可配置限额、权限、过期时间
* ✅ **合规叙事清晰：非托管协议，而非托管机构**

  * 资金由不可升级合约控制
  * 项目方没有“后台动资金”的能力
  * 方便对接合规交易所 / 机构资本
* ✅ **为 1024Quant 打造“策略即资产”的基础设施**

  * 每个策略 → 一个 API Key
  * Vault → “资金母账户”
  * 将来策略可以被打包成指数 / NFT / vault 份额

---

## 2. 设计原则

1. **绝对非托管**

   * Program 不持有任何私钥
   * 所有资金在链上由 PDA + 合约逻辑控制
   * Program 无“管理员提币 / 后门转账 / 紧急转移资金”指令

2. **单向授权、可撤销**

   * 用户主钱包是 Vault 的最终 owner
   * API Key 代表的 delegate 仅拥有被授予的权限
   * 用户可以随时撤销、修改 API Key

3. **多 API Key，多策略**

   * 一个 Vault 可以挂多个 API Key（delegate）
   * 不同 Key 可设不同：

     * 权限（Trade / Withdraw / Close only…）
     * 风险限额（max_notional / max_withdrawable）
     * 有效期（expiry_slot）

4. **可组合 / 可扩展**

   * Vault 层只负责“资金 & 授权 & 风控框架”
   * 具体业务（永续交易、期权、结构性产品）由其他 Program 实现
   * 其他 Program 通过 CPI 调用 Vault Program 做：

     * 保证金锁定 / 解锁
     * PnL 结算
     * 权限校验

---

## 3. 核心概念

### 3.1 用户主钱包（Owner Wallet）

* 一个普通的 1024Chain 地址（Ed25519 公钥）
* 代表用户本人
* 只用于：

  * 创建 Vault
  * 初次存入/提取资金
  * 添加 / 更新 / 撤销 API Key 授权

**商业面向：**

* 用户在 1024EX / 1024Quant UI 连接的钱包就是 Owner Wallet

---

### 3.2 用户 Vault（User Vault）

* 一个由 program 控制的 PDA 账户，记录：

  * 该用户的总保证金 / 可用保证金
  * 挂单 / 仓位锁定保证金
  * 授权给哪些 API Key（delegate）

特点：

* Owner Wallet 对 Vault 拥有最终最高权限（通过 program 逻辑）
* Vault 本身没有私钥，只能在 program 逻辑中被修改

**商业面向：**

* UI 文案：

  > “你的 1024 专属量化金库（Vault），资金只受合约约束，我们无法挪用。”

---

### 3.3 API Key / Delegate

在链上视角：

* 一个 `delegate_pubkey: Pubkey` 对应一个 API Key
* 每个 delegate 由一个单独的 PDA 记录其授权信息（Delegate Account）

字段概念（逻辑）：

* `key`: delegate 公钥
* `is_active`: 是否生效
* `permissions`: 权限 bitmask（例如：Trade / Withdraw / CloseOnly）
* `max_notional`: 最大可用名义敞口
* `used_notional`: 当前使用中的敞口（由业务方更新）
* `expiry_slot`: 过期 slot（过期自动失效）
* `nonce`: 防重放 / 防双花计数器

**链下视角（用户体验）：**

* 用户在 quant1024 / 1024Quant 控制台创建“策略 API Key”时：

  * 本地生成一对 (pubkey, secret_key)
  * 使用 Owner Wallet 签名一笔 on-chain 交易，把 `pubkey` 加入 Vault 的 delegate 列表
  * secret_key 保存在用户自己的运行环境里，用来为交易签名

**商业面向：**

* UI 可以展示为：

  > * Grid 机器人（API Key A）：允许交易 BTC-PERP，最大 10,000 USDC
  > * Trend 机器人（API Key B）：允许交易 ETH-PERP，最大 5,000 USDC，Close-only

---

### 3.4 Token & 资产支持范围

第一期：

* 只支持一个稳定币：**USDC（1024Chain 上的官方 USDC Mint）**
* 后续可扩展多资产版本（USDT、N1024、wBTC等）

USDC Mint 地址将在 `GlobalConfig` 中配置，方便：

* Devnet / Testnet / Mainnet 环境不同 Mint
* 未来替换 / 停用某个 USDC 版本

---

## 4. 链上架构设计

### 4.1 PDA & Account 总览

主要账户类型：

1. `GlobalConfig`（全局配置单例）
2. `UserVault`（用户金库账号）
3. `VaultTokenAccount`（Vault 对应的 USDC Token Account）
4. `DelegateAccount`（每个 Vault × API Key 一条记录）

#### 4.1.1 GlobalConfig

* PDA：`["global", version]`
* 字段（概念）：

  * `version: u8`
  * `admin: Pubkey`（可选，初始化后可设为 `Pubkey::default()` 放弃管理）
  * `usdc_mint: Pubkey`
  * `bump: u8`

**注意：**

* 如果要追求极致非托管，可以在正式网络上线前：

  * 通过一笔交易将 `admin` 置为 `Pubkey::default()`
  * 或引入多签 + timelock
* 需要和法律团队协同决定升级策略

---

#### 4.1.2 UserVault

* PDA：`["vault", owner_wallet]`
* 字段（逻辑）：

  * `owner: Pubkey`            // Owner Wallet
  * `usdc_vault: Pubkey`       // Vault USDC Token Account
  * `bump: u8`
  * `total_deposit: u64`
  * `total_withdrawn: u64`
  * `free_collateral: u64`
  * `locked_collateral: u64`   // 挂单/仓位锁定保证金（由业务程序更新）
  * `flags: u64`               // 冻结等状态
  * `reserved: [u8; N]`        // 预留字段方便后续扩展

> **Note:**
>
> * `free_collateral + locked_collateral <= usdc_vault 里实际余额`
> * 真实余额由 Token Program 维护，Vault 只记录逻辑状态

---

#### 4.1.3 VaultTokenAccount（USDC 账户）

* PDA：`["vault-usdc", owner_wallet]`
* 类型：SPL Token Account

特点：

* `owner` 字段为 `UserVault` program id 或其 PDA，无法由项目方钱包控制
* Program 调用 SPL Token Program 完成转账和结算

---

#### 4.1.4 DelegateAccount（API Key 设置）

* PDA：`["delegate", owner_wallet, delegate_pubkey]`

字段（逻辑）：

* `owner: Pubkey`            // Vault owner
* `vault: Pubkey`            // 对应 UserVault PDA
* `delegate: Pubkey`         // API Key 对应的公钥
* `is_active: bool`
* `permissions: u64`         // bitmask
* `max_notional: u64`
* `used_notional: u64`       // 当前已分配的名义敞口
* `expiry_slot: u64`
* `nonce: u64`               // 防重放计数器
* `bump: u8`
* `reserved: [u8; M]`

**权限 bitmask 预设：**

* `1 << 0`：`PERM_TRADE` — 允许下单 / 调整仓位
* `1 << 1`：`PERM_WITHDRAW` — 允许从 Vault 提现到 owner 钱包
* `1 << 2`：`PERM_CLOSE_ONLY` — 只允许减仓 / 平仓（不增加净敞口）
* `1 << 3`：`PERM_VIEW_ONLY` — 只读（未来扩展）

---

### 4.2 指令设计（Instruction）

#### 4.2.1 初始化全局配置

`InitializeGlobalConfig { usdc_mint, admin }`

* 只允许部署时调用一次
* 由部署脚本 / 多签账户发起
* 设置 `GlobalConfig`

> 正式环境可以后续通过特殊流程将 `admin` 置空，锁死配置。

---

#### 4.2.2 创建用户 Vault

`CreateVault {}`

* Signer：`owner_wallet`（用户主钱包）
* 账户：

  * `GlobalConfig`
  * `UserVault`（PDA，创建）
  * `VaultTokenAccount`（PDA，创建）
  * `System Program`
  * `Token Program`
* 行为：

  * 使用 `["vault", owner_wallet]` 派生并创建 `UserVault`
  * 使用 `["vault-usdc", owner_wallet]` 创建 USDC Token Account，owner 为 program
  * 初始化 `UserVault`：

    * `owner = owner_wallet`
    * `usdc_vault = vault_usdc_ata`
    * `total_deposit = 0`
    * `free_collateral = 0`

**商业点：**

* UI 文案：“一键创建你的 1024 专属量化金库（Vault）”

---

#### 4.2.3 存款：用户钱包 → Vault

`Deposit { amount: u64 }`

* Signer：`owner_wallet`
* 账户：

  * `GlobalConfig`
  * `UserVault`
  * `owner_usdc_ata`（用户自己的 USDC ATA）
  * `vault_usdc_ata`（Vault 的 USDC token account）
  * `Token Program`
* 行为：

  * 从 `owner_usdc_ata` 向 `vault_usdc_ata` 执行 SPL Token `TransferChecked`
  * 更新：

    * `total_deposit += amount`
    * `free_collateral += amount`

**UX：**

* Web 端展示：

  * “你正在将 X USDC 存入 1024 Vault，由智能合约托管，用于永续合约保证金。”

---

#### 4.2.4 提款：Vault → 用户钱包

`Withdraw { amount: u64 }`

* Signer：**`owner_wallet` 或某个允许 `PERM_WITHDRAW` 的 delegate**

* 账户：

  * `GlobalConfig`
  * `UserVault`
  * `DelegateAccount`（可选，如果 signer 是 delegate）
  * `owner_usdc_ata`
  * `vault_usdc_ata`
  * `Token Program`

* 校验逻辑：

  1. 如果 `signer == owner_wallet`：

     * 直接通过（仍需检查风险逻辑，例如 `amount <= free_collateral`）
  2. 否则：

     * 读取 `DelegateAccount`
     * 检查：

       * `is_active == true`
       * 当前 slot ≤ `expiry_slot`
       * `permissions` 包含 `PERM_WITHDRAW`
       * 风险模块允许（如不得在有大额净敞口时全额提现）

* 行为：

  * 从 `vault_usdc_ata` 转 `amount` 到 `owner_usdc_ata`
  * `free_collateral -= amount`
  * `total_withdrawn += amount`

**商业点：**

* “机器人可以按你设置的规则帮你提现（例如盈利自动部分提现），你任何时候可以关掉这个权限。”

---

#### 4.2.5 添加 / 更新 API Key（Delegate）

`UpsertDelegate { delegate_pubkey, permissions, max_notional, expiry_slot }`

* Signer：`owner_wallet`
* 账户：

  * `UserVault`
  * `DelegateAccount`（PDA，如果不存在则创建）
  * `System Program`
* 行为：

  * 如果 `DelegateAccount` 不存在：创建一个
  * 设置 / 更新：

    * `delegate = delegate_pubkey`
    * `permissions`
    * `max_notional`
    * `expiry_slot`
    * `is_active = true`
    * 如果是更新保留 `used_notional`，或按策略重置
* 约束：

  * `expiry_slot` 不得太远（比如最长 1 年，防止永久授权）
  * 可以加入限制：`max_notional <= free_collateral * K`

**商业文案：**

* “你正在为策略 【Grid-BTC】 创建一个 API Key：

  * 最大可用保证金：10,000 USDC
  * 允许操作：开平仓
  * 到期时间：30 天后”

---

#### 4.2.6 撤销 API Key（Delegate）

`RevokeDelegate { delegate_pubkey }`

* Signer：`owner_wallet`
* 账户：

  * `UserVault`
  * `DelegateAccount`
* 行为：

  * 设置 `is_active = false`
  * 可选项：同时重置 `max_notional = 0`、`used_notional = 0`

**效果：**

* 所有后续使用该 `delegate_pubkey` 作为 signer 的交易，在进入业务程序时，通过 CPI 调用 Vault Program 做校验时都会失败

**商业文案：**

* “你已停止策略【Grid-BTC】对金库的访问，该 API Key 即刻失效。”

---

#### 4.2.7 业务方使用的“锁定保证金 & 更新敞口”

这些指令主要被 **永续合约 Program / 其他业务 Program** 调用，用户一般不直接调用。

**示意：**

1. `LockMarginForTrade { owner_wallet, required_notional }`
2. `UnlockMarginAndUpdatePnl { owner_wallet, pnl_delta, notional_delta }`

* Signer：业务 Program（通过 CPI 调用），但仍需用户或 delegate 签名原始交易
* 账户：

  * `UserVault`
  * `DelegateAccount`（若使用 delegate）
  * `GlobalConfig`
  * 调用程序自身账户

校验逻辑（概念）：

* 确认本次交易的 signer：

  * 要么是 `owner_wallet`
  * 要么是一个合法、未过期、权限涵盖 `PERM_TRADE` 的 delegate
* 若 `LockMarginForTrade`：

  * 检查 `free_collateral` 是否足够
  * 检查 `used_notional + required_notional <= max_notional`
  * 更新：

    * `free_collateral -= locked_amount`
    * `locked_collateral += locked_amount`
    * `used_notional += required_notional`
* 若 `UnlockMarginAndUpdatePnl`：

  * 根据业务逻辑传入的 `pnl_delta`：

    * 若盈利则 `free_collateral += pnl_delta`
    * 若亏损则减少保证金
  * 根据 `notional_delta` 更新 `used_notional` 和 `locked_collateral`

**这部分是 Vault Program 与 1024EX Perp Program 的交界，属于 Phase 2/3 扩展。**
第一阶段可以只实现：Deposit / Withdraw / Delegate 管理，后续迭代接入真正的永续撮合程序。

---

## 5. 关键交互流程（User Story 视角）

### 5.1 创建 Vault + 存入 USDC

1. 用户在 1024EX Web → 点击 “创建我的金库（Vault）”
2. 钱包弹出 `CreateVault` 交易 → 用户签名
3. 创建成功后 UI 显示 Vault 地址，余额为 0
4. 用户在 UI 输入 “存入 10,000 USDC”
5. 钱包弹出 `Deposit{10000}` → 用户签名
6. 链上：

   * User USDC ATA → Vault USDC ATA
   * `free_collateral = 10,000`

---

### 5.2 创建一个策略 API Key

1. 用户在 1024Quant 控制台点击 “新建策略 API Key”
2. 在本地（或用户自己的 Railway / VPS）生成一对 keypair：

   * `delegate_pubkey`
   * `delegate_secret`
3. 前端要求用户用钱包签名一笔 `UpsertDelegate`：

   * 参数：

     * `delegate_pubkey`
     * `permissions = PERM_TRADE`
     * `max_notional = 8,000`
     * `expiry_slot = now + 30 days`
4. 链上创建 / 更新 `DelegateAccount`
5. 用户将 `delegate_secret` 配置到 quant1024 策略环境变量中

---

### 5.3 策略自动交易（简化）

1. 策略代码（运行在用户自己的环境里）：

   * 监听行情
   * 决定要下单时，构造永续合约 Program 的 `PlaceOrder` 指令
   * 使用 `delegate_secret` 作为签名者之一签名交易
2. 永续 Program 在处理 `PlaceOrder` 前：

   * CPI 调用 Vault Program：

     * `LockMarginForTrade{owner_wallet, required_notional}`
   * Vault Program 检查：

     * 是否存在 `DelegateAccount(owner, delegate_pubkey)`
     * 是否 `is_active == true && current_slot <= expiry_slot`
     * 是否 `permissions` 允许交易
     * 是否 `used_notional + required_notional <= max_notional`
   * 成功后锁定保证金，并返回 OK
3. 永续 Program 继续执行下单逻辑

---

### 5.4 作废某个 API Key

1. 用户在 1024Quant 控制台找到某个 API Key → 点击 “作废”
2. 钱包弹出 `RevokeDelegate{delegate_pubkey}` → 用户签名
3. 之后所有用该 `delegate_pubkey` 签的交易，在进入业务 Program 时经 Vault Program 校验会被拒绝

---

### 5.5 从 Vault 提回 USDC

1. 用户在 UI 点击 “提现 3,000 USDC”
2. 钱包弹出 `Withdraw{3000}`，由 Owner Wallet 签名
3. 链上：

   * Vault Program 确保 `free_collateral >= 3000` 且没违反风控
   * 转账：Vault USDC ATA → User USDC ATA
   * 更新 `free_collateral -= 3000`

---

## 6. 安全模型 & 非托管说明

### 6.1 非托管核心要点

1. **项目方不持有任何能直接动用户资金的私钥**
2. **Vault USDC Token Account 的 owner 为 program / PDA，而非项目方钱包**
3. **Program 无管理员提币 / 后门指令**
4. **合约不可升级（推荐）或使用严格的多签 + timelock 升级机制**
5. **就算 1024 后端、网站、API 全部被攻破，攻击者仍无法直接盗取链上的用户资产**

### 6.2 风险场景分析

* **Delegate 私钥泄露：**

  * 攻击者可以在其权限和限额范围内操作 Vault
  * 用户依旧可以用 Owner Wallet 立即 `RevokeDelegate`，完全吊销权限
* **Owner Wallet 私钥泄露：**

  * 这是传统钱包风险，不属于协议层面托管问题
* **Program 存在逻辑 bug：**

  * 可能导致某些情况下错算敞口 / 无法正确限制风险
  * 需要严格审计与测试（见后文测试计划）

---

## 7. 实现计划 & 进度管理（给 Cursor / Dev）

### 7.1 目录结构建议

```text
1024-api-key-vault-program/
  ├─ programs/
  │   └─ vault/
  │       ├─ src/
  │       │   ├─ lib.rs
  │       │   ├─ instruction.rs
  │       │   ├─ state.rs
  │       │   ├─ error.rs
  │       │   └─ utils.rs
  │       └─ Cargo.toml
  ├─ tests/
  │   ├─ vault_basic_flow.rs
  │   └─ delegate_permissions.rs
  ├─ docs/
  │   ├─ 1024-vault-api-key-design.md
  │   └─ security-model.md
  ├─ Anchor.toml / Cargo.toml
  ├─ README.md
  └─ LICENSE
```

> 是否使用 Anchor 由实现阶段决定；设计文档本身不依赖特定框架。

---

### 7.2 开发里程碑

**M0 – 仓库初始化（0.5 天）**

* [ ] 初始化 Git 仓库 & 基础目录结构
* [ ] 添加 README（项目简介、非托管原则、Roadmap）
* [ ] 添加 LICENSE（建议 MIT 或 Apache-2.0）

**M1 – GlobalConfig & Vault 基础（1–2 天）**

* [ ] 实现 `GlobalConfig` 账户
* [ ] 实现 `CreateVault` 指令
* [ ] 实现 `Deposit` / `Withdraw`（仅 Owner Wallet 允许）
* [ ] 单元测试：

  * [ ] 创建 Vault 正常流程
  * [ ] 多次存入 / 提出
  * [ ] 余额不足时的失败情况

**M2 – Delegate / API Key 模块（3–4 天）**

* [ ] 实现 `DelegateAccount` 账户结构
* [ ] 实现 `UpsertDelegate` 指令
* [ ] 实现 `RevokeDelegate` 指令
* [ ] 在 `Withdraw` 中支持 delegate 校验逻辑（`PERM_WITHDRAW`）
* [ ] 单元测试：

  * [ ] 创建 Delegate
  * [ ] 过期 / 撤销后的失败场景
  * [ ] 权限不足（没有 WITHDRAW 权限）时的失败

**M3 – 与业务程序的 Margin Lock 接口（5+ 天）**

* [ ] 设计并实现 `LockMarginForTrade` / `UnlockMarginAndUpdatePnl`
* [ ] 提供给永续合约 Program 的 CPI 示例
* [ ] 设计 `max_notional/used_notional` 更新规则
* [ ] 集成测试（localnet + 简化的永续 Program mock）

**M4 – 文档 & 安全审计准备（持续）**

* [ ] 完成 `docs/security-model.md`
* [ ] 总结公开文档给社区开发者使用
* [ ] 为审计准备：账户图、指令图、边界条件说明

---

## 8. 测试与质量保证

### 8.1 单元测试（Rust 层）

* 各指令的 happy path 和失败场景：

  * 创建 Vault
  * 存入 / 提取
  * 添加 / 更新 / 撤销 Delegate
  * Delegate 过期 / 权限不足 / 超出额度的场景

### 8.2 集成测试（localnet）

* 用 `solana-test-validator` 或 1024Chain 的本地 validator：

  * 启动 Vault Program
  * 模拟真实钱包地址（owner）与几个 delegate
  * 模拟业务 Program 调用 Margin Lock 接口
* 覆盖场景：

  * 多个策略同时操作一个 Vault
  * 错误 / 恶意参数下系统的行为

### 8.3 Fuzz / 属性测试（可选）

* 使用 proptest / fuzzing 对下列性质做检查：

  * `free_collateral + locked_collateral` 不超过真实余额
  * 在任何操作序列下，不出现负数 / 溢出
  * 不存在无签名就能移动资金的路径

### 8.4 安全审计准备

* 输出：

  * 账户交互图
  * 指令流程图
  * 所有 `unsafe` / `unchecked` 逻辑清单

---

## 9. 对外 README / 商业版本摘要（可放 README）

> **1024 Vault & API Key Program** 是运行在 1024Chain（Solana/Agave 兼容）的一个开源智能合约，用于为 1024 生态提供：
>
> * 用户专属的链上金库（Vault），由不可升级的智能合约托管
> * 每个 Vault 可以绑定多个可撤销的 API Key（Delegate），支持精细化授权和限额控制
> * 为 1024EX 永续合约、1024Quant 策略平台、未来的期权 / 结构化产品提供统一资金层
>
> **我们承诺：**
>
> * 任何项目方私钥都无法单方面挪用用户的资金
> * 所有资金只在两处出现：用户自托管钱包 & 智能合约控制的 Vault
> * API Key 仅由用户自己生成和保存，我们从不托管用户的私钥
>
> 这使得 1024 能够同时提供：
>
> * 接近 CEX 的量化交易体验（多 API Key、多策略、多账户风控）
> * 严格的非托管安全模型（Smart-Contract Non-Custodial）

---

如果你愿意，下一步我可以帮你：

* 把上面拆成 **README.md 初稿** + **docs/security-model.md 初稿**
* 或者直接为 `programs/vault/src/state.rs` / `instruction.rs` 写一个“只有结构体和枚举签名”的骨架（不写具体业务逻辑），方便你在 Cursor 里一键补全实现。
