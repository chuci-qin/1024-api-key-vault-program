# 1024 API Key Vault Program - 集成指南

**版本**: v1.0  
**日期**: 2025-11-17  
**目标**: 与 1024-core 后端和 1024-chain-frontend 前端集成

---

## 📋 目录

1. [集成概述](#集成概述)
2. [后端集成 (1024-core)](#后端集成-1024-core)
3. [前端集成 (1024-chain-frontend)](#前端集成-1024-chain-frontend)
4. [SDK 开发](#sdk-开发)
5. [测试指南](#测试指南)

---

## 集成概述

### 架构图

```
┌─────────────────────────────────────────────────────────────┐
│                    1024-chain-frontend                       │
│                     (Next.js + React)                        │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │  钱包连接    │  │  创建 Vault  │  │ API Key 管理 │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
└───────────────────────────┬──────────────────────────────────┘
                            │
                            │ RPC Calls
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                       1024-core                              │
│                    (Rust Backend)                            │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │Vault Manager │  │ Order System │  │ Settlement   │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
└───────────────────────────┬──────────────────────────────────┘
                            │
                            │ CPI Calls
                            ▼
┌─────────────────────────────────────────────────────────────┐
│              1024 API Key Vault Program                      │
│                  (Solana Program)                            │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │ UserVault    │  │ Delegate     │  │ LockMargin   │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
└─────────────────────────────────────────────────────────────┘
```

### 集成目标

1. **后端集成**
   - Vault 账户管理
   - CPI 调用锁定/解锁保证金
   - 订单系统集成

2. **前端集成**
   - 用户创建 Vault UI
   - API Key 管理界面
   - 余额显示

---

## 后端集成 (1024-core)

### 1. 添加依赖

**文件**: `1024-core/crates/vault-client/Cargo.toml`

```toml
[package]
name = "vault-client"
version = "0.1.0"
edition = "2021"

[dependencies]
solana-program = "=1.18.26"
solana-client = "=1.18.26"
solana-sdk = "=1.18.26"
spl-token = "4.0.0"
borsh = "0.10"
thiserror = "1.0"
```

### 2. 创建 Vault Client

**文件**: `1024-core/crates/vault-client/src/lib.rs`

```rust
use solana_program::pubkey::Pubkey;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    signer::Signer,
    transaction::Transaction,
};
use std::str::FromStr;

// Vault Program ID (1024Chain Testnet)
pub const VAULT_PROGRAM_ID: &str = "9omyQr3wY5K5KyL53BQzLz9QTzAve6oYzg8LyfXFpsj8";

pub fn get_vault_program_id() -> Pubkey {
    Pubkey::from_str(VAULT_PROGRAM_ID).unwrap()
}

/// 派生 UserVault PDA
pub fn derive_vault_pda(owner: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"vault", owner.as_ref()],
        &get_vault_program_id(),
    )
}

/// 派生 Vault USDC Token Account PDA
pub fn derive_vault_usdc_pda(owner: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"vault-usdc", owner.as_ref()],
        &get_vault_program_id(),
    )
}

/// 派生 DelegateAccount PDA
pub fn derive_delegate_pda(owner: &Pubkey, delegate: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"delegate", owner.as_ref(), delegate.as_ref()],
        &get_vault_program_id(),
    )
}

/// 派生 GlobalConfig PDA
pub fn derive_global_config_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"global", &[1u8]],
        &get_vault_program_id(),
    )
}

#[repr(u8)]
pub enum VaultInstruction {
    InitializeGlobalConfig = 0,
    CreateVault = 1,
    Deposit = 2,
    Withdraw = 3,
    UpsertDelegate = 4,
    RevokeDelegate = 5,
    LockMargin = 6,
    UnlockMarginAndUpdatePnl = 7,
    TransferAdmin = 8,
    RenounceAdmin = 9,
    FreezeVault = 10,
    UnfreezeVault = 11,
}

/// 锁定保证金指令
pub fn lock_margin_instruction(
    owner: &Pubkey,
    signer: &Pubkey,
    required_margin: u64,
    required_notional: u64,
    delegate_account: Option<&Pubkey>,
) -> Instruction {
    let (vault_pda, _) = derive_vault_pda(owner);
    let (global_config_pda, _) = derive_global_config_pda();

    #[derive(borsh::BorshSerialize)]
    struct LockMarginData {
        instruction: u8,
        required_margin: u64,
        required_notional: u64,
    }

    let data = LockMarginData {
        instruction: VaultInstruction::LockMargin as u8,
        required_margin,
        required_notional,
    };

    let mut accounts = vec![
        AccountMeta::new(vault_pda, false),
        AccountMeta::new_readonly(*signer, true),
    ];

    if let Some(delegate) = delegate_account {
        accounts.push(AccountMeta::new(*delegate, false));
    }

    accounts.push(AccountMeta::new_readonly(global_config_pda, false));
    accounts.push(AccountMeta::new_readonly(solana_program::sysvar::clock::id(), false));

    Instruction {
        program_id: get_vault_program_id(),
        accounts,
        data: borsh::to_vec(&data).unwrap(),
    }
}

/// 解锁保证金并更新 PnL 指令
pub fn unlock_margin_and_update_pnl_instruction(
    owner: &Pubkey,
    signer: &Pubkey,
    unlocked_margin: u64,
    pnl_delta: i64,
    notional_delta: i64,
    delegate_account: Option<&Pubkey>,
) -> Instruction {
    let (vault_pda, _) = derive_vault_pda(owner);
    let (global_config_pda, _) = derive_global_config_pda();

    #[derive(borsh::BorshSerialize)]
    struct UnlockMarginData {
        instruction: u8,
        unlocked_margin: u64,
        pnl_delta: i64,
        notional_delta: i64,
    }

    let data = UnlockMarginData {
        instruction: VaultInstruction::UnlockMarginAndUpdatePnl as u8,
        unlocked_margin,
        pnl_delta,
        notional_delta,
    };

    let mut accounts = vec![
        AccountMeta::new(vault_pda, false),
        AccountMeta::new_readonly(*signer, true),
    ];

    if let Some(delegate) = delegate_account {
        accounts.push(AccountMeta::new(*delegate, false));
    }

    accounts.push(AccountMeta::new_readonly(global_config_pda, false));

    Instruction {
        program_id: get_vault_program_id(),
        accounts,
        data: borsh::to_vec(&data).unwrap(),
    }
}
```

### 3. 集成到订单系统

**文件**: `1024-core/crates/gateway/src/order_handler.rs`

```rust
use vault_client::{lock_margin_instruction, unlock_margin_and_update_pnl_instruction};

impl OrderHandler {
    /// 下单时锁定保证金
    pub async fn place_order_with_vault(
        &self,
        user: &Pubkey,
        side: Side,
        size: f64,
        price: f64,
        api_key: Option<&Pubkey>, // 如果使用 API Key
    ) -> Result<String> {
        // 计算所需保证金
        let notional = size * price;
        let required_margin = (notional / self.config.leverage as f64) as u64;
        let required_notional = notional as u64;

        // 派生 delegate PDA（如果使用 API Key）
        let delegate_pda = api_key.map(|key| {
            vault_client::derive_delegate_pda(user, key).0
        });

        // 构造锁定保证金指令
        let lock_ix = lock_margin_instruction(
            user,
            api_key.unwrap_or(user), // 使用 API Key 或 owner
            required_margin,
            required_notional,
            delegate_pda.as_ref(),
        );

        // 构造下单指令
        let place_order_ix = self.create_place_order_instruction(...);

        // 组合成一笔交易
        let tx = Transaction::new_with_payer(
            &[lock_ix, place_order_ix],
            Some(user),
        );

        // 发送交易
        self.send_and_confirm_transaction(tx).await
    }

    /// 平仓时解锁保证金
    pub async fn close_position_with_vault(
        &self,
        user: &Pubkey,
        position_id: u64,
        pnl: i64, // 盈亏（正数盈利，负数亏损）
        api_key: Option<&Pubkey>,
    ) -> Result<String> {
        // 获取仓位信息
        let position = self.get_position(position_id)?;
        let unlocked_margin = position.margin;
        let notional_delta = -(position.notional as i64); // 释放敞口

        // 派生 delegate PDA
        let delegate_pda = api_key.map(|key| {
            vault_client::derive_delegate_pda(user, key).0
        });

        // 构造解锁保证金指令
        let unlock_ix = unlock_margin_and_update_pnl_instruction(
            user,
            api_key.unwrap_or(user),
            unlocked_margin,
            pnl,
            notional_delta,
            delegate_pda.as_ref(),
        );

        // 构造平仓指令
        let close_position_ix = self.create_close_position_instruction(...);

        // 组合成一笔交易
        let tx = Transaction::new_with_payer(
            &[close_position_ix, unlock_ix],
            Some(user),
        );

        // 发送交易
        self.send_and_confirm_transaction(tx).await
    }
}
```

### 4. 添加 API 端点

**文件**: `1024-core/crates/gateway/src/api/vault.rs`

```rust
use axum::{Json, Router, routing::{get, post}};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct VaultInfo {
    pub vault_pda: String,
    pub vault_usdc_pda: String,
    pub free_collateral: u64,
    pub locked_collateral: u64,
    pub total_deposit: u64,
    pub total_withdrawn: u64,
}

#[derive(Deserialize)]
pub struct CreateDelegateRequest {
    pub delegate_pubkey: String,
    pub permissions: u64,
    pub max_notional: u64,
    pub expiry_slot: u64,
}

pub fn vault_routes() -> Router {
    Router::new()
        .route("/vault/info/:user", get(get_vault_info))
        .route("/vault/delegates/:user", get(get_user_delegates))
        .route("/vault/delegate/create", post(create_delegate))
        .route("/vault/delegate/revoke", post(revoke_delegate))
}

async fn get_vault_info(
    Path(user): Path<String>,
) -> Result<Json<VaultInfo>> {
    // 实现获取 Vault 信息
    todo!()
}

async fn get_user_delegates(
    Path(user): Path<String>,
) -> Result<Json<Vec<DelegateInfo>>> {
    // 实现获取用户的所有 delegates
    todo!()
}

async fn create_delegate(
    Json(req): Json<CreateDelegateRequest>,
) -> Result<Json<String>> {
    // 实现创建 delegate
    todo!()
}

async fn revoke_delegate(
    Json(req): Json<RevokeDelegateRequest>,
) -> Result<Json<String>> {
    // 实现撤销 delegate
    todo!()
}
```

---

## 前端集成 (1024-chain-frontend)

### 1. 创建 Vault SDK

**文件**: `src/lib/vault/index.ts`

```typescript
import { Connection, PublicKey, Transaction, TransactionInstruction } from '@solana/web3.js';
import { AnchorWallet } from '@solana/wallet-adapter-react';

export const VAULT_PROGRAM_ID = new PublicKey('9omyQr3wY5K5KyL53BQzLz9QTzAve6oYzg8LyfXFpsj8');
export const USDC_MINT = new PublicKey('6u1x12yV2XFcEDGd8KByZZqnjipRiq9BJB2xKprhAipy');

export class VaultClient {
  constructor(
    private connection: Connection,
    private wallet: AnchorWallet
  ) {}

  /**
   * 派生 Vault PDA
   */
  deriveVaultPDA(owner: PublicKey): [PublicKey, number] {
    return PublicKey.findProgramAddressSync(
      [Buffer.from('vault'), owner.toBuffer()],
      VAULT_PROGRAM_ID
    );
  }

  /**
   * 派生 Vault USDC Token Account PDA
   */
  deriveVaultUsdcPDA(owner: PublicKey): [PublicKey, number] {
    return PublicKey.findProgramAddressSync(
      [Buffer.from('vault-usdc'), owner.toBuffer()],
      VAULT_PROGRAM_ID
    );
  }

  /**
   * 检查用户是否已创建 Vault
   */
  async hasVault(owner: PublicKey): Promise<boolean> {
    const [vaultPDA] = this.deriveVaultPDA(owner);
    const accountInfo = await this.connection.getAccountInfo(vaultPDA);
    return accountInfo !== null && accountInfo.data.length > 0;
  }

  /**
   * 获取 Vault 余额
   */
  async getVaultBalance(owner: PublicKey): Promise<{
    freeCollateral: bigint;
    lockedCollateral: bigint;
    totalDeposit: bigint;
    totalWithdrawn: bigint;
  }> {
    const [vaultPDA] = this.deriveVaultPDA(owner);
    const accountInfo = await this.connection.getAccountInfo(vaultPDA);
    
    if (!accountInfo) {
      throw new Error('Vault not found');
    }

    // 解析 UserVault 数据 (简化版本)
    const data = accountInfo.data;
    
    // UserVault 结构:
    // discriminator: 8 bytes
    // version: 1 byte
    // bump: 1 byte
    // usdc_bump: 1 byte
    // reserved_align: 5 bytes
    // owner: 32 bytes
    // usdc_vault: 32 bytes
    // total_deposit: 8 bytes (offset: 80)
    // total_withdrawn: 8 bytes (offset: 88)
    // free_collateral: 8 bytes (offset: 96)
    // locked_collateral: 8 bytes (offset: 104)
    
    return {
      totalDeposit: data.readBigUInt64LE(80),
      totalWithdrawn: data.readBigUInt64LE(88),
      freeCollateral: data.readBigUInt64LE(96),
      lockedCollateral: data.readBigUInt64LE(104),
    };
  }

  /**
   * 创建 Vault
   */
  async createVault(): Promise<string> {
    // 实现创建 Vault 逻辑
    // 参考 test-vault.ts 中的代码
    throw new Error('Not implemented');
  }

  /**
   * 存款
   */
  async deposit(amount: bigint): Promise<string> {
    // 实现存款逻辑
    throw new Error('Not implemented');
  }

  /**
   * 提款
   */
  async withdraw(amount: bigint): Promise<string> {
    // 实现提款逻辑
    throw new Error('Not implemented');
  }
}
```

### 2. 创建 Vault 管理 UI

**文件**: `src/components/Vault/VaultDashboard.tsx`

```typescript
'use client';

import { useWallet } from '@solana/wallet-adapter-react';
import { useConnection } from '@solana/wallet-adapter-react';
import { useState, useEffect } from 'react';
import { VaultClient } from '@/lib/vault';
import { PublicKey } from '@solana/web3.js';

export function VaultDashboard() {
  const { connection } = useConnection();
  const { publicKey, sendTransaction } = useWallet();
  const [hasVault, setHasVault] = useState(false);
  const [balance, setBalance] = useState<{
    free: number;
    locked: number;
    total: number;
  } | null>(null);

  useEffect(() => {
    if (!publicKey) return;

    const client = new VaultClient(connection, wallet);
    
    // 检查是否有 Vault
    client.hasVault(publicKey).then(setHasVault);

    // 获取余额
    if (hasVault) {
      client.getVaultBalance(publicKey).then((bal) => {
        setBalance({
          free: Number(bal.freeCollateral) / 1e9,
          locked: Number(bal.lockedCollateral) / 1e9,
          total: Number(bal.freeCollateral + bal.lockedCollateral) / 1e9,
        });
      });
    }
  }, [publicKey, hasVault]);

  return (
    <div className="vault-dashboard">
      <h2>My Vault</h2>

      {!hasVault ? (
        <div>
          <p>You don't have a Vault yet.</p>
          <button onClick={() => {/* Create Vault */}}>
            Create Vault
          </button>
        </div>
      ) : (
        <div>
          <div className="balance-card">
            <h3>Balance</h3>
            <p>Total: {balance?.total} USDC</p>
            <p>Available: {balance?.free} USDC</p>
            <p>Locked: {balance?.locked} USDC</p>
          </div>

          <div className="actions">
            <button onClick={() => {/* Deposit */}}>Deposit</button>
            <button onClick={() => {/* Withdraw */}}>Withdraw</button>
          </div>
        </div>
      )}
    </div>
  );
}
```

### 3. 创建 API Key 管理 UI

**文件**: `src/components/Vault/APIKeyManager.tsx`

```typescript
'use client';

import { useState } from 'react';
import { Keypair } from '@solana/web3.js';
import bs58 from 'bs58';

export function APIKeyManager() {
  const [apiKeys, setApiKeys] = useState<Array<{
    publicKey: string;
    permissions: number;
    maxNotional: number;
    expiry: number;
    isActive: boolean;
  }>>([]);

  const createAPIKey = () => {
    const newKey = Keypair.generate();
    
    // 显示私钥给用户保存
    alert(`
      API Key Created!
      
      Public Key: ${newKey.publicKey.toBase58()}
      Private Key: ${bs58.encode(newKey.secretKey)}
      
      ⚠️ IMPORTANT: Save your private key securely!
      We will NOT store it.
    `);

    // 调用合约创建 delegate
    // ...
  };

  return (
    <div className="api-key-manager">
      <h2>API Keys</h2>

      <button onClick={createAPIKey}>
        Create New API Key
      </button>

      <div className="api-keys-list">
        {apiKeys.map((key) => (
          <div key={key.publicKey} className="api-key-card">
            <p>Public Key: {key.publicKey}</p>
            <p>Permissions: {key.permissions}</p>
            <p>Max Notional: {key.maxNotional} USDC</p>
            <p>Status: {key.isActive ? 'Active' : 'Revoked'}</p>
            
            {key.isActive && (
              <button onClick={() => {/* Revoke */}}>
                Revoke
              </button>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
```

---

## SDK 开发

### TypeScript SDK 结构

```
vault-sdk/
├── src/
│   ├── index.ts           # 主入口
│   ├── client.ts          # VaultClient
│   ├── instructions.ts    # 指令构造器
│   ├── types.ts           # 类型定义
│   └── utils.ts           # 工具函数
├── package.json
└── tsconfig.json
```

### 发布到 NPM

```json
{
  "name": "@1024/vault-sdk",
  "version": "0.1.0",
  "main": "dist/index.js",
  "types": "dist/index.d.ts",
  "scripts": {
    "build": "tsc",
    "prepublish": "npm run build"
  }
}
```

---

## 测试指南

### 1. 单元测试

```bash
# 运行测试脚本
cd scripts
npm run test
```

### 2. 集成测试

**后端测试**:
```bash
cd 1024-core
cargo test --all
```

**前端测试**:
```bash
cd 1024-chain-frontend
npm test
```

### 3. E2E 测试

创建完整的用户流程测试：
1. 连接钱包
2. 创建 Vault
3. 存款
4. 创建 API Key
5. 使用 API Key 交易
6. 提款
7. 撤销 API Key

---

## 部署清单

### Testnet 部署

- [x] Program 已部署
- [x] GlobalConfig 已初始化
- [ ] 后端集成完成
- [ ] 前端集成完成
- [ ] E2E 测试通过

### Mainnet 部署准备

- [ ] 外部安全审计
- [ ] 压力测试
- [ ] 用户文档
- [ ] 运维手册
- [ ] 监控告警
- [ ] 备份方案

---

## 相关资源

- **部署指南**: `DEPLOYMENT_GUIDE.md`
- **设计文档**: `design.md`
- **测试脚本**: `scripts/test-vault.ts`
- **代码审计**: `../CODE_AUDIT_REPORT.md`

---

**文档版本**: v1.0  
**最后更新**: 2025-11-17  
**维护者**: 1024 Team

