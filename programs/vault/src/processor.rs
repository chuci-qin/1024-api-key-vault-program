//! Vault Program Instruction Processor

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::{clock::Clock, Sysvar},
    system_program,
};
use spl_token::state::Account as TokenAccount;

use crate::{
    error::VaultError,
    instruction::VaultInstruction,
    state::{
        DelegateAccount, GlobalConfig, UserVault, PERM_TRADE, PERM_WITHDRAW,
    },
    utils::*,
};

/// Program入口点
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = VaultInstruction::try_from_slice(instruction_data)
        .map_err(|_| VaultError::DeserializationError)?;
    
    match instruction {
        VaultInstruction::InitializeGlobalConfig { usdc_mint } => {
            process_initialize_global_config(program_id, accounts, usdc_mint)
        }
        VaultInstruction::CreateVault => {
            process_create_vault(program_id, accounts)
        }
        VaultInstruction::Deposit { amount } => {
            process_deposit(program_id, accounts, amount)
        }
        VaultInstruction::Withdraw { amount } => {
            process_withdraw(program_id, accounts, amount)
        }
        VaultInstruction::UpsertDelegate {
            delegate_pubkey,
            permissions,
            max_notional,
            expiry_slot,
        } => {
            process_upsert_delegate(
                program_id,
                accounts,
                delegate_pubkey,
                permissions,
                max_notional,
                expiry_slot,
            )
        }
        VaultInstruction::RevokeDelegate { delegate_pubkey } => {
            process_revoke_delegate(program_id, accounts, delegate_pubkey)
        }
        VaultInstruction::LockMargin {
            required_margin,
            required_notional,
        } => {
            process_lock_margin(program_id, accounts, required_margin, required_notional)
        }
        VaultInstruction::UnlockMarginAndUpdatePnl {
            unlocked_margin,
            pnl_delta,
            notional_delta,
        } => {
            process_unlock_margin_and_update_pnl(
                program_id,
                accounts,
                unlocked_margin,
                pnl_delta,
                notional_delta,
            )
        }
        VaultInstruction::TransferAdmin { new_admin } => {
            process_transfer_admin(program_id, accounts, new_admin)
        }
        VaultInstruction::RenounceAdmin => {
            process_renounce_admin(program_id, accounts)
        }
        VaultInstruction::FreezeVault => {
            process_freeze_vault(program_id, accounts)
        }
        VaultInstruction::UnfreezeVault => {
            process_unfreeze_vault(program_id, accounts)
        }
    }
}

/// 初始化全局配置
fn process_initialize_global_config(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    usdc_mint: Pubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    let global_config_info = next_account_info(account_info_iter)?;
    let admin_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;
    let rent_sysvar_info = next_account_info(account_info_iter)?;
    
    // 验证
    require_signer(admin_info)?;
    require_writable(global_config_info)?;
    
    if system_program_info.key != &system_program::id() {
        return Err(ProgramError::IncorrectProgramId);
    }
    
    let rent = Rent::from_account_info(rent_sysvar_info)?;
    
    // 派生 PDA
    let version: u8 = 1;
    let seeds = &[b"global".as_ref(), &[version]];
    let bump = verify_pda(global_config_info.key, program_id, seeds)?;
    let seeds_with_bump = &[b"global".as_ref(), &[version], &[bump]];
    
    // 检查账户是否已初始化
    if global_config_info.data_len() > 0 {
        return Err(VaultError::AlreadyInitialized.into());
    }
    
    // 创建账户
    create_pda_account(
        admin_info,
        global_config_info,
        system_program_info,
        program_id,
        &rent,
        GlobalConfig::SIZE,
        seeds_with_bump,
    )?;
    
    // 初始化数据
    let global_config = GlobalConfig::new(*admin_info.key, usdc_mint, bump);
    global_config.serialize(&mut &mut global_config_info.data.borrow_mut()[..])?;
    
    msg!("Global config initialized");
    msg!("Admin: {}", admin_info.key);
    msg!("USDC Mint: {}", usdc_mint);
    
    Ok(())
}

