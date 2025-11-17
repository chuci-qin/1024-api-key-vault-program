很好，这个目标现在已经很清晰了，我帮你把整个体系设计成一套 **“非托管的：用户专属 vault + 多个可撤销 API Key”** 架构。

我会分三层讲：

1. **整体架构图（用人话串一遍）**
2. **链上 program 设计：vault & api key 的数据结构 + 指令设计**
3. **链下 API Key 设计：怎么生成、怎么用、怎么作废，而且依然非托管**

---

## 1. 整体目标，用一句话概括

> **每个用户在 1024Chain 上有一个自己专属的 vault（由 program 控制），
> 用户可以创建 1～N 个 “API Key（其实就是代理公钥）”，
> 这些 API Key 可以在权限范围内操作该 vault 里的资金，
> API Key 可随时作废，
> 用户可以随时把 USDC 从自己的钱包转入 vault，也可以从 vault 提回到自己的钱包。
> 整个过程 1024 项目方不托管用户资金。**

这个目标可以拆成三个核心事实：

1. **资金只在两处：**

   * 用户自钱包（自托管）
   * program 控制的 vault（代码托管，不是公司托管）

2. **谁能动 vault 里的钱：**

   * 用户主钱包（owner）
   * 用户自己授权的一些“API Key（delegate 公钥）”，有权限 & 限额 & 可过期

3. **你们公司永远不持有这些 key，也不拥有任何能单方面挪用资金的权限**

---

## 2. 链上 program 设计：Vault + API Key 模型

### 2.1 账户（Account / PDA）结构

#### ① User Vault 账户（PDA）

* 地址：`VaultPDA(user_pubkey)`
* 拥有者：**你的 1024EX program**（不是用户，也不是你公司钱包）
* 数据字段大概这样（概念上）：

```text
struct UserVault {
    owner_wallet: Pubkey          // 用户主钱包地址
    usdc_token_account: Pubkey    // 这个 vault 对应的 USDC token account (PDA)
    delegates: Vec<Delegate>      // 授权的 API Key 列表
    total_deposit: u64            // 历史总存入
    free_collateral: u64          // 可用保证金
    locked_collateral: u64        // 挂单/仓位锁定保证金
    status_flags: u64             // 一些状态标记（是否冻结等）
}
```

> 是否“一个 vault 一个 token account”，还是“一个大池子 + 内部记账”，你可以二选一：
>
> * **纯 per-user 模式**：每个 vault 有自己的 USDC token account（最直觉，链上透明）；
> * **共享池模式**：所有资金在一个 GlobalPool USDC 账户里，UserVault 只做记账（更节省账户数量）。
>   从“非托管”角度，两者都是代码托管，只要 USDC token account 的 `owner` 是 program，而不是你们钱包。

---

#### ② Delegate / API Key 记录（存在 vault data 里）

```text
struct Delegate {
    key: Pubkey          // 这个 API Key 的公钥（链上视角）
    is_active: bool      // 是否有效
    permissions: u64     // 权限 bitmask，如 TRADE / WITHDRAW / CLOSE_ONLY
    max_notional: u64    // 最大允许使用的名义头寸 / 最大可用保证金
    expiry_slot: u64     // 过期 slot 或 timestamp
    nonce: u64           // 防重放的序号（可选）
}
```

* **key** 就是这个 API Key 对应的 Ed25519 公钥。
* 你可以支持多个 delegate，让一个 vault 有多个策略（网格、CTA、Arb）各自一个 key。

---

### 2.2 关键指令（Instruction）设计

你大概需要这些指令：

#### 1）初始化 vault

* `InitVault {}`

  * signer：`owner_wallet`（用户主钱包）
  * 逻辑：

    * 用 `seeds = ["vault", owner_wallet]` 创建 `VaultPDA`
    * 创建/绑定 `vault_usdc_token_account`（`Token Account` 的 owner 为 program 或 PDA）
    * 初始化 `UserVault` 结构

#### 2）存入 USDC 到 vault

* `Deposit { amount }`

  * signer：`owner_wallet`
  * 账户：用户的 USDC ATA、vault 的 USDC token account
  * 逻辑：

    * 使用 SPL Token `TransferChecked`，从 `user_ata` → `vault_usdc_token_account`
    * 更新 `UserVault.total_deposit` 和 `free_collateral`

> **注意：**
>
> * 这一步永远只能用户自己签，证明“我自愿把 USDC 存入这个 vault（由代码托管）”。

#### 3）从 vault 提出 USDC 回钱包

* `Withdraw { amount }`

  * signer：**owner_wallet 或某个有效 delegate.key**
  * 逻辑：

    * 检查：

      * vault 的 `free_collateral` 是否足够
      * 若 signer 是 delegate，再检查权限 & 限额（是否允许 withdraw）
    * 使用 SPL Token `TransferChecked` 从 `vault_usdc_token_account` → `user_ata`
    * 更新 `free_collateral`

