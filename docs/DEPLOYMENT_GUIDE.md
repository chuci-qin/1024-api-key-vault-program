# 1024 API Key Vault Program - 部署和使用完整指南

**版本**: v0.1.1  
**部署日期**: 2025-11-17  
**网络**: 1024Chain Testnet  
**状态**: ✅ 已部署并验证

---

## 📋 目录

1. [概述](#概述)
2. [部署信息](#部署信息)
3. [架构设计](#架构设计)
4. [快速开始](#快速开始)
5. [完整使用流程](#完整使用流程)
6. [API 参考](#api-参考)
7. [常见问题](#常见问题)

---

## 概述

### 什么是 1024 API Key Vault Program?

1024 API Key Vault Program 是一个部署在 1024Chain 上的**完全非托管**智能合约，为 1024EX 永续合约交易所和 1024Quant 量化平台提供：

- ✅ **用户专属金库 (Vault)** - 每个用户拥有独立的链上金库存储 USDC
- ✅ **多 API Key 授权** - 支持创建多个 API Key，每个策略独立授权
- ✅ **精细权限控制** - 交易、提现、只平仓等多种权限组合
- ✅ **风险限额管理** - 每个 API Key 独立的敞口限额
- ✅ **可撤销授权** - 随时作废任何 API Key，立即生效

### 核心优势

1. **完全非托管**
   - 项目方不持有用户私钥
   - 资金由智能合约控制，无后门
   - 可通过 `RenounceAdmin` 实现完全去中心化

2. **量化友好**
   - 类似 CEX 的 API Key 体验
   - 支持 7×24 自动交易
   - 多策略隔离，风险可控

3. **统一资金层**
   - 一份保证金可用于多个产品
   - 避免资金分散，提高利用率

---

## 部署信息

### 网络配置

```yaml
网络: 1024Chain Testnet
RPC URL: https://testnet-rpc.1024chain.com/rpc/
WebSocket: wss://testnet-rpc.1024chain.com/ws/
区块浏览器: https://testnet-scan.1024chain.com/
```

### Program 信息

| 项目 | 值 |
|------|-----|
| **Program ID** | `3CioJbGydCvrRidfB2fbKzcpFUadtap6S7e5MHTm4XRe` |
| **GlobalConfig PDA** | `Fjn64GP2tRzMVucy8R1M3vL8ZqmEyxqtStgvkFFDFgZr` |
| **USDC Mint** | `6u1x12yV2XFcEDGd8KByZZqnjipRiq9BJB2xKprhAipy` |
| **Admin** | `J1Szw8HZYL95NvYUsNhg3e6NzKQLUZ9UxQsKg4hsQnad` |
| **Program Size** | 161,336 bytes (157 KB) |
| **Deployment Slot** | 17430785 |

### 配置文件位置

```bash
scripts/config.json  # 包含所有部署配置
```

---

## 架构设计

### 核心账户类型

#### 1. GlobalConfig (全局配置单例)

```rust
pub struct GlobalConfig {
    pub discriminator: u64,    // 账户类型标识
    pub version: u8,           // 配置版本
    pub bump: u8,              // PDA bump
    pub admin: Pubkey,         // 管理员（可放弃）
    pub usdc_mint: Pubkey,     // USDC Mint 地址
    pub created_at: i64,       // 创建时间
    pub reserved: [u8; 64],    // 预留扩展
}
```

**PDA Seeds**: `["global", 1]`

#### 2. UserVault (用户金库)

```rust
pub struct UserVault {
    pub discriminator: u64,       // 账户类型标识
    pub version: u8,              // 数据版本
    pub bump: u8,                 // Vault PDA bump
    pub usdc_bump: u8,            // USDC Token Account bump
    pub owner: Pubkey,            // 用户主钱包
    pub usdc_vault: Pubkey,       // USDC Token Account
    pub total_deposit: u64,       // 历史总存入
    pub total_withdrawn: u64,     // 历史总提出
    pub free_collateral: u64,     // 可用保证金
    pub locked_collateral: u64,   // 锁定保证金
    pub flags: u64,               // 状态标记（冻结等）
    pub created_at: i64,          // 创建时间
    pub updated_at: i64,          // 更新时间
    pub reserved: [u8; 64],       // 预留扩展
}
```

**PDA Seeds**: `["vault", owner_pubkey]`

#### 3. DelegateAccount (API Key 授权)

```rust
pub struct DelegateAccount {
    pub discriminator: u64,    // 账户类型标识
    pub version: u8,           // 数据版本
    pub bump: u8,              // PDA bump
    pub owner: Pubkey,         // Vault 所有者
    pub vault: Pubkey,         // 对应的 UserVault
    pub delegate: Pubkey,      // API Key 公钥
    pub is_active: bool,       // 是否激活
    pub permissions: u64,      // 权限位掩码
    pub max_notional: u64,     // 最大可用敞口
    pub used_notional: u64,    // 当前已用敞口
    pub expiry_slot: u64,      // 过期 slot
    pub nonce: u64,            // 防重放计数器
    pub created_at: i64,       // 创建时间
    pub updated_at: i64,       // 更新时间
    pub reserved: [u8; 64],    // 预留扩展
}
```

**PDA Seeds**: `["delegate", owner_pubkey, delegate_pubkey]`

### 权限系统

```rust
// 权限位定义
pub const PERM_TRADE: u64 = 1 << 0;       // 允许交易
pub const PERM_WITHDRAW: u64 = 1 << 1;    // 允许提现
pub const PERM_CLOSE_ONLY: u64 = 1 << 2;  // 只允许平仓
pub const PERM_VIEW_ONLY: u64 = 1 << 3;   // 只读权限
```

**权限组合示例**:
- 只交易: `permissions = 1` (PERM_TRADE)
- 交易+提现: `permissions = 3` (PERM_TRADE | PERM_WITHDRAW)
- 只平仓: `permissions = 5` (PERM_TRADE | PERM_CLOSE_ONLY)
- 完全权限: `permissions = 7` (PERM_TRADE | PERM_WITHDRAW | PERM_CLOSE_ONLY)

---

## 快速开始

### 前置要求

```bash
# 安装 Solana CLI
sh -c "$(curl -sSfL https://release.solana.com/stable/install)"

# 配置 RPC
solana config set --url https://testnet-rpc.1024chain.com/rpc/

# 验证连接
solana epoch-info
```

### 环境配置

```bash
# 克隆仓库
cd /path/to/1024-api-key-vault-program

# 查看配置
cat scripts/config.json
```

### 配置文件说明

```json
{
  "program_id": "3CioJbGydCvrRidfB2fbKzcpFUadtap6S7e5MHTm4XRe",
  "usdc_mint": "6u1x12yV2XFcEDGd8KByZZqnjipRiq9BJB2xKprhAipy",
  "rpc_url": "https://testnet-rpc.1024chain.com/rpc/",
  "network": "1024chain-testnet",
  "test_accounts": {
    "account1": {
      "pubkey": "9ocm9zv5F2QghKaFSLGSjkVg6f8XZf54nVTjfC2M3dG4",
      "secret": "65d7pAydmKwgo5mVBwnKQUS7BUP1ZBhisEbeRyfzFnGLez85AGSqcqbZCUbsccogzSyLBqYcoZVgU7x7AARtKMHz"
    }
  }
}
```

---

## 完整使用流程

### 1. 创建 Vault

#### 1.1 准备工作

```typescript
import { Connection, PublicKey, Keypair } from '@solana/web3.js';
import bs58 from 'bs58';

// 连接
const connection = new Connection('https://testnet-rpc.1024chain.com/rpc/');

// 用户钱包（使用测试账户 #1）
const owner = Keypair.fromSecretKey(
  bs58.decode("65d7pAydmKwgo5mVBwnKQUS7BUP1ZBhisEbeRyfzFnGLez85AGSqcqbZCUbsccogzSyLBqYcoZVgU7x7AARtKMHz")
);

// Program ID
const PROGRAM_ID = new PublicKey("3CioJbGydCvrRidfB2fbKzcpFUadtap6S7e5MHTm4XRe");
const USDC_MINT = new PublicKey("6u1x12yV2XFcEDGd8KByZZqnjipRiq9BJB2xKprhAipy");
```

#### 1.2 派生 PDA

```typescript
// UserVault PDA
const [vaultPDA, vaultBump] = PublicKey.findProgramAddressSync(
  [Buffer.from("vault"), owner.publicKey.toBuffer()],
  PROGRAM_ID
);

// Vault USDC Token Account PDA
const [vaultUsdcPDA, vaultUsdcBump] = PublicKey.findProgramAddressSync(
  [Buffer.from("vault-usdc"), owner.publicKey.toBuffer()],
  PROGRAM_ID
);

console.log('Vault PDA:', vaultPDA.toBase58());
console.log('Vault USDC PDA:', vaultUsdcPDA.toBase58());
```

#### 1.3 构造 CreateVault 指令

```typescript
import { 
  Transaction, 
  TransactionInstruction,
  SystemProgram,
  SYSVAR_RENT_PUBKEY 
} from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '@solana/spl-token';

// GlobalConfig PDA
const [globalConfigPDA] = PublicKey.findProgramAddressSync(
  [Buffer.from("global"), Buffer.from([1])],
  PROGRAM_ID
);

// 指令数据 (VaultInstruction::CreateVault = 1)
const instructionData = Buffer.from([1]); // 只有指令索引

// 构造指令
const createVaultIx = new TransactionInstruction({
  keys: [
    { pubkey: vaultPDA, isSigner: false, isWritable: true },
    { pubkey: vaultUsdcPDA, isSigner: false, isWritable: true },
    { pubkey: owner.publicKey, isSigner: true, isWritable: true },
    { pubkey: globalConfigPDA, isSigner: false, isWritable: false },
    { pubkey: USDC_MINT, isSigner: false, isWritable: false },
    { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
    { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
  ],
  programId: PROGRAM_ID,
  data: instructionData,
});

// 发送交易
const tx = new Transaction().add(createVaultIx);
const signature = await connection.sendTransaction(tx, [owner]);
await connection.confirmTransaction(signature);

console.log('✅ Vault created! Signature:', signature);
```

---

### 2. 存款 USDC

#### 2.1 创建/获取 Token Account

```typescript
import { 
  getOrCreateAssociatedTokenAccount,
  getAssociatedTokenAddress 
} from '@solana/spl-token';

// 获取或创建用户的 USDC Token Account
const userUsdcAccount = await getOrCreateAssociatedTokenAccount(
  connection,
  owner,
  USDC_MINT,
  owner.publicKey
);

console.log('User USDC Account:', userUsdcAccount.address.toBase58());
```

#### 2.2 构造 Deposit 指令

```typescript
import * as borsh from 'borsh';

// Deposit 指令数据结构
class DepositData {
  instruction = 2; // VaultInstruction::Deposit
  amount: bigint;

  constructor(amount: bigint) {
    this.amount = amount;
  }
}

const depositSchema = new Map([
  [DepositData, {
    kind: 'struct',
    fields: [
      ['instruction', 'u8'],
      ['amount', 'u64'],
    ],
  }],
]);

// 存入 1000 USDC (注意：USDC decimals = 9)
const depositAmount = 1000n * 1_000_000_000n; // 1000 USDC

const depositData = new DepositData(depositAmount);
const depositInstructionData = borsh.serialize(depositSchema, depositData);

// 构造指令
const depositIx = new TransactionInstruction({
  keys: [
    { pubkey: vaultPDA, isSigner: false, isWritable: true },
    { pubkey: owner.publicKey, isSigner: true, isWritable: false },
    { pubkey: userUsdcAccount.address, isSigner: false, isWritable: true },
    { pubkey: vaultUsdcPDA, isSigner: false, isWritable: true },
    { pubkey: globalConfigPDA, isSigner: false, isWritable: false },
    { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
  ],
  programId: PROGRAM_ID,
  data: Buffer.from(depositInstructionData),
});

// 发送交易
const depositTx = new Transaction().add(depositIx);
const depositSig = await connection.sendTransaction(depositTx, [owner]);
await connection.confirmTransaction(depositSig);

console.log('✅ Deposited 1000 USDC! Signature:', depositSig);
```

---

### 3. 创建 API Key (Delegate)

#### 3.1 生成 API Key

```typescript
// 生成新的 API Key
const apiKey = Keypair.generate();

console.log('API Key Public:', apiKey.publicKey.toBase58());
console.log('API Key Secret:', bs58.encode(apiKey.secretKey));

// ⚠️ 重要：私钥只保存在用户环境，项目方不保存
```

#### 3.2 派生 DelegateAccount PDA

```typescript
const [delegatePDA, delegateBump] = PublicKey.findProgramAddressSync(
  [
    Buffer.from("delegate"),
    owner.publicKey.toBuffer(),
    apiKey.publicKey.toBuffer()
  ],
  PROGRAM_ID
);

console.log('Delegate PDA:', delegatePDA.toBase58());
```

#### 3.3 构造 UpsertDelegate 指令

```typescript
// UpsertDelegate 指令数据
class UpsertDelegateData {
  instruction = 4; // VaultInstruction::UpsertDelegate
  delegate_pubkey: Uint8Array;
  permissions: bigint;
  max_notional: bigint;
  expiry_slot: bigint;

  constructor(props: {
    delegate_pubkey: PublicKey,
    permissions: bigint,
    max_notional: bigint,
    expiry_slot: bigint
  }) {
    this.delegate_pubkey = props.delegate_pubkey.toBytes();
    this.permissions = props.permissions;
    this.max_notional = props.max_notional;
    this.expiry_slot = props.expiry_slot;
  }
}

const upsertDelegateSchema = new Map([
  [UpsertDelegateData, {
    kind: 'struct',
    fields: [
      ['instruction', 'u8'],
      ['delegate_pubkey', [32]],
      ['permissions', 'u64'],
      ['max_notional', 'u64'],
      ['expiry_slot', 'u64'],
    ],
  }],
]);

// 设置权限：允许交易，最大 5000 USDC 敞口
const PERM_TRADE = 1n << 0n;
const currentSlot = await connection.getSlot();
const expirySlot = BigInt(currentSlot) + 100_000n; // ~1天后过期

const upsertData = new UpsertDelegateData({
  delegate_pubkey: apiKey.publicKey,
  permissions: PERM_TRADE,
  max_notional: 5000n * 1_000_000_000n, // 5000 USDC
  expiry_slot: expirySlot,
});

const upsertInstructionData = borsh.serialize(upsertDelegateSchema, upsertData);

// 构造指令
const upsertIx = new TransactionInstruction({
  keys: [
    { pubkey: delegatePDA, isSigner: false, isWritable: true },
    { pubkey: vaultPDA, isSigner: false, isWritable: true },
    { pubkey: owner.publicKey, isSigner: true, isWritable: true },
    { pubkey: globalConfigPDA, isSigner: false, isWritable: false },
    { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
  ],
  programId: PROGRAM_ID,
  data: Buffer.from(upsertInstructionData),
});

// 发送交易
const upsertTx = new Transaction().add(upsertIx);
const upsertSig = await connection.sendTransaction(upsertTx, [owner]);
await connection.confirmTransaction(upsertSig);

console.log('✅ API Key created! Signature:', upsertSig);
```

---

### 4. 使用 API Key 提款

#### 4.1 使用 API Key 签名

```typescript
// 提款指令数据
class WithdrawData {
  instruction = 3; // VaultInstruction::Withdraw
  amount: bigint;

  constructor(amount: bigint) {
    this.amount = amount;
  }
}

const withdrawSchema = new Map([
  [WithdrawData, {
    kind: 'struct',
    fields: [
      ['instruction', 'u8'],
      ['amount', 'u64'],
    ],
  }],
]);

// 提取 100 USDC
const withdrawAmount = 100n * 1_000_000_000n;
const withdrawData = new WithdrawData(withdrawAmount);
const withdrawInstructionData = borsh.serialize(withdrawSchema, withdrawData);

// 构造指令（使用 delegate 签名）
const withdrawIx = new TransactionInstruction({
  keys: [
    { pubkey: vaultPDA, isSigner: false, isWritable: true },
    { pubkey: apiKey.publicKey, isSigner: true, isWritable: false }, // delegate 签名
    { pubkey: userUsdcAccount.address, isSigner: false, isWritable: true },
    { pubkey: vaultUsdcPDA, isSigner: false, isWritable: true },
    { pubkey: globalConfigPDA, isSigner: false, isWritable: false },
    { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
    { pubkey: delegatePDA, isSigner: false, isWritable: false }, // DelegateAccount
  ],
  programId: PROGRAM_ID,
  data: Buffer.from(withdrawInstructionData),
});

// 使用 API Key 签名发送
const withdrawTx = new Transaction().add(withdrawIx);
const withdrawSig = await connection.sendTransaction(withdrawTx, [apiKey]);
await connection.confirmTransaction(withdrawSig);

console.log('✅ Withdrawn 100 USDC using API Key! Signature:', withdrawSig);
```

---

### 5. 撤销 API Key

```typescript
// RevokeDelegate 指令数据
class RevokeDelegateData {
  instruction = 5; // VaultInstruction::RevokeDelegate
  delegate_pubkey: Uint8Array;

  constructor(delegate_pubkey: PublicKey) {
    this.delegate_pubkey = delegate_pubkey.toBytes();
  }
}

const revokeSchema = new Map([
  [RevokeDelegateData, {
    kind: 'struct',
    fields: [
      ['instruction', 'u8'],
      ['delegate_pubkey', [32]],
    ],
  }],
]);

const revokeData = new RevokeDelegateData(apiKey.publicKey);
const revokeInstructionData = borsh.serialize(revokeSchema, revokeData);

// 构造指令（必须由 owner 签名）
const revokeIx = new TransactionInstruction({
  keys: [
    { pubkey: delegatePDA, isSigner: false, isWritable: true },
    { pubkey: vaultPDA, isSigner: false, isWritable: true },
    { pubkey: owner.publicKey, isSigner: true, isWritable: false },
    { pubkey: globalConfigPDA, isSigner: false, isWritable: false },
  ],
  programId: PROGRAM_ID,
  data: Buffer.from(revokeInstructionData),
});

// 发送交易
const revokeTx = new Transaction().add(revokeIx);
const revokeSig = await connection.sendTransaction(revokeTx, [owner]);
await connection.confirmTransaction(revokeSig);

console.log('✅ API Key revoked! Signature:', revokeSig);
```

---

## API 参考

### 指令索引

| 指令 | 索引 | 签名者 | 说明 |
|------|------|--------|------|
| `InitializeGlobalConfig` | 0 | Admin | 初始化全局配置 |
| `CreateVault` | 1 | Owner | 创建用户 Vault |
| `Deposit` | 2 | Owner | 存款到 Vault |
| `Withdraw` | 3 | Owner/Delegate | 从 Vault 提款 |
| `UpsertDelegate` | 4 | Owner | 添加/更新 API Key |
| `RevokeDelegate` | 5 | Owner | 撤销 API Key |
| `LockMargin` | 6 | Owner/Delegate | 锁定保证金 |
| `UnlockMarginAndUpdatePnl` | 7 | Owner/Delegate | 解锁保证金 |
| `TransferAdmin` | 8 | Admin | 转移管理员 |
| `RenounceAdmin` | 9 | Admin | 放弃管理员 |
| `FreezeVault` | 10 | Owner | 冻结 Vault |
| `UnfreezeVault` | 11 | Owner | 解冻 Vault |

### 账户验证规则

| 操作 | Owner | Delegate + PERM_TRADE | Delegate + PERM_WITHDRAW |
|------|-------|----------------------|-------------------------|
| CreateVault | ✅ | ❌ | ❌ |
| Deposit | ✅ | ❌ | ❌ |
| Withdraw | ✅ | ❌ | ✅ |
| UpsertDelegate | ✅ | ❌ | ❌ |
| RevokeDelegate | ✅ | ❌ | ❌ |
| LockMargin | ✅ | ✅ | ❌ |

---

## 常见问题

### Q1: USDC Decimals 是多少？

A: 链上 USDC 的 decimals 是 **9**（而非标准的 6）。所有金额计算需要使用 `amount * 1_000_000_000`。

### Q2: 如何查看 Vault 余额？

```bash
# 查看 Vault 账户
solana account <VAULT_PDA>

# 或查看 Vault USDC Token Account
spl-token balance <USDC_MINT> --owner <VAULT_USDC_PDA>
```

### Q3: API Key 私钥丢失怎么办？

A: 
1. 使用 owner 钱包调用 `RevokeDelegate` 撤销该 API Key
2. 创建新的 API Key
3. API Key 私钥丢失不影响资金安全，owner 始终拥有完全控制权

### Q4: 如何实现完全非托管？

A: 管理员可以调用 `RenounceAdmin` 指令，将 admin 设为 `Pubkey::default()`，之后无人可以修改 GlobalConfig，实现完全去中心化。

### Q5: Vault 可以同时有多少个 API Key？

A: 理论上无限制，每个 API Key 对应一个独立的 DelegateAccount PDA。

---

## 相关文档

- **设计文档**: `docs/design.md` - 详细架构设计
- **代码审计**: `CODE_AUDIT_REPORT.md` - 完整审计报告
- **部署总结**: `DEPLOYMENT_SUMMARY.md` - 部署信息
- **验证报告**: `DEPLOYMENT_VERIFICATION.md` - 验证结果

---

**文档版本**: v1.0  
**最后更新**: 2025-11-17  
**维护者**: 1024 Team