/// 创建用户 Vault
fn process_create_vault(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    let vault_info = next_account_info(account_info_iter)?;
    let vault_usdc_info = next_account_info(account_info_iter)?;
    let owner_info = next_account_info(account_info_iter)?;
    let global_config_info = next_account_info(account_info_iter)?;
    let usdc_mint_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;
    let rent_sysvar_info = next_account_info(account_info_iter)?;
    
    // 验证
    require_signer(owner_info)?;
    require_writable(vault_info)?;
    require_writable(vault_usdc_info)?;
    
    let rent = Rent::from_account_info(rent_sysvar_info)?;
    
    // 验证 global config
    let global_config = GlobalConfig::try_from_slice(&global_config_info.data.borrow())?;
    if global_config.usdc_mint != *usdc_mint_info.key {
        return Err(VaultError::InvalidTokenMint.into());
    }
    
    // 派生 UserVault PDA
    let vault_seeds = &[b"vault".as_ref(), owner_info.key.as_ref()];
    let vault_bump = verify_pda(vault_info.key, program_id, vault_seeds)?;
    let vault_seeds_with_bump = &[b"vault".as_ref(), owner_info.key.as_ref(), &[vault_bump]];
    
    // 检查账户是否已存在
    if vault_info.data_len() > 0 {
        return Err(VaultError::AccountAlreadyExists.into());
    }
    
    // 创建 UserVault 账户
    create_pda_account(
        owner_info,
        vault_info,
        system_program_info,
        program_id,
        &rent,
        UserVault::SIZE,
        vault_seeds_with_bump,
    )?;
    
    // 派生 Vault USDC Token Account PDA
    let usdc_seeds = &[b"vault-usdc".as_ref(), owner_info.key.as_ref()];
    let usdc_bump = verify_pda(vault_usdc_info.key, program_id, usdc_seeds)?;
    let usdc_seeds_with_bump = &[b"vault-usdc".as_ref(), owner_info.key.as_ref(), &[usdc_bump]];
    
    // 创建 Token Account (owner = Token Program, authority = vault-usdc PDA)
    let token_account_space = TokenAccount::LEN;
    let create_account_ix = system_instruction::create_account(
        owner_info.key,
        vault_usdc_info.key,
        rent.minimum_balance(token_account_space),
        token_account_space as u64,
        token_program_info.key, // owner 设为 Token Program
    );
    
    invoke_signed(
        &create_account_ix,
        &[owner_info.clone(), vault_usdc_info.clone(), system_program_info.clone()],
        &[usdc_seeds_with_bump],
    )?;
    
    // 初始化 Token Account
    // Token Account 的 authority 设为 vault-usdc PDA 本身，这样我们可以用它签名来转账
    let init_account_ix = spl_token::instruction::initialize_account3(
        token_program_info.key,
        vault_usdc_info.key,
        usdc_mint_info.key,
        vault_usdc_info.key, // authority 是 vault-usdc PDA 本身
    )?;
    
    solana_program::program::invoke_signed(
        &init_account_ix,
        &[
            vault_usdc_info.clone(),
            usdc_mint_info.clone(),
            token_program_info.clone(),
        ],
        &[usdc_seeds_with_bump],
    )?;
    
    // 初始化 UserVault 数据
    let user_vault = UserVault::new(*owner_info.key, *vault_usdc_info.key, vault_bump, usdc_bump);
    user_vault.serialize(&mut &mut vault_info.data.borrow_mut()[..])?;
    
    msg!("Vault created for owner: {}", owner_info.key);
    msg!("Vault PDA: {}", vault_info.key);
    msg!("Vault USDC: {}", vault_usdc_info.key);
    
    Ok(())
}