> 实际永续场景中，你会限制：
>
> * 有未平仓位 & 挂单时，不能把保证金全部提走，只能提“可用保证金”。

#### 4）添加 API Key（on-chain 授权代理）

* `AddDelegate { delegate_pubkey, permissions, max_notional, expiry_slot }`

  * signer：**owner_wallet**（必须用主钱包签）
  * 逻辑：

    * 在 `UserVault.delegates` 中插入一条记录
    * 若已存在则更新参数

> 这里的 `delegate_pubkey` 就是“这个 API Key 对应的公钥”。

#### 5）撤销 / 作废 API Key

* `RevokeDelegate { delegate_pubkey }`

  * signer：**owner_wallet**
  * 逻辑：

    * 找到 `UserVault.delegates` 中对应记录，把 `is_active = false`
    * 或直接删除这条记录

> 从这一刻开始，任何用该 API Key 签名的交易，进 program 时都会被 auth 逻辑拒绝，**彻底失效。**

#### 6）交易相关指令（开仓 / 平仓 / 调整仓位）

例如：

* `PlaceOrder { side, size, price, … }`
* `CancelOrder { order_id }`
* `AdjustLeverage { new_leverage }`

统一认证逻辑：

```text
fn assert_vault_authority(vault: &UserVault, signer: Pubkey, action: Action) {
    if signer == vault.owner_wallet {
        // 直接通过
        return;
    }

    // 否则必须是 delegate
    let delegate = vault.delegates.find(d => d.key == signer && d.is_active);
    assert!(delegate.exists, "NOT_AUTHORIZED_DELEGATE");

    // 检查是否过期
    assert!(current_slot <= delegate.expiry_slot, "DELEGATE_EXPIRED");

    // 检查权限 bitmask
    assert!(delegate.permissions.contains(action), "NO_PERMISSION_FOR_ACTION");

    // 检查风险/限额（可选）
}
```

> 这样就达到了你要的：
>
> * 一个 vault 对应多个 API Key（delegate），
> * 每个可以有不同权限和最大风险敞口，
> * 随时可以撤销。

---

## 3. 链下 API Key 设计：怎么生成和使用，才保证非托管？

注意：**在这个设计里，链上的 “API Key” 实际就是一个 Ed25519 公钥；
链下的 “API 密钥字符串” 对应它的私钥。**

### 3.1 API Key 生成

推荐两种方式（都保持“只在用户环境生成和保存”）：

1. **用户本地生成**

   * 用 `solana-keygen new` 或 1024 的 `quant1024 keygen`
   * 得到一对 `(pubkey, secret_key)`
   * `pubkey` 填到 UI / SDK 中，调用 `AddDelegate` 指令
   * `secret_key` 只保存在用户机器 / 自己的 Railway 项目 env 里

2. **你的 SDK 帮用户生成**

   * 在用户本地执行：

     ```python
     from quant1024.keys import generate_api_key
     pub, secret = generate_api_key()
     ```

   * SDK 自动发起一个 `AddDelegate` 交易（让用户主钱包签名）

   * 同时把 secret 写入 `.env` 或 keyfile，只存在用户本地/用户云环境（你看不到）

**关键点：**

> 无论哪种方式，**你们的服务器从来不接触到这个私钥**，也不会存下来。

---

### 3.2 API Key 实际怎么“操作 vault 里的资金”？

流程：

1. 你提供一个 SDK，比如 `quant1024-1024chain`：

   ```python
   client = VaultClient(
       rpc_url="https://rpc.1024chain.xyz",
       program_id="1024EXProg...",
       api_key_secret="xxx",        # 本地存的私钥
       api_key_pubkey="Pubkey",     # 对应的公钥
       owner_wallet="UserMainWalletPubkey",
   )
   ```

2. 用户的策略进程（运行在他自己的 Railway / VPS / 本地）：

   * 用 `api_key_secret` 对交易进行签名；
   * 通过 RPC 调用你的 program 指令：

     * `PlaceOrder, CancelOrder, Withdraw, ...`

3. Program 收到交易：

   * 根据 `signer_pubkey`（就是 delegate.key）在 `UserVault.delegates` 里做权限/限额/过期检查；
   * 合法就执行操作，修改 vault 里的资金状态。

> **整个过程你们公司从未为用户签名，也未拿到能签名的私钥。**
> 你做的是：
>
> * 提供 SDK + RPC + 工具
> * 由用户自己“在自己的 compute 环境里”拿着 API Key 对资金进行操作。

---

### 3.3 API Key 作废流程

1. 用户在 UI / CLI 里选择一个 API Key（delegate_pubkey）
2. 用主钱包签一笔交易调用 `RevokeDelegate { delegate_pubkey }`
3. Program 更新 `UserVault.delegates`：`is_active = false`
4. 从这一刻起：

   * 所有用这个私钥签的新交易都会被拒绝
   * 旧的交易若还没上链，打包之后也会因为检查失败被 revert

