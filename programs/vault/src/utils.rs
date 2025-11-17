//! Vault Program Utility Functions

use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
};
use spl_token::state::Account as TokenAccount;
use crate::error::VaultError;

/// 创建 PDA 账户
pub fn create_pda_account<'a>(
    payer: &AccountInfo<'a>,
    pda: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    program_id: &Pubkey,
    rent: &Rent,
    space: usize,
    seeds: &[&[u8]],
) -> ProgramResult {
    let required_lamports = rent
        .minimum_balance(space)
        .saturating_sub(pda.lamports());
    
    if required_lamports > 0 {
        invoke(
            &system_instruction::transfer(payer.key, pda.key, required_lamports),
            &[payer.clone(), pda.clone(), system_program.clone()],
        )?;
    }
    
    invoke_signed(
        &system_instruction::allocate(pda.key, space as u64),
        &[pda.clone(), system_program.clone()],
        &[seeds],
    )?;
    
    invoke_signed(
        &system_instruction::assign(pda.key, program_id),
        &[pda.clone(), system_program.clone()],
        &[seeds],
    )?;
    
    Ok(())
}

/// 验证 PDA
pub fn verify_pda(
    pda: &Pubkey,
    program_id: &Pubkey,
    seeds: &[&[u8]],
) -> Result<u8, ProgramError> {
    let (derived_pda, bump) = Pubkey::find_program_address(seeds, program_id);
    if derived_pda != *pda {
        return Err(ProgramError::InvalidSeeds);
    }
    Ok(bump)
}

/// SPL Token 转账（普通）
pub fn token_transfer<'a>(
    token_program: &AccountInfo<'a>,
    source: &AccountInfo<'a>,
    destination: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    amount: u64,
) -> ProgramResult {
    let transfer_instruction = spl_token::instruction::transfer(
        token_program.key,
        source.key,
        destination.key,
        authority.key,
        &[],
        amount,
    )?;
    
    invoke(
        &transfer_instruction,
        &[
            source.clone(),
            destination.clone(),
            authority.clone(),
            token_program.clone(),
        ],
    )
}

/// SPL Token 转账（使用 PDA 签名）
pub fn token_transfer_signed<'a>(
    token_program: &AccountInfo<'a>,
    source: &AccountInfo<'a>,
    destination: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    amount: u64,
    signer_seeds: &[&[u8]],
) -> ProgramResult {
    let transfer_instruction = spl_token::instruction::transfer(
        token_program.key,
        source.key,
        destination.key,
        authority.key,
        &[],
        amount,
    )?;
    
    invoke_signed(
        &transfer_instruction,
        &[
            source.clone(),
            destination.clone(),
            authority.clone(),
            token_program.clone(),
        ],
        &[signer_seeds],
    )
}

/// 验证 Token Account 的 owner 和 mint
pub fn verify_token_account(
    token_account_info: &AccountInfo,
    expected_owner: &Pubkey,
    expected_mint: &Pubkey,
) -> Result<TokenAccount, ProgramError> {
    let token_account = TokenAccount::unpack(&token_account_info.data.borrow())?;
    
    if token_account.owner != *expected_owner {
        return Err(VaultError::InvalidTokenAccount.into());
    }
    
    if token_account.mint != *expected_mint {
        return Err(VaultError::InvalidTokenMint.into());
    }
    
    Ok(token_account)
}

/// 安全加法（检查溢出）
pub fn safe_add(a: u64, b: u64) -> Result<u64, ProgramError> {
    a.checked_add(b)
        .ok_or(VaultError::ArithmeticOverflow.into())
}

/// 安全减法（检查下溢）
pub fn safe_sub(a: u64, b: u64) -> Result<u64, ProgramError> {
    a.checked_sub(b)
        .ok_or(VaultError::ArithmeticUnderflow.into())
}

/// 安全的有符号加法（用于 PnL 计算）
pub fn safe_add_signed(a: u64, delta: i64) -> Result<u64, ProgramError> {
    if delta >= 0 {
        a.checked_add(delta as u64)
            .ok_or(VaultError::ArithmeticOverflow.into())
    } else {
        a.checked_sub(delta.unsigned_abs())
            .ok_or(VaultError::ArithmeticUnderflow.into())
    }
}

/// 检查账户是否为签名者
pub fn require_signer(account: &AccountInfo) -> ProgramResult {
    if !account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    Ok(())
}

/// 检查账户是否可写
pub fn require_writable(account: &AccountInfo) -> ProgramResult {
    if !account.is_writable {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

/// 检查账户所有者
pub fn require_owner(account: &AccountInfo, expected_owner: &Pubkey) -> ProgramResult {
    if account.owner != expected_owner {
        return Err(ProgramError::IllegalOwner);
    }
    Ok(())
}

/// 验证 Vault 余额一致性
/// 
/// 确保 Token Account 的实际余额 = free_collateral + locked_collateral
/// 
/// # 参数
/// - `vault`: UserVault 账户
/// - `vault_usdc_info`: Vault 的 USDC Token Account
/// 
/// # 错误
/// - `VaultError::InvalidTokenAccount` - 余额不一致
pub fn verify_vault_balance_integrity(
    vault: &crate::state::UserVault,
    vault_usdc_info: &AccountInfo,
) -> ProgramResult {
    use solana_program::msg;
    use crate::error::VaultError;
    
    let token_account = TokenAccount::unpack(&vault_usdc_info.data.borrow())?;
    
    let expected_balance = vault.free_collateral
        .checked_add(vault.locked_collateral)
        .ok_or(VaultError::ArithmeticOverflow)?;
    
    if token_account.amount != expected_balance {
        msg!("❌ Balance mismatch detected!");
        msg!("Expected: {} (free: {} + locked: {})", 
            expected_balance, vault.free_collateral, vault.locked_collateral);
        msg!("Actual token balance: {}", token_account.amount);
        return Err(VaultError::InvalidTokenAccount.into());
    }
    
    Ok(())
}