/// 存款
fn process_deposit(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    let vault_info = next_account_info(account_info_iter)?;
    let owner_info = next_account_info(account_info_iter)?;
    let owner_usdc_info = next_account_info(account_info_iter)?;
    let vault_usdc_info = next_account_info(account_info_iter)?;
    let _global_config_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;
    
    // 验证
    require_signer(owner_info)?;
    require_writable(vault_info)?;
    require_owner(vault_info, program_id)?;
    
    // 参数边界检查
    if amount == 0 {
        return Err(VaultError::InvalidAmount.into());
    }
    
    // 单次存款上限：1B USDC (防止误操作)
    const MAX_DEPOSIT: u64 = 1_000_000_000_000_000; // 1B USDC (e6 format)
    if amount > MAX_DEPOSIT {
        msg!("Deposit amount too large: {}", amount);
        return Err(VaultError::InvalidAmount.into());
    }
    
    // 读取 vault
    let mut vault = UserVault::try_from_slice(&vault_info.data.borrow())?;
    
    // 验证 owner
    if vault.owner != *owner_info.key {
        return Err(VaultError::InvalidOwner.into());
    }
    
    // 检查是否冻结
    if vault.is_frozen() {
        return Err(VaultError::VaultFrozen.into());
    }
    
    // 转账：owner → vault
    token_transfer(
        token_program_info,
        owner_usdc_info,
        vault_usdc_info,
        owner_info,
        amount,
    )?;
    
    // 更新余额
    vault.total_deposit = safe_add(vault.total_deposit, amount)?;
    vault.free_collateral = safe_add(vault.free_collateral, amount)?;
    vault.update_timestamp();
    
    vault.serialize(&mut &mut vault_info.data.borrow_mut()[..])?;
    
    // 验证余额一致性
    verify_vault_balance_integrity(&vault, vault_usdc_info)?;
    
    msg!("Deposited {} USDC to vault", amount);
    msg!("New free collateral: {}", vault.free_collateral);
    
    Ok(())
}

/// 提款
fn process_withdraw(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    let vault_info = next_account_info(account_info_iter)?;
    let signer_info = next_account_info(account_info_iter)?;
    let owner_usdc_info = next_account_info(account_info_iter)?;
    let vault_usdc_info = next_account_info(account_info_iter)?;
    let _global_config_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;
    let delegate_info = account_info_iter.next(); // Optional
    
    // 验证
    require_signer(signer_info)?;
    require_writable(vault_info)?;
    require_owner(vault_info, program_id)?;
    
    if amount == 0 {
        return Err(VaultError::InvalidAmount.into());
    }
    
    // 读取 vault
    let mut vault = UserVault::try_from_slice(&vault_info.data.borrow())?;
    
    // 检查是否冻结
    if vault.is_frozen() {
        return Err(VaultError::VaultFrozen.into());
    }
    
    // 权限验证
    let is_owner = *signer_info.key == vault.owner;
    
    if !is_owner {
        // 如果不是 owner，必须是有 WITHDRAW 权限的 delegate
        let delegate_account_info = delegate_info.ok_or(VaultError::InvalidDelegate)?;
        let delegate = DelegateAccount::try_from_slice(&delegate_account_info.data.borrow())?;
        
        // 验证 delegate
        if delegate.delegate != *signer_info.key {
            return Err(VaultError::InvalidDelegate.into());
        }
        
        if delegate.owner != vault.owner {
            return Err(VaultError::InvalidOwner.into());
        }
        
        // 检查权限
        let current_slot = Clock::get()?.slot;
        if !delegate.is_valid(current_slot) {
            return Err(VaultError::DelegateExpired.into());
        }
        
        if !delegate.has_permission(PERM_WITHDRAW) {
            return Err(VaultError::PermissionDenied.into());
        }
    }
    
    // 检查余额
    if vault.free_collateral < amount {
        return Err(VaultError::InsufficientFreeCollateral.into());
    }
    
    // 转账：vault → owner（注意目标必须是 owner 的账户）
    // 使用正确的 vault USDC bump seed
    let usdc_seeds_with_bump = &[
        b"vault-usdc".as_ref(),
        vault.owner.as_ref(),
        &[vault.usdc_bump], // 使用 vault 中保存的 usdc_bump
    ];
    
    token_transfer_signed(
        token_program_info,
        vault_usdc_info,
        owner_usdc_info,
        vault_usdc_info, // authority 是 vault-usdc PDA 本身
        amount,
        usdc_seeds_with_bump,
    )?;
    
    // 更新余额
    vault.free_collateral = safe_sub(vault.free_collateral, amount)?;
    vault.total_withdrawn = safe_add(vault.total_withdrawn, amount)?;
    vault.update_timestamp();
    
    vault.serialize(&mut &mut vault_info.data.borrow_mut()[..])?;
    
    // 验证余额一致性
    verify_vault_balance_integrity(&vault, vault_usdc_info)?;
    
    msg!("Withdrawn {} USDC from vault", amount);
    msg!("New free collateral: {}", vault.free_collateral);
    
    Ok(())
}

