# 1024 API Key Vault Program - 快速开始指南

## 🚀 5分钟快速上手

### 1. 安装依赖

```bash
# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安装 Solana CLI
sh -c "$(curl -sSfL https://release.solana.com/stable/install)"

# 验证安装
rustc --version
solana --version
```

### 2. 克隆和构建

```bash
git clone https://github.com/1024-org/1024-api-key-vault-program.git
cd 1024-api-key-vault-program

# 编译检查
cargo check

# 构建 Solana BPF 程序
cargo build-sbf
```

### 3. 运行测试

```bash
# 单元测试
cargo test

# 集成测试（需要 solana-test-validator）
cargo test-sbf
```

### 4. 本地部署测试

```bash
# 启动本地测试网
solana-test-validator

# 新开一个终端，部署程序
solana program deploy target/deploy/vault_program.so

# 记录 Program ID
export VAULT_PROGRAM_ID=<YOUR_PROGRAM_ID>
```

## 📖 核心概念速览

### 非托管架构

```
用户资金
    ↓
存入 UserVault（智能合约托管）
    ↓
授权给 API Key（delegate）
    ↓
策略自动交易（用户环境运行）
    ↓
随时撤销权限
```

### 权限系统

| 权限位 | 值 | 说明 |
|--------|-----|------|
| `PERM_TRADE` | `1 << 0` | 允许交易（开平仓） |
| `PERM_WITHDRAW` | `1 << 1` | 允许提现 |
| `PERM_CLOSE_ONLY` | `1 << 2` | 只允许平仓 |
| `PERM_VIEW_ONLY` | `1 << 3` | 只读权限 |

### 核心账户

1. **GlobalConfig** - 全局配置（单例）
2. **UserVault** - 用户金库（每用户一个）
3. **DelegateAccount** - API Key 授权（每个 Key 一个）
4. **Vault USDC Account** - USDC 存储（每 Vault 一个）

## 🔧 使用示例

### 场景：创建量化策略 API Key

```rust
// 1. 用户创建 Vault
CreateVault { }

// 2. 存入 USDC
Deposit { amount: 10_000_000_000 } // 10,000 USDC

// 3. 创建 API Key（本地生成 keypair）
let api_key = Keypair::new();

UpsertDelegate {
    delegate_pubkey: api_key.pubkey(),
    permissions: PERM_TRADE,  // 只允许交易
    max_notional: 5_000_000_000,  // 最大 5,000 USDC
    expiry_slot: current_slot + 30_days,
}

// 4. 策略使用 API Key 签名交易
// （在用户自己的服务器上运行）
let tx = Transaction::new_signed_with_payer(
    &[lock_margin_ix],
    Some(&user.pubkey()),
    &[&user, &api_key],  // API Key 签名
    recent_blockhash,
);

// 5. 随时撤销
RevokeDelegate {
    delegate_pubkey: api_key.pubkey(),
}
```

## 📂 项目结构

```
1024-api-key-vault-program/
├── programs/vault/src/
│   ├── lib.rs          # 程序入口
│   ├── state.rs        # 数据结构（GlobalConfig, UserVault, Delegate）
│   ├── instruction.rs  # 指令定义
│   ├── processor.rs    # 指令处理器
│   ├── error.rs        # 错误类型
│   └── utils.rs        # 工具函数
├── tests/              # 测试文件
├── docs/               # 文档
└── README.md
```

## 🧪 测试示例

```rust
#[tokio::test]
async fn test_create_and_deposit() {
    // 初始化测试环境
    let program_test = ProgramTest::new(...);
    let (banks_client, payer, blockhash) = program_test.start().await;
    
    // 创建 Vault
    let create_vault_ix = create_vault(...);
    // ...执行交易...
    
    // 存款
    let deposit_ix = deposit(10_000_000_000);
    // ...执行交易...
    
    // 验证余额
    let vault = get_vault_account(...);
    assert_eq!(vault.free_collateral, 10_000_000_000);
}
```

## 📚 下一步

1. 阅读 [完整文档](docs/DEVELOPMENT_PROGRESS.md)
2. 查看 [设计文档](docs/design.md)
3. 运行测试了解功能
4. 参与贡献 [CONTRIBUTING.md](CONTRIBUTING.md)

## ⚠️ 注意事项

- **测试网先行**：先在 devnet/testnet 充分测试
- **审计必要**：mainnet 部署前进行安全审计
- **密钥安全**：API Key 私钥只在用户环境保存
- **权限最小化**：只授予必要的权限

## 🆘 遇到问题？

- 查看 [文档](docs/)
- 提交 [Issue](https://github.com/1024-org/1024-api-key-vault-program/issues)
- 加入 [Discord](https://discord.gg/1024ex)

---

**开始你的非托管量化之旅！** 🚀

