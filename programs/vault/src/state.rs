//! Vault Program State Definitions
//! 
//! 定义 Vault Program 的核心数据结构：
//! - GlobalConfig: 全局配置（单例）
//! - UserVault: 用户金库
//! - DelegateAccount: API Key 授权记录

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{pubkey::Pubkey, clock::Clock, sysvar::Sysvar};

/// 全局配置（单例PDA）
/// PDA Seeds: [b"global", version]
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct GlobalConfig {
    /// 账户类型标识符 "GLBCFG" = 0x474c4243_46470000
    pub discriminator: u64,
    
    /// 配置版本（预留多版本支持）
    pub version: u8,
    
    /// PDA bump seed
    pub bump: u8,
    
    /// 预留字段（对齐）
    pub reserved_align: [u8; 6],
    
    /// Program 管理员（可置空实现完全非托管）
    pub admin: Pubkey,
    
    /// 支持的 USDC Mint 地址
    pub usdc_mint: Pubkey,
    
    /// 创建时间戳（秒）
    pub created_at: i64,
    
    /// 预留扩展字段
    pub reserved: [u8; 64],
}

impl GlobalConfig {
    pub const DISCRIMINATOR: u64 = 0x474c4243_46470000;
    pub const VERSION: u8 = 1;
    
    /// 8 + 1 + 1 + 6 + 32 + 32 + 8 + 64 = 152 bytes
    pub const SIZE: usize = 152;
    
    pub fn new(admin: Pubkey, usdc_mint: Pubkey, bump: u8) -> Self {
        let now = Clock::get()
            .map(|clock| clock.unix_timestamp)
            .unwrap_or(0);
        
        Self {
            discriminator: Self::DISCRIMINATOR,
            version: Self::VERSION,
            bump,
            reserved_align: [0; 6],
            admin,
            usdc_mint,
            created_at: now,
            reserved: [0; 64],
        }
    }
}

/// 用户金库（每个用户一个PDA）
/// PDA Seeds: [b"vault", owner_wallet]
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct UserVault {
    /// 账户类型标识符 "USERVLT" = 0x55534552_564c5400
    pub discriminator: u64,
    
    /// 数据版本
    pub version: u8,
    
    /// PDA bump seed (for UserVault PDA)
    pub bump: u8,
    
    /// Vault USDC Token Account bump seed
    pub usdc_bump: u8,
    
    /// 预留字段（对齐）
    pub reserved_align: [u8; 5],
    
    /// Vault 所有者（用户主钱包）
    pub owner: Pubkey,
    
    /// Vault 对应的 USDC Token Account PDA
    pub usdc_vault: Pubkey,
    
    /// 历史总存入（e6格式）
    pub total_deposit: u64,
    
    /// 历史总提出（e6格式）
    pub total_withdrawn: u64,
    
    /// 可用保证金（e6格式）
    pub free_collateral: u64,
    
    /// 锁定保证金（e6格式，由业务程序更新）
    pub locked_collateral: u64,
    
    /// 状态标记位
    /// Bit 0: is_frozen (冻结)
    /// Bit 1-63: 预留
    pub flags: u64,
    
    /// 创建时间戳（秒）
    pub created_at: i64,
    
    /// 更新时间戳（秒）
    pub updated_at: i64,
    
    /// 预留扩展字段
    pub reserved: [u8; 64],
}

impl UserVault {
    pub const DISCRIMINATOR: u64 = 0x55534552_564c5400;
    pub const VERSION: u8 = 1;
    
    /// 8 + 1 + 1 + 1 + 5 + 32 + 32 + 8*6 + 8 + 8 + 64 = 208 bytes
    pub const SIZE: usize = 208;
    
    pub fn new(owner: Pubkey, usdc_vault: Pubkey, bump: u8, usdc_bump: u8) -> Self {
        let now = Clock::get()
            .map(|clock| clock.unix_timestamp)
            .unwrap_or(0);
        
        Self {
            discriminator: Self::DISCRIMINATOR,
            version: Self::VERSION,
            bump,
            usdc_bump,
            reserved_align: [0; 5],
            owner,
            usdc_vault,
            total_deposit: 0,
            total_withdrawn: 0,
            free_collateral: 0,
            locked_collateral: 0,
            flags: 0,
            created_at: now,
            updated_at: now,
            reserved: [0; 64],
        }
    }
    