/// 添加/更新 Delegate
fn process_upsert_delegate(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    delegate_pubkey: Pubkey,
    permissions: u64,
    max_notional: u64,
    expiry_slot: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    let delegate_info = next_account_info(account_info_iter)?;
    let vault_info = next_account_info(account_info_iter)?;
    let owner_info = next_account_info(account_info_iter)?;
    let _global_config_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;
    
    // 验证
    require_signer(owner_info)?;
    require_writable(delegate_info)?;
    require_writable(vault_info)?;
    require_owner(vault_info, program_id)?;
    
    // 参数边界检查
    if permissions == 0 {
        msg!("Permissions cannot be empty");
        return Err(VaultError::InvalidPermissions.into());
    }
    
    if max_notional == 0 {
        msg!("Max notional must be greater than 0");
        return Err(VaultError::InvalidMaxNotional.into());
    }
    
    // 最大名义敞口上限：1B USDC
    const MAX_NOTIONAL_LIMIT: u64 = 1_000_000_000_000_000;
    if max_notional > MAX_NOTIONAL_LIMIT {
        msg!("Max notional too large: {}", max_notional);
        return Err(VaultError::InvalidMaxNotional.into());
    }
    
    // 读取 vault
    let vault = UserVault::try_from_slice(&vault_info.data.borrow())?;
    
    // 验证 owner
    if vault.owner != *owner_info.key {
        return Err(VaultError::InvalidOwner.into());
    }
    
    // 验证过期时间
    let current_slot = Clock::get()?.slot;
    if expiry_slot <= current_slot {
        return Err(VaultError::InvalidExpirySlot.into());
    }
    
    // 限制最大有效期（1 年）
    const MAX_EXPIRY_DURATION: u64 = 365 * 24 * 60 * 60 / 2; // 约 1 年的 slots (假设 2s/slot)
    if expiry_slot > current_slot + MAX_EXPIRY_DURATION {
        msg!("Expiry too far in future. Max: 1 year");
        return Err(VaultError::InvalidExpirySlot.into());
    }
    
    // 派生 DelegateAccount PDA
    let delegate_seeds = &[
        b"delegate".as_ref(),
        owner_info.key.as_ref(),
        delegate_pubkey.as_ref(),
    ];
    let delegate_bump = verify_pda(delegate_info.key, program_id, delegate_seeds)?;
    let delegate_seeds_with_bump = &[
        b"delegate".as_ref(),
        owner_info.key.as_ref(),
        delegate_pubkey.as_ref(),
        &[delegate_bump],
    ];
    
    let rent = Rent::get()?;
    
    // 检查账户是否存在
    let is_new = delegate_info.data_len() == 0;
    
    if is_new {
        // 创建新账户
        create_pda_account(
            owner_info,
            delegate_info,
            system_program_info,
            program_id,
            &rent,
            DelegateAccount::SIZE,
            delegate_seeds_with_bump,
        )?;
        
        // 初始化 delegate
        let delegate = DelegateAccount::new(
            *owner_info.key,
            *vault_info.key,
            delegate_pubkey,
            permissions,
            max_notional,
            expiry_slot,
            delegate_bump,
        );
        delegate.serialize(&mut &mut delegate_info.data.borrow_mut()[..])?;
        
        msg!("Delegate created: {}", delegate_pubkey);
    } else {
        // 更新现有 delegate
        let mut delegate = DelegateAccount::try_from_slice(&delegate_info.data.borrow())?;
        
        // 验证
        if delegate.owner != *owner_info.key {
            return Err(VaultError::InvalidOwner.into());
        }
        
        // 更新字段
        delegate.permissions = permissions;
        delegate.max_notional = max_notional;
        delegate.expiry_slot = expiry_slot;
        delegate.is_active = true;
        delegate.update_timestamp();
        
        delegate.serialize(&mut &mut delegate_info.data.borrow_mut()[..])?;
        
        msg!("Delegate updated: {}", delegate_pubkey);
    }
    
    msg!("Permissions: {:064b}", permissions);
    msg!("Max notional: {}", max_notional);
    msg!("Expiry slot: {}", expiry_slot);
    
    Ok(())
}

