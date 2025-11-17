//! 1024 API Key Vault Program
//! 
//! Non-custodial vault with multi-API-key authorization for 1024EX
//! 
//! 核心特性：
//! - 用户专属金库（UserVault）- 非托管保证金托管
//! - 多 API Key 授权（Delegate）- 支持量化策略自动交易
//! - 精细权限控制 - Trade / Withdraw / Close-only
//! - 风险限额管理 - 每个 API Key 独立限额
//! - 可撤销授权 - 随时作废 API Key
//! 
//! GitHub: https://github.com/1024-org/1024-api-key-vault-program
//! Version: 0.1.0
//! Author: Chuci Qin

use solana_program::{
    account_info::AccountInfo,
    entrypoint,
    entrypoint::ProgramResult,
    pubkey::Pubkey,
};

pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;
pub mod utils;

// Vault Program ID（占位，部署后更新）
// 部署后需要更新为实际的 Program ID
solana_program::declare_id!("VAu1tProg1111111111111111111111111111111111");

// Program 入口点
#[cfg(not(feature = "no-entrypoint"))]
entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    processor::process_instruction(program_id, accounts, instruction_data)
}

// 导出公共类型
pub use error::VaultError;
pub use instruction::VaultInstruction;
pub use state::{
    DelegateAccount, GlobalConfig, UserVault, 
    PERM_CLOSE_ONLY, PERM_TRADE, PERM_VIEW_ONLY, PERM_WITHDRAW,
};

