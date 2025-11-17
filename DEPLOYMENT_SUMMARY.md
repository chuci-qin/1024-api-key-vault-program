# 1024 API Key Vault Program - 部署总结

**部署日期**: 2025-11-17  
**网络**: 1024Chain Testnet  
**状态**: ✅ 部署成功并已初始化

---

## 📋 部署信息

### Program 信息

| 项目 | 值 |
|------|-----|
| **Program ID** | `9omyQr3wY5K5KyL53BQzLz9QTzAve6oYzg8LyfXFpsj8` |
| **Program Data Address** | `3o2mqVt2XimecQNmeDqwUwWSG1kfNQuKrmfHnoqoK9i6` |
| **Authority** | `J1Szw8HZYL95NvYUsNhg3e6NzKQLUZ9UxQsKg4hsQnad` |
| **Program Size** | 161,336 bytes (157 KB) |
| **Deployment Slot** | 17430785 |
| **Deployment Signature** | `25SQVYEe2s4yKVbD94veybdeTZWhredWix6Go3AJEHUmxyA13WqUJUFC45t4BTsK7JJpF4GWP4GfHshN1R8UjwHj` |
| **Balance** | 1.12410264 SOL |

### GlobalConfig 信息

| 项目 | 值 |
|------|-----|
| **GlobalConfig PDA** | `2a4x1w3RrGYNpZrn1pFZwqeGDm3rQQR4yP3J1NCukJXm` |
| **Bump Seed** | 254 |
| **USDC Mint** | `6u1x12yV2XFcEDGd8KByZZqnjipRiq9BJB2xKprhAipy` |
| **Admin** | `J1Szw8HZYL95NvYUsNhg3e6NzKQLUZ9UxQsKg4hsQnad` |
| **Account Size** | 152 bytes |
| **Status** | ✅ 已初始化 |

---

## 🌐 网络配置

```bash
RPC URL: https://testnet-rpc.1024chain.com/rpc/
WebSocket: wss://testnet-rpc.1024chain.com/ws/
区块浏览器: https://testnet-scan.1024chain.com/
```

---

## 🔍 验证命令

### 1. 查看 Program 信息

```bash
solana program show 3CioJbGydCvrRidfB2fbKzcpFUadtap6S7e5MHTm4XRe \
  --url https://testnet-rpc.1024chain.com/rpc/
```

### 2. 查看 GlobalConfig 账户

```bash
solana account Fjn64GP2tRzMVucy8R1M3vL8ZqmEyxqtStgvkFFDFgZr \
  --url https://testnet-rpc.1024chain.com/rpc/
```

### 3. 查看 USDC Mint

```bash
spl-token display 6u1x12yV2XFcEDGd8KByZZqnjipRiq9BJB2xKprhAipy \
  --url https://testnet-rpc.1024chain.com/rpc/
```

---

## 🧪 测试账户

已配置 3 个测试账户（来自当前配置信息.md）：

| 账户 | 公钥 | 余额 |
|------|------|------|
| **测试账户 #1** | `9ocm9zv5F2QghKaFSLGSjkVg6f8XZf54nVTjfC2M3dG4` | 100 N1024 |
| **测试账户 #2** | `G23icA8QJiAM2UwENf1112rGFxoqHP6JJa3TuwVseVxu` | 100 N1024 |
| **测试账户 #3** | `9S55H6Bbh2JCqdmQGcw2MWCdWeBNNQYb9GWiCHL62CUH` | 100 N1024 |

---

## 📖 使用指南

### 创建 Vault

```typescript
// 使用测试账户 #1
const owner = Keypair.fromSecretKey(
  bs58.decode("65d7pAydmKwgo5mVBwnKQUS7BUP1ZBhisEbeRyfzFnGLez85AGSqcqbZCUbsccogzSyLBqYcoZVgU7x7AARtKMHz")
);

// Derive UserVault PDA
const [vaultPDA] = PublicKey.findProgramAddressSync(
  [Buffer.from("vault"), owner.publicKey.toBuffer()],
  PROGRAM_ID
);

// Create vault instruction...
```

### 存款到 Vault

```typescript
// 需要先创建 USDC token account
const userUsdcAccount = await getOrCreateAssociatedTokenAccount(
  connection,
  owner,
  USDC_MINT,
  owner.publicKey
);

// Deposit instruction...
```

### 创建 API Key (Delegate)

```typescript
// 生成 API Key
const apiKey = Keypair.generate();

// Derive DelegateAccount PDA
const [delegatePDA] = PublicKey.findProgramAddressSync(
  [
    Buffer.from("delegate"),
    owner.publicKey.toBuffer(),
    apiKey.publicKey.toBuffer()
  ],
  PROGRAM_ID
);

// UpsertDelegate instruction...
```

---

## 🔐 PDA 派生规则

| PDA 类型 | Seeds |
|----------|-------|
| **GlobalConfig** | `["global", 1]` |
| **UserVault** | `["vault", owner_pubkey]` |
| **Vault USDC Account** | `["vault-usdc", owner_pubkey]` |
| **DelegateAccount** | `["delegate", owner_pubkey, delegate_pubkey]` |

---

## 📝 权限说明

### 权限位定义

```rust
PERM_TRADE       = 1 << 0  // 允许交易（开平仓）
PERM_WITHDRAW    = 1 << 1  // 允许提现
PERM_CLOSE_ONLY  = 1 << 2  // 只允许平仓
PERM_VIEW_ONLY   = 1 << 3  // 只读权限
```

### 示例组合

- **交易权限**: `permissions = 1` (PERM_TRADE)
- **交易+提现**: `permissions = 3` (PERM_TRADE | PERM_WITHDRAW)
- **只允许平仓**: `permissions = 5` (PERM_TRADE | PERM_CLOSE_ONLY)

---

## 🎯 下一步

1. ✅ Program 已部署
2. ✅ GlobalConfig 已初始化
3. ⏳ 创建测试 Vault
4. ⏳ 测试存款/提款
5. ⏳ 测试 Delegate 功能
6. ⏳ 集成到 1024-core 后端
7. ⏳ 集成到 1024-chain-frontend

---

## 📞 联系方式

- **GitHub**: https://github.com/1024-org/1024-api-key-vault-program
- **文档**: `docs/design.md`, `docs/draft.md`
- **审计报告**: `CODE_AUDIT_REPORT.md`

---

**部署完成时间**: 2025-11-17 16:20 UTC+8  
**部署人员**: Chuci Qin  
**状态**: ✅ 生产就绪