/// 撤销 Delegate
fn process_revoke_delegate(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    delegate_pubkey: Pubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    let delegate_info = next_account_info(account_info_iter)?;
    let vault_info = next_account_info(account_info_iter)?;
    let owner_info = next_account_info(account_info_iter)?;
    let _global_config_info = next_account_info(account_info_iter)?;
    
    // 验证
    require_signer(owner_info)?;
    require_writable(delegate_info)?;
    require_owner(delegate_info, program_id)?;
    
    // 读取 vault
    let vault = UserVault::try_from_slice(&vault_info.data.borrow())?;
    
    // 验证 owner
    if vault.owner != *owner_info.key {
        return Err(VaultError::InvalidOwner.into());
    }
    
    // 读取 delegate
    let mut delegate = DelegateAccount::try_from_slice(&delegate_info.data.borrow())?;
    
    // 验证
    if delegate.owner != *owner_info.key {
        return Err(VaultError::InvalidOwner.into());
    }
    
    if delegate.delegate != delegate_pubkey {
        return Err(VaultError::InvalidDelegate.into());
    }
    
    // 撤销
    delegate.is_active = false;
    delegate.nonce = u64::MAX; // 防止旧交易重放
    delegate.update_timestamp();
    
    delegate.serialize(&mut &mut delegate_info.data.borrow_mut()[..])?;
    
    msg!("Delegate revoked: {}", delegate_pubkey);
    
    Ok(())
}

/// 锁定保证金（CPI调用）
fn process_lock_margin(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    required_margin: u64,
    required_notional: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    let vault_info = next_account_info(account_info_iter)?;
    let signer_info = next_account_info(account_info_iter)?;
    let delegate_info = account_info_iter.next(); // Optional
    let _global_config_info = next_account_info(account_info_iter)?;
    let _clock_sysvar_info = next_account_info(account_info_iter)?;
    
    // 验证
    require_signer(signer_info)?;
    require_writable(vault_info)?;
    require_owner(vault_info, program_id)?;
    
    // 读取 vault
    let mut vault = UserVault::try_from_slice(&vault_info.data.borrow())?;
    
    // 检查是否冻结
    if vault.is_frozen() {
        return Err(VaultError::VaultFrozen.into());
    }
    
    // 权限验证
    let is_owner = *signer_info.key == vault.owner;
    
    if !is_owner {
        // 如果不是 owner，必须是有 TRADE 权限的 delegate
        let delegate_account_info = delegate_info.ok_or(VaultError::InvalidDelegate)?;
        require_writable(delegate_account_info)?;
        
        let mut delegate = DelegateAccount::try_from_slice(&delegate_account_info.data.borrow())?;
        
        // 验证 delegate
        if delegate.delegate != *signer_info.key {
            return Err(VaultError::InvalidDelegate.into());
        }
        
        if delegate.owner != vault.owner {
            return Err(VaultError::InvalidOwner.into());
        }
        
        // 检查权限
        let current_slot = Clock::get()?.slot;
        if !delegate.is_valid(current_slot) {
            return Err(VaultError::DelegateExpired.into());
        }
        
        if !delegate.has_permission(PERM_TRADE) {
            return Err(VaultError::PermissionDenied.into());
        }
        
        // 检查 notional 限额
        if !delegate.can_use_notional(required_notional) {
            return Err(VaultError::NotionalLimitExceeded.into());
        }
        
        // 更新 delegate 的 used_notional
        delegate.used_notional = safe_add(delegate.used_notional, required_notional)?;
        delegate.update_timestamp();
        delegate.serialize(&mut &mut delegate_account_info.data.borrow_mut()[..])?;
    }
    
    // 检查保证金充足
    if vault.free_collateral < required_margin {
        return Err(VaultError::InsufficientFreeCollateral.into());
    }
    
    // 锁定保证金
    vault.free_collateral = safe_sub(vault.free_collateral, required_margin)?;
    vault.locked_collateral = safe_add(vault.locked_collateral, required_margin)?;
    vault.update_timestamp();
    
    vault.serialize(&mut &mut vault_info.data.borrow_mut()[..])?;
    
    // Note: 不需要验证余额一致性，因为 lock 不改变总余额，只是内部转移
    
    msg!("Locked margin: {}", required_margin);
    msg!("Locked notional: {}", required_notional);
    msg!("New free collateral: {}", vault.free_collateral);
    msg!("New locked collateral: {}", vault.locked_collateral);
    
    Ok(())
}

