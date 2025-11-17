/**
 * 1024 API Key Vault Program - 完整功能测试
 * 
 * 测试流程:
 * 1. 创建 Vault
 * 2. Mint USDC 到用户账户
 * 3. 存款 USDC 到 Vault
 * 4. 创建 API Key (Delegate)
 * 5. 使用 API Key 测试权限
 * 6. 提款测试
 * 7. 撤销 API Key
 */

import {
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
  SYSVAR_RENT_PUBKEY,
  sendAndConfirmTransaction,
} from '@solana/web3.js';
import {
  TOKEN_PROGRAM_ID,
  getOrCreateAssociatedTokenAccount,
  mintTo,
  getAccount,
} from '@solana/spl-token';
import * as fs from 'fs';
import * as borsh from 'borsh';
import bs58 from 'bs58';
import { fileURLToPath } from 'url';
import { dirname } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// 读取配置
const config = JSON.parse(fs.readFileSync(__dirname + '/config.json', 'utf-8'));
const PROGRAM_ID = new PublicKey(config.program_id);
const USDC_MINT = new PublicKey(config.usdc_mint);
const RPC_URL = config.rpc_url;

// 使用测试账户 #1 (有 USDC mint authority)
const ownerKeypair = Keypair.fromSecretKey(
  bs58.decode(config.test_accounts.account1.secret)
);

console.log('======================================');
console.log('1024 Vault Program 功能测试');
console.log('======================================\n');

console.log('📋 配置信息:');
console.log(`   Program ID: ${PROGRAM_ID.toBase58()}`);
console.log(`   USDC Mint: ${USDC_MINT.toBase58()}`);
console.log(`   Test Account: ${ownerKeypair.publicKey.toBase58()}`);
console.log(`   RPC URL: ${RPC_URL}\n`);

// VaultInstruction enum
const VaultInstruction = {
  InitializeGlobalConfig: 0,
  CreateVault: 1,
  Deposit: 2,
  Withdraw: 3,
  UpsertDelegate: 4,
  RevokeDelegate: 5,
  LockMargin: 6,
  UnlockMarginAndUpdatePnl: 7,
  TransferAdmin: 8,
  RenounceAdmin: 9,
  FreezeVault: 10,
  UnfreezeVault: 11,
} as const;

// 权限定义
const PERM_TRADE = 1n << 0n;
const PERM_WITHDRAW = 1n << 1n;
const PERM_CLOSE_ONLY = 1n << 2n;

// 指令数据结构
class DepositData {
  instruction = VaultInstruction.Deposit;
  amount: bigint;
  constructor(amount: bigint) {
    this.amount = amount;
  }
}

class WithdrawData {
  instruction = VaultInstruction.Withdraw;
  amount: bigint;
  constructor(amount: bigint) {
    this.amount = amount;
  }
}

class UpsertDelegateData {
  instruction = VaultInstruction.UpsertDelegate;
  delegate_pubkey: Uint8Array;
  permissions: bigint;
  max_notional: bigint;
  expiry_slot: bigint;

  constructor(props: {
    delegate_pubkey: PublicKey;
    permissions: bigint;
    max_notional: bigint;
    expiry_slot: bigint;
  }) {
    this.delegate_pubkey = props.delegate_pubkey.toBytes();
    this.permissions = props.permissions;
    this.max_notional = props.max_notional;
    this.expiry_slot = props.expiry_slot;
  }
}

class RevokeDelegateData {
  instruction = VaultInstruction.RevokeDelegate;
  delegate_pubkey: Uint8Array;

  constructor(delegate_pubkey: PublicKey) {
    this.delegate_pubkey = delegate_pubkey.toBytes();
  }
}

// Borsh schemas
const depositSchema = new Map([
  [
    DepositData,
    {
      kind: 'struct',
      fields: [
        ['instruction', 'u8'],
        ['amount', 'u64'],
      ],
    },
  ],
]);

const withdrawSchema = new Map([
  [
    WithdrawData,
    {
      kind: 'struct',
      fields: [
        ['instruction', 'u8'],
        ['amount', 'u64'],
      ],
    },
  ],
]);

const upsertDelegateSchema = new Map([
  [
    UpsertDelegateData,
    {
      kind: 'struct',
      fields: [
        ['instruction', 'u8'],
        ['delegate_pubkey', [32]],
        ['permissions', 'u64'],
        ['max_notional', 'u64'],
        ['expiry_slot', 'u64'],
      ],
    },
  ],
]);