可选强化措施：

* 在 `Delegate` 中维护一个 `nonce`，每笔交易携带 `nonce`，并在执行时 `nonce += 1`，避免旧交易被重放。
* Revoke 时也可以把 `nonce` 设置成一个巨大值（例如 `u64::MAX`），让旧交易全部失效。

---

## 4. 用户视角完整流程（User Story）

用一个“Xavier 创建 vault + 两个策略 API Key”的故事串起来：

### Step 1：开 vault

* Xavier 用 1024EX Web App 连接 1024Chain 钱包（Phantom 等）
* 点击 “创建专属交易金库（Vault）”
* 前端发起 `InitVault` 交易，钱包弹窗，Xavier 签名
* 链上生成：

  * `VaultPDA(Xavier)` + 对应 USDC token account
  * `UserVault{ owner_wallet = Xavier, delegates = [] }`

### Step 2：转入 USDC

* Xavier 在 UI 上点击 “存入 USDC 到 Vault”，输入 10,000 USDC
* 钱包发起 `Deposit{10000}`，从他的 USDC ATA 转到 `vault_usdc_token_account`
* Program 更新 `free_collateral = 10,000`

### Step 3：创建第一个 API Key：Grid Bot

* Xavier 在自己的电脑上运行：

  ```bash
  quant1024 api-key new --name grid-bot
  ```

  得到：

  * `pubkey = Gxxxxxx`
  * `secret = ...`（写进 `.env`，你们服务端看不到）

* 前端或 CLI 调用：

  ```text
  AddDelegate {
     delegate_pubkey = Gxxxxxx,
     permissions = TRADE | CLOSE_ONLY,
     max_notional = 5000,
     expiry_slot = now + 30 days
  }
  ```

  由 Xavier 主钱包签名提交。

* 链上 vault 现在有了一个 delegate：Grid Bot。

### Step 4：Grid Bot 自动操作 vault

* Xavier 把 `secret` 配在自己 Railway 项目或本地 Python 进程里，策略代码大致：

  ```python
  client = VaultClient(api_key_secret="...", api_key_pubkey="Gxxxxxx", owner_wallet="Xavier")

  # 策略逻辑
  while True:
      signal = compute_signal()
      if signal.should_long:
          client.place_order(market="BTC-PERP", side="long", size=1.0, price=..., ...)
  ```

* 每次 `place_order` 都会：

  * 用 `api_key_secret` 对交易签名；
  * 调用 `PlaceOrder`；
  * program 检查 signer 是否是有效 delegate（Gxxxxxx），且权限/限额/过期均 OK；
  * 然后在 vault 内部调整仓位/锁定保证金。

### Step 5：撤销这个 API Key

* 某天 Xavier 不想让 Grid Bot 控制这个 vault 了：

  * 打开 UI → “API Key 管理” → 选择 Grid Bot → 点击“作废”
  * 钱包签名 `RevokeDelegate{delegate_pubkey = Gxxxxxx}`

* 从此：

  * `UserVault.delegates` 中这条记录 `is_active = false`
  * 任何用 `Gxxxxxx` 签名的交易都会被拒绝

（若他在 Rails 项目中忘记删 env，也无所谓，只是“已经没权限”的一串无用 key）

### Step 6：随时从 vault 提回 USDC

* Xavier 在 UI 上点击 “提现”：

  * 选择 3,000 USDC
  * 钱包弹出 `Withdraw{3000}` 交易（可以要求必须主钱包签名；也可以允许有 `WITHDRAW` 权限的 delegate，但提现给主钱包）
* Program 检查：

  * `free_collateral >= 3000`
  * 仓位风险在安全范围内
* 调用 SPL Token，把 USDC 从 `vault_usdc_token_account` 转回 Xavier 的 USDC ATA

---

## 5. 非托管（No Custody）结论（你可以直接复用）

在上述设计里，**你实现了想要的功能，又保持了完整非托管：**

1. **资金只在：**

   * 用户自托管钱包
   * 不可升级的合约控制的 vault（PDA）

2. **项目方/团队没有任何“能单方面动用资金”的私钥或权限：**

   * Vault & USDC token account 的 owner 都是 program / PDA，不是你们钱包。
   * 没有“admin withdraw / emergency transfer”之类后门函数。

3. **API Key 是用户自己生成、自己保存的 delegate 公钥 + 私钥：**

   * 链上只记录公钥 + 权限；
   * 私钥永远只存在用户自己的 compute 环境里（本地、自己的 Railway 账户等）。

4. **项目方提供的只是：**

   * 链上合约（代码托管资金）
   * SDK / API（生成交易、回测、信号等）
   * 用户自己在自己的环境里拿 API Key 对交易做签名。

### 换句话说：

> **资金托管在“不可篡改的代码里”，而不是托管在“你们公司”手里。
> 你们不具备 unilateral control，因而不构成传统意义上的“资金托管方”。**