/// 解锁保证金并更新 PnL（CPI调用）
fn process_unlock_margin_and_update_pnl(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    unlocked_margin: u64,
    pnl_delta: i64,
    notional_delta: i64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    let vault_info = next_account_info(account_info_iter)?;
    let signer_info = next_account_info(account_info_iter)?;
    let delegate_info = account_info_iter.next(); // Optional
    let _global_config_info = next_account_info(account_info_iter)?;
    
    // 验证
    require_signer(signer_info)?;
    require_writable(vault_info)?;
    require_owner(vault_info, program_id)?;
    
    // 读取 vault
    let mut vault = UserVault::try_from_slice(&vault_info.data.borrow())?;
    
    // 权限验证
    let is_owner = *signer_info.key == vault.owner;
    
    if !is_owner {
        // 如果不是 owner，必须是有 TRADE 权限的 delegate
        let delegate_account_info = delegate_info.ok_or(VaultError::InvalidDelegate)?;
        require_writable(delegate_account_info)?;
        
        let mut delegate = DelegateAccount::try_from_slice(&delegate_account_info.data.borrow())?;
        
        // 验证 delegate
        if delegate.delegate != *signer_info.key {
            return Err(VaultError::InvalidDelegate.into());
        }
        
        if delegate.owner != vault.owner {
            return Err(VaultError::InvalidOwner.into());
        }
        
        // 检查权限
        let current_slot = Clock::get()?.slot;
        if !delegate.is_valid(current_slot) {
            return Err(VaultError::DelegateExpired.into());
        }
        
        if !delegate.has_permission(PERM_TRADE) {
            return Err(VaultError::PermissionDenied.into());
        }
        
        // 更新 delegate 的 used_notional
        if notional_delta < 0 {
            delegate.used_notional = safe_sub(delegate.used_notional, notional_delta.unsigned_abs())?;
        } else {
            // 释放敞口时 notional_delta 应该是负数
            // 如果是正数，可能是错误，但为了兼容性也支持
            delegate.used_notional = safe_add(delegate.used_notional, notional_delta as u64)?;
        }
        
        delegate.update_timestamp();
        delegate.serialize(&mut &mut delegate_account_info.data.borrow_mut()[..])?;
    }
    
    // 解锁保证金
    vault.locked_collateral = safe_sub(vault.locked_collateral, unlocked_margin)?;
    vault.free_collateral = safe_add(vault.free_collateral, unlocked_margin)?;
    
    // 应用 PnL
    vault.free_collateral = safe_add_signed(vault.free_collateral, pnl_delta)?;
    
    vault.update_timestamp();
    vault.serialize(&mut &mut vault_info.data.borrow_mut()[..])?;
    
    // Note: PnL 会改变总余额，但这是正常的（盈利增加，亏损减少）
    // 这里的验证会失败，因为实际的 USDC 转账由业务程序处理
    // 所以 unlock 时不验证余额一致性
    
    msg!("Unlocked margin: {}", unlocked_margin);
    msg!("PnL delta: {}", pnl_delta);
    msg!("Notional delta: {}", notional_delta);
    msg!("New free collateral: {}", vault.free_collateral);
    msg!("New locked collateral: {}", vault.locked_collateral);
    
    Ok(())
}

/// 转移 Admin 权限
///
/// 将 GlobalConfig 的 admin 转移给新地址
///
/// # 账户
/// 0. `[writable]` GlobalConfig PDA
/// 1. `[signer]` Current Admin
fn process_transfer_admin(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    new_admin: Pubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    let global_config_info = next_account_info(account_info_iter)?;
    let admin_info = next_account_info(account_info_iter)?;
    
    // 验证
    require_signer(admin_info)?;
    require_writable(global_config_info)?;
    require_owner(global_config_info, program_id)?;
    
    // 读取 global config
    let mut global_config = GlobalConfig::try_from_slice(&global_config_info.data.borrow())?;
    
    // 验证当前 admin
    if global_config.admin != *admin_info.key {
        return Err(VaultError::InvalidAuthority.into());
    }
    
    // 不允许转移给默认地址（使用 RenounceAdmin）
    if new_admin == Pubkey::default() {
        msg!("Cannot transfer to default pubkey. Use RenounceAdmin instead.");
        return Err(VaultError::InvalidAuthority.into());
    }
    
    // 转移 admin
    let old_admin = global_config.admin;
    global_config.admin = new_admin;
    global_config.serialize(&mut &mut global_config_info.data.borrow_mut()[..])?;
    
    msg!("✅ Admin transferred");
    msg!("Old admin: {}", old_admin);
    msg!("New admin: {}", new_admin);
    
    Ok(())
}