    /// 检查 vault 是否冻结
    pub fn is_frozen(&self) -> bool {
        self.flags & 1 != 0
    }
    
    /// 冻结 vault
    pub fn freeze(&mut self) {
        self.flags |= 1;
        self.update_timestamp();
    }
    
    /// 解冻 vault
    pub fn unfreeze(&mut self) {
        self.flags &= !1;
        self.update_timestamp();
    }
    
    /// 更新时间戳
    pub fn update_timestamp(&mut self) {
        self.updated_at = Clock::get()
            .map(|clock| clock.unix_timestamp)
            .unwrap_or(0);
    }
}

/// 权限位定义
pub const PERM_TRADE: u64 = 1 << 0;          // 允许交易（开平仓）
pub const PERM_WITHDRAW: u64 = 1 << 1;       // 允许提现
pub const PERM_CLOSE_ONLY: u64 = 1 << 2;     // 只允许平仓（减仓）
pub const PERM_VIEW_ONLY: u64 = 1 << 3;      // 只读权限（未来扩展）

/// API Key 授权记录（每个 vault × delegate 一条记录）
/// PDA Seeds: [b"delegate", owner_wallet, delegate_pubkey]
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct DelegateAccount {
    /// 账户类型标识符 "DELEGATE" = 0x44454c45_47415445
    pub discriminator: u64,
    
    /// 数据版本
    pub version: u8,
    
    /// PDA bump seed
    pub bump: u8,
    
    /// 预留字段（对齐）
    pub reserved_align: [u8; 6],
    
    /// Vault 所有者
    pub owner: Pubkey,
    
    /// 对应的 UserVault PDA
    pub vault: Pubkey,
    
    /// 被授权的 delegate 公钥（API Key）
    pub delegate: Pubkey,
    
    /// 是否激活
    pub is_active: bool,
    
    /// 预留字段（对齐）
    pub reserved_align2: [u8; 7],
    
    /// 权限位掩码（使用上述 PERM_* 常量）
    pub permissions: u64,
    
    /// 最大可用名义敞口（e6格式）
    pub max_notional: u64,
    
    /// 当前已使用的名义敞口（e6格式，由业务程序更新）
    pub used_notional: u64,
    
    /// 过期时间（slot number）
    pub expiry_slot: u64,
    
    /// 防重放计数器（每次操作递增）
    pub nonce: u64,
    
    /// 创建时间戳（秒）
    pub created_at: i64,
    
    /// 更新时间戳（秒）
    pub updated_at: i64,
    
    /// 预留扩展字段
    pub reserved: [u8; 64],
}

impl DelegateAccount {
    pub const DISCRIMINATOR: u64 = 0x44454c45_47415445;
    pub const VERSION: u8 = 1;
    
    /// 8 + 1 + 1 + 6 + 32*3 + 1 + 7 + 8*5 + 8 + 8 + 64 = 237 bytes
    pub const SIZE: usize = 237;
    
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        owner: Pubkey,
        vault: Pubkey,
        delegate: Pubkey,
        permissions: u64,
        max_notional: u64,
        expiry_slot: u64,
        bump: u8,
    ) -> Self {
        let now = Clock::get()
            .map(|clock| clock.unix_timestamp)
            .unwrap_or(0);
        
        Self {
            discriminator: Self::DISCRIMINATOR,
            version: Self::VERSION,
            bump,
            reserved_align: [0; 6],
            owner,
            vault,
            delegate,
            is_active: true,
            reserved_align2: [0; 7],
            permissions,
            max_notional,
            used_notional: 0,
            expiry_slot,
            nonce: 0,
            created_at: now,
            updated_at: now,
            reserved: [0; 64],
        }
    }
    
    /// 检查是否有指定权限
    pub fn has_permission(&self, permission: u64) -> bool {
        self.permissions & permission != 0
    }
    
    /// 检查是否在有效期内
    pub fn is_valid(&self, current_slot: u64) -> bool {
        self.is_active && current_slot <= self.expiry_slot
    }
    
    /// 检查是否可以使用指定的名义敞口
    pub fn can_use_notional(&self, additional_notional: u64) -> bool {
        self.used_notional.saturating_add(additional_notional) <= self.max_notional
    }
    
    /// 更新时间戳
    pub fn update_timestamp(&mut self) {
        self.updated_at = Clock::get()
            .map(|clock| clock.unix_timestamp)
            .unwrap_or(0);
    }
}

