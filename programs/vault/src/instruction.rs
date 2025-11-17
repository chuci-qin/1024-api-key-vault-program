//! Vault Program Instructions

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum VaultInstruction {
    /// 初始化全局配置（仅一次，由管理员调用）
    /// 
    /// Accounts:
    /// 0. `[writable]` GlobalConfig PDA - 将被创建
    /// 1. `[signer, writable]` Admin - 管理员，支付租金
    /// 2. `[]` System Program
    /// 3. `[]` Rent Sysvar
    InitializeGlobalConfig {
        usdc_mint: Pubkey,
    },
    
    /// 创建用户 Vault
    /// 
    /// Accounts:
    /// 0. `[writable]` UserVault PDA - 将被创建
    /// 1. `[writable]` Vault USDC Token Account - 将被创建
    /// 2. `[signer, writable]` Owner - 用户主钱包，支付租金
    /// 3. `[]` GlobalConfig PDA
    /// 4. `[]` USDC Mint
    /// 5. `[]` System Program
    /// 6. `[]` Token Program
    /// 7. `[]` Rent Sysvar
    CreateVault,
    
    /// 存款：用户钱包 → Vault
    /// 
    /// Accounts:
    /// 0. `[writable]` UserVault PDA
    /// 1. `[signer]` Owner - 用户主钱包
    /// 2. `[writable]` Owner USDC Account - 用户的 USDC 账户
    /// 3. `[writable]` Vault USDC Account - Vault 的 USDC 账户
    /// 4. `[]` GlobalConfig PDA
    /// 5. `[]` Token Program
    Deposit {
        amount: u64,
    },
    
    /// 提款：Vault → 用户钱包
    /// 
    /// Accounts:
    /// 0. `[writable]` UserVault PDA
    /// 1. `[signer]` Signer - Owner 或有 PERM_WITHDRAW 权限的 delegate
    /// 2. `[writable]` Owner USDC Account - 目标账户（必须是 owner 的）
    /// 3. `[writable]` Vault USDC Account - Vault 的 USDC 账户
    /// 4. `[]` GlobalConfig PDA
    /// 5. `[]` Token Program
    /// 6. `[writable, optional]` DelegateAccount PDA - 如果 signer 是 delegate
    Withdraw {
        amount: u64,
    },
    
    /// 添加/更新 API Key（Delegate）
    /// 
    /// Accounts:
    /// 0. `[writable]` DelegateAccount PDA - 将被创建或更新
    /// 1. `[writable]` UserVault PDA
    /// 2. `[signer, writable]` Owner - 用户主钱包，支付租金（如果创建）
    /// 3. `[]` GlobalConfig PDA
    /// 4. `[]` System Program
    UpsertDelegate {
        delegate_pubkey: Pubkey,
        permissions: u64,
        max_notional: u64,
        expiry_slot: u64,
    },
    
    /// 撤销 API Key（Delegate）
    /// 
    /// Accounts:
    /// 0. `[writable]` DelegateAccount PDA
    /// 1. `[writable]` UserVault PDA
    /// 2. `[signer]` Owner - 用户主钱包
    /// 3. `[]` GlobalConfig PDA
    RevokeDelegate {
        delegate_pubkey: Pubkey,
    },
    
    /// 锁定保证金（由业务程序 CPI 调用）
    /// 
    /// Accounts:
    /// 0. `[writable]` UserVault PDA
    /// 1. `[signer]` Signer - Owner 或有 PERM_TRADE 权限的 delegate
    /// 2. `[writable, optional]` DelegateAccount PDA - 如果 signer 是 delegate
    /// 3. `[]` GlobalConfig PDA
    /// 4. `[]` Clock Sysvar
    LockMargin {
        required_margin: u64,
        required_notional: u64,
    },
    
    /// 解锁保证金并更新 PnL（由业务程序 CPI 调用）
    /// 
    /// Accounts:
    /// 0. `[writable]` UserVault PDA
    /// 1. `[signer]` Signer - Owner 或有 PERM_TRADE 权限的 delegate
    /// 2. `[writable, optional]` DelegateAccount PDA - 如果 signer 是 delegate
    /// 3. `[]` GlobalConfig PDA
    UnlockMarginAndUpdatePnl {
        unlocked_margin: u64,
        pnl_delta: i64,          // 正数为盈利，负数为亏损
        notional_delta: i64,     // 释放的名义敞口（可为负）
    },
    
    /// 转移 Admin 权限
    /// 
    /// Accounts:
    /// 0. `[writable]` GlobalConfig PDA
    /// 1. `[signer]` Current Admin
    TransferAdmin {
        new_admin: Pubkey,
    },
    
    /// 放弃 Admin 权限（设为 Pubkey::default()，实现完全非托管）
    /// 
    /// Accounts:
    /// 0. `[writable]` GlobalConfig PDA
    /// 1. `[signer]` Current Admin
    RenounceAdmin,
    
    /// 冻结 Vault（仅 owner 可调用）
    /// 
    /// Accounts:
    /// 0. `[writable]` UserVault PDA
    /// 1. `[signer]` Owner
    FreezeVault,
    
    /// 解冻 Vault（仅 owner 可调用）
    /// 
    /// Accounts:
    /// 0. `[writable]` UserVault PDA
    /// 1. `[signer]` Owner
    UnfreezeVault,
}