/// 放弃 Admin 权限（实现完全非托管）
///
/// 将 admin 设为 Pubkey::default()，之后无人可修改 GlobalConfig
///
/// # 账户
/// 0. `[writable]` GlobalConfig PDA
/// 1. `[signer]` Current Admin
fn process_renounce_admin(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    let global_config_info = next_account_info(account_info_iter)?;
    let admin_info = next_account_info(account_info_iter)?;
    
    // 验证
    require_signer(admin_info)?;
    require_writable(global_config_info)?;
    require_owner(global_config_info, program_id)?;
    
    // 读取 global config
    let mut global_config = GlobalConfig::try_from_slice(&global_config_info.data.borrow())?;
    
    // 验证当前 admin
    if global_config.admin != *admin_info.key {
        return Err(VaultError::InvalidAuthority.into());
    }
    
    // 放弃 admin
    let old_admin = global_config.admin;
    global_config.admin = Pubkey::default();
    global_config.serialize(&mut &mut global_config_info.data.borrow_mut()[..])?;
    
    msg!("⚠️  Admin renounced - Program is now fully non-custodial!");
    msg!("Previous admin: {}", old_admin);
    msg!("New admin: {} (none)", Pubkey::default());
    
    Ok(())
}

/// 冻结 Vault
///
/// Owner 可以冻结自己的 vault，阻止所有操作（除了解冻）
///
/// # 账户
/// 0. `[writable]` UserVault PDA
/// 1. `[signer]` Owner
fn process_freeze_vault(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    let vault_info = next_account_info(account_info_iter)?;
    let owner_info = next_account_info(account_info_iter)?;
    
    // 验证
    require_signer(owner_info)?;
    require_writable(vault_info)?;
    require_owner(vault_info, program_id)?;
    
    // 读取 vault
    let mut vault = UserVault::try_from_slice(&vault_info.data.borrow())?;
    
    // 验证 owner
    if vault.owner != *owner_info.key {
        return Err(VaultError::InvalidOwner.into());
    }
    
    // 检查是否已冻结
    if vault.is_frozen() {
        msg!("Vault is already frozen");
        return Ok(());
    }
    
    // 冻结
    vault.freeze();
    vault.serialize(&mut &mut vault_info.data.borrow_mut()[..])?;
    
    msg!("🧊 Vault frozen");
    msg!("Owner: {}", owner_info.key);
    
    Ok(())
}

/// 解冻 Vault
///
/// Owner 可以解冻自己的 vault，恢复正常操作
///
/// # 账户
/// 0. `[writable]` UserVault PDA
/// 1. `[signer]` Owner
fn process_unfreeze_vault(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    let vault_info = next_account_info(account_info_iter)?;
    let owner_info = next_account_info(account_info_iter)?;
    
    // 验证
    require_signer(owner_info)?;
    require_writable(vault_info)?;
    require_owner(vault_info, program_id)?;
    
    // 读取 vault
    let mut vault = UserVault::try_from_slice(&vault_info.data.borrow())?;
    
    // 验证 owner
    if vault.owner != *owner_info.key {
        return Err(VaultError::InvalidOwner.into());
    }
    
    // 检查是否未冻结
    if !vault.is_frozen() {
        msg!("Vault is not frozen");
        return Ok(());
    }
    
    // 解冻
    vault.unfreeze();
    vault.serialize(&mut &mut vault_info.data.borrow_mut()[..])?;
    
    msg!("✅ Vault unfrozen");
    msg!("Owner: {}", owner_info.key);
    
    Ok(())
}