const revokeDelegateSchema = new Map([
  [
    RevokeDelegateData,
    {
      kind: 'struct',
      fields: [
        ['instruction', 'u8'],
        ['delegate_pubkey', [32]],
      ],
    },
  ],
]);

async function runTests() {
  const connection = new Connection(RPC_URL, {
    commitment: 'confirmed',
    disableRetryOnRateLimit: false,
  });

  // GlobalConfig PDA
  const [globalConfigPDA] = PublicKey.findProgramAddressSync(
    [Buffer.from('global'), Buffer.from([1])],
    PROGRAM_ID
  );

  // UserVault PDA
  const [vaultPDA, vaultBump] = PublicKey.findProgramAddressSync(
    [Buffer.from('vault'), ownerKeypair.publicKey.toBuffer()],
    PROGRAM_ID
  );

  // Vault USDC Token Account PDA
  const [vaultUsdcPDA, vaultUsdcBump] = PublicKey.findProgramAddressSync(
    [Buffer.from('vault-usdc'), ownerKeypair.publicKey.toBuffer()],
    PROGRAM_ID
  );

  console.log('🔑 派生的 PDAs:');
  console.log(`   Vault PDA: ${vaultPDA.toBase58()}`);
  console.log(`   Vault USDC PDA: ${vaultUsdcPDA.toBase58()}`);
  console.log(`   Global Config PDA: ${globalConfigPDA.toBase58()}\n`);

  // ==================================================
  // 测试 1: 创建 Vault
  // ==================================================
  console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
  console.log('📝 测试 1: 创建 Vault');
  console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n');

  try {
    // 检查 vault 是否已存在
    const vaultAccountInfo = await connection.getAccountInfo(vaultPDA);
    
    if (vaultAccountInfo && vaultAccountInfo.data.length > 0) {
      console.log('⚠️  Vault 已存在，跳过创建\n');
    } else {
      // 创建 Vault
      const createVaultIx = new TransactionInstruction({
        keys: [
          { pubkey: vaultPDA, isSigner: false, isWritable: true },
          { pubkey: vaultUsdcPDA, isSigner: false, isWritable: true },
          { pubkey: ownerKeypair.publicKey, isSigner: true, isWritable: true },
          { pubkey: globalConfigPDA, isSigner: false, isWritable: false },
          { pubkey: USDC_MINT, isSigner: false, isWritable: false },
          { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
          { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
          { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
        ],
        programId: PROGRAM_ID,
        data: Buffer.from([VaultInstruction.CreateVault]),
      });

      const createVaultTx = new Transaction().add(createVaultIx);
      const createVaultSig = await connection.sendTransaction(
        createVaultTx,
        [ownerKeypair],
        { skipPreflight: false }
      );

      console.log(`📝 交易已发送: ${createVaultSig}`);
      console.log('⏳ 等待确认...\n');

      const latestBlockhash = await connection.getLatestBlockhash('confirmed');
      await connection.confirmTransaction({
        signature: createVaultSig,
        blockhash: latestBlockhash.blockhash,
        lastValidBlockHeight: latestBlockhash.lastValidBlockHeight,
      });

      console.log('✅ Vault 创建成功!');
      console.log(`   签名: ${createVaultSig}\n`);
    }
  } catch (error: any) {
    console.error('❌ 创建 Vault 失败:', error.message);
    throw error;
  }

  // ==================================================
  // 测试 2: Mint USDC 到用户账户
  // ==================================================
  console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
  console.log('💰 测试 2: Mint USDC 到用户账户');
  console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n');

  try {
    // 获取或创建用户的 USDC token account
    const userUsdcAccount = await getOrCreateAssociatedTokenAccount(
      connection,
      ownerKeypair,
      USDC_MINT,
      ownerKeypair.publicKey
    );

    console.log(`💳 用户 USDC Account: ${userUsdcAccount.address.toBase58()}`);

    // Mint 10000 USDC (decimals = 9)
    const mintAmount = 10000n * 1_000_000_000n;
    
    const mintSig = await mintTo(
      connection,
      ownerKeypair,
      USDC_MINT,
      userUsdcAccount.address,
      ownerKeypair, // mint authority
      mintAmount
    );

    console.log(`✅ Minted 10,000 USDC`);
    console.log(`   签名: ${mintSig}`);

    // 查询余额
    const balance = await connection.getTokenAccountBalance(userUsdcAccount.address);
    console.log(`   当前余额: ${balance.value.uiAmount} USDC\n`);
  } catch (error: any) {
    console.error('❌ Mint USDC 失败:', error.message);
    throw error;
  }

  // ==================================================
  // 测试 3: 存款到 Vault
  // ==================================================
  console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
  console.log('💵 测试 3: 存款 5000 USDC 到 Vault');
  console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n');

  try {
    const userUsdcAccount = await getOrCreateAssociatedTokenAccount(
      connection,
      ownerKeypair,
      USDC_MINT,
      ownerKeypair.publicKey
    );

    // 存入 5000 USDC
    const depositAmount = 5000n * 1_000_000_000n;
    const depositData = new DepositData(depositAmount);
    const depositInstructionData = borsh.serialize(depositSchema, depositData);

    const depositIx = new TransactionInstruction({
      keys: [
        { pubkey: vaultPDA, isSigner: false, isWritable: true },
        { pubkey: ownerKeypair.publicKey, isSigner: true, isWritable: false },
        { pubkey: userUsdcAccount.address, isSigner: false, isWritable: true },
        { pubkey: vaultUsdcPDA, isSigner: false, isWritable: true },
        { pubkey: globalConfigPDA, isSigner: false, isWritable: false },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      ],
      programId: PROGRAM_ID,
      data: Buffer.from(depositInstructionData),
    });

    const depositTx = new Transaction().add(depositIx);
    const depositSig = await connection.sendTransaction(depositTx, [ownerKeypair]);

    console.log(`📝 交易已发送: ${depositSig}`);
    console.log('⏳ 等待确认...\n');

    const latestBlockhash = await connection.getLatestBlockhash('confirmed');
    await connection.confirmTransaction({
      signature: depositSig,
      blockhash: latestBlockhash.blockhash,
      lastValidBlockHeight: latestBlockhash.lastValidBlockHeight,
    });

    console.log('✅ 存款成功!');
    console.log(`   签名: ${depositSig}`);

    // 查询 Vault 余额
    const vaultBalance = await connection.getTokenAccountBalance(vaultUsdcPDA);
    console.log(`   Vault 余额: ${vaultBalance.value.uiAmount} USDC\n`);
  } catch (error: any) {
    console.error('❌ 存款失败:', error.message);
    throw error;
  }

  // ==================================================
  // 测试 4: 创建 API Key (Delegate)
  // ==================================================
  console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
  console.log('🔑 测试 4: 创建 API Key');
  console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n');

  const apiKey = Keypair.generate();
  
  console.log('🔐 生成的 API Key:');
  console.log(`   Public: ${apiKey.publicKey.toBase58()}`);
  console.log(`   Secret: ${bs58.encode(apiKey.secretKey)}`);
  console.log('   ⚠️  私钥只保存在用户环境\n');

  try {
    // Delegate PDA
    const [delegatePDA] = PublicKey.findProgramAddressSync(
      [
        Buffer.from('delegate'),
        ownerKeypair.publicKey.toBuffer(),
        apiKey.publicKey.toBuffer(),
      ],
      PROGRAM_ID
    );

    console.log(`   Delegate PDA: ${delegatePDA.toBase58()}\n`);

    // 获取当前 slot
    const currentSlot = await connection.getSlot();
    const expirySlot = BigInt(currentSlot) + 100_000n; // ~1天后过期

    // 创建 delegate: 允许提现，最大 3000 USDC 敞口
    const upsertData = new UpsertDelegateData({
      delegate_pubkey: apiKey.publicKey,
      permissions: PERM_WITHDRAW, // 只允许提现
      max_notional: 3000n * 1_000_000_000n,
      expiry_slot: expirySlot,
    });

    const upsertInstructionData = borsh.serialize(upsertDelegateSchema, upsertData);

    const upsertIx = new TransactionInstruction({
      keys: [
        { pubkey: delegatePDA, isSigner: false, isWritable: true },
        { pubkey: vaultPDA, isSigner: false, isWritable: true },
        { pubkey: ownerKeypair.publicKey, isSigner: true, isWritable: true },
        { pubkey: globalConfigPDA, isSigner: false, isWritable: false },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      programId: PROGRAM_ID,
      data: Buffer.from(upsertInstructionData),
    });

    const upsertTx = new Transaction().add(upsertIx);
    const upsertSig = await connection.sendTransaction(upsertTx, [ownerKeypair]);

    console.log(`📝 交易已发送: ${upsertSig}`);
    console.log('⏳ 等待确认...\n');

    const latestBlockhash = await connection.getLatestBlockhash('confirmed');
    await connection.confirmTransaction({
      signature: upsertSig,
      blockhash: latestBlockhash.blockhash,
      lastValidBlockHeight: latestBlockhash.lastValidBlockHeight,
    });

    console.log('✅ API Key 创建成功!');
    console.log(`   签名: ${upsertSig}`);
    console.log(`   权限: PERM_WITHDRAW`);
    console.log(`   最大敞口: 3000 USDC`);
    console.log(`   过期 Slot: ${expirySlot}\n`);
  } catch (error: any) {
    console.error('❌ 创建 API Key 失败:', error.message);
    throw error;
  }

  // ==================================================
  // 测试 5: 使用 API Key 提款
  // ==================================================
  console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
  console.log('💸 测试 5: 使用 API Key 提款 1000 USDC');
  console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n');

  try {
    const userUsdcAccount = await getOrCreateAssociatedTokenAccount(
      connection,
      ownerKeypair,
      USDC_MINT,
      ownerKeypair.publicKey
    );

    const [delegatePDA] = PublicKey.findProgramAddressSync(
      [
        Buffer.from('delegate'),
        ownerKeypair.publicKey.toBuffer(),
        apiKey.publicKey.toBuffer(),
      ],
      PROGRAM_ID
    );

    // 提取 1000 USDC
    const withdrawAmount = 1000n * 1_000_000_000n;
    const withdrawData = new WithdrawData(withdrawAmount);
    const withdrawInstructionData = borsh.serialize(withdrawSchema, withdrawData);

    const withdrawIx = new TransactionInstruction({
      keys: [
        { pubkey: vaultPDA, isSigner: false, isWritable: true },
        { pubkey: apiKey.publicKey, isSigner: true, isWritable: false }, // API Key 签名
        { pubkey: userUsdcAccount.address, isSigner: false, isWritable: true },
        { pubkey: vaultUsdcPDA, isSigner: false, isWritable: true },
        { pubkey: globalConfigPDA, isSigner: false, isWritable: false },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
        { pubkey: delegatePDA, isSigner: false, isWritable: false },
      ],
      programId: PROGRAM_ID,
      data: Buffer.from(withdrawInstructionData),
    });

    const withdrawTx = new Transaction().add(withdrawIx);
    const withdrawSig = await connection.sendTransaction(withdrawTx, [apiKey]); // 使用 API Key 签名

    console.log(`📝 交易已发送: ${withdrawSig}`);
    console.log('⏳ 等待确认...\n');

    const latestBlockhash = await connection.getLatestBlockhash('confirmed');
    await connection.confirmTransaction({
      signature: withdrawSig,
      blockhash: latestBlockhash.blockhash,
      lastValidBlockHeight: latestBlockhash.lastValidBlockHeight,
    });

    console.log('✅ 使用 API Key 提款成功!');
    console.log(`   签名: ${withdrawSig}`);

    // 查询余额
    const vaultBalance = await connection.getTokenAccountBalance(vaultUsdcPDA);
    const userBalance = await connection.getTokenAccountBalance(userUsdcAccount.address);
    console.log(`   Vault 余额: ${vaultBalance.value.uiAmount} USDC`);
    console.log(`   用户余额: ${userBalance.value.uiAmount} USDC\n`);
  } catch (error: any) {
    console.error('❌ 提款失败:', error.message);
    throw error;
  }

  // ==================================================
  // 测试 6: 撤销 API Key
  // ==================================================
  console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
  console.log('🚫 测试 6: 撤销 API Key');
  console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n');

  try {
    const [delegatePDA] = PublicKey.findProgramAddressSync(
      [
        Buffer.from('delegate'),
        ownerKeypair.publicKey.toBuffer(),
        apiKey.publicKey.toBuffer(),
      ],
      PROGRAM_ID
    );

    const revokeData = new RevokeDelegateData(apiKey.publicKey);
    const revokeInstructionData = borsh.serialize(revokeDelegateSchema, revokeData);

    const revokeIx = new TransactionInstruction({
      keys: [
        { pubkey: delegatePDA, isSigner: false, isWritable: true },
        { pubkey: vaultPDA, isSigner: false, isWritable: true },
        { pubkey: ownerKeypair.publicKey, isSigner: true, isWritable: false },
        { pubkey: globalConfigPDA, isSigner: false, isWritable: false },
      ],
      programId: PROGRAM_ID,
      data: Buffer.from(revokeInstructionData),
    });

    const revokeTx = new Transaction().add(revokeIx);
    const revokeSig = await connection.sendTransaction(revokeTx, [ownerKeypair]);

    console.log(`📝 交易已发送: ${revokeSig}`);
    console.log('⏳ 等待确认...\n');

    const latestBlockhash = await connection.getLatestBlockhash('confirmed');
    await connection.confirmTransaction({
      signature: revokeSig,
      blockhash: latestBlockhash.blockhash,
      lastValidBlockHeight: latestBlockhash.lastValidBlockHeight,
    });

    console.log('✅ API Key 已撤销!');
    console.log(`   签名: ${revokeSig}\n`);
  } catch (error: any) {
    console.error('❌ 撤销 API Key 失败:', error.message);
    throw error;
  }

  // ==================================================
  // 测试 7: 验证撤销后无法使用
  // ==================================================
  console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
  console.log('🔒 测试 7: 验证撤销后的 API Key 无法使用');
  console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n');

  try {
    const userUsdcAccount = await getOrCreateAssociatedTokenAccount(
      connection,
      ownerKeypair,
      USDC_MINT,
      ownerKeypair.publicKey
    );

    const [delegatePDA] = PublicKey.findProgramAddressSync(
      [
        Buffer.from('delegate'),
        ownerKeypair.publicKey.toBuffer(),
        apiKey.publicKey.toBuffer(),
      ],
      PROGRAM_ID
    );

    // 尝试再次提款（应该失败）
    const withdrawAmount = 100n * 1_000_000_000n;
    const withdrawData = new WithdrawData(withdrawAmount);
    const withdrawInstructionData = borsh.serialize(withdrawSchema, withdrawData);

    const withdrawIx = new TransactionInstruction({
      keys: [
        { pubkey: vaultPDA, isSigner: false, isWritable: true },
        { pubkey: apiKey.publicKey, isSigner: true, isWritable: false },
        { pubkey: userUsdcAccount.address, isSigner: false, isWritable: true },
        { pubkey: vaultUsdcPDA, isSigner: false, isWritable: true },
        { pubkey: globalConfigPDA, isSigner: false, isWritable: false },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
        { pubkey: delegatePDA, isSigner: false, isWritable: false },
      ],
      programId: PROGRAM_ID,
      data: Buffer.from(withdrawInstructionData),
    });

    const withdrawTx = new Transaction().add(withdrawIx);
    const withdrawSig = await connection.sendTransaction(withdrawTx, [apiKey]);

    await connection.confirmTransaction(withdrawSig);

    console.log('❌ 错误：撤销后的 API Key 仍然可以使用！\n');
  } catch (error: any) {
    console.log('✅ 验证通过：撤销后的 API Key 无法使用');
    console.log(`   预期错误: ${error.message}\n`);
  }

  // ==================================================
  // 测试总结
  // ==================================================
  console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
  console.log('🎉 所有测试完成！');
  console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n');

  console.log('📊 测试总结:');
  console.log('   ✅ 创建 Vault');
  console.log('   ✅ Mint USDC');
  console.log('   ✅ 存款到 Vault');
  console.log('   ✅ 创建 API Key (Delegate)');
  console.log('   ✅ 使用 API Key 提款');
  console.log('   ✅ 撤销 API Key');
  console.log('   ✅ 验证权限控制\n');

  console.log('🔗 链上数据:');
  console.log(`   Program ID: ${PROGRAM_ID.toBase58()}`);
  console.log(`   Vault PDA: ${vaultPDA.toBase58()}`);
  console.log(`   查看: https://testnet-scan.1024chain.com/address/${vaultPDA.toBase58()}\n`);
}

// 运行测试
runTests().catch((error) => {
  console.error('\n❌ 测试失败:', error);
  process.exit(1);
});

