/**
 * 1024 API Key Vault Program - 初始化 GlobalConfig
 * 
 * 使用方式:
 * npx ts-node scripts/initialize-cli.ts
 */

import {
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
  sendAndConfirmTransaction,
  SYSVAR_RENT_PUBKEY,
} from '@solana/web3.js';
import * as fs from 'fs';
import * as borsh from 'borsh';
import { fileURLToPath } from 'url';
import { dirname } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// 读取配置
const config = JSON.parse(fs.readFileSync(__dirname + '/config.json', 'utf-8'));
const PROGRAM_ID = new PublicKey(config.program_id);
const USDC_MINT = new PublicKey(config.usdc_mint);
const RPC_URL = config.rpc_url;

// 读取 payer keypair（使用 settlement-authority-fixed.json）
const payerKeypairPath = '/Users/chuciqin/Desktop/project1024/1024codebase/1024-core/settlement-authority-fixed.json';
const payerKeypair = Keypair.fromSecretKey(
  Uint8Array.from(JSON.parse(fs.readFileSync(payerKeypairPath, 'utf-8')))
);

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

// InitializeGlobalConfig 指令数据结构
class InitializeGlobalConfigData {
  instruction: number;
  usdc_mint: Uint8Array;

  constructor(props: { usdc_mint: PublicKey }) {
    this.instruction = VaultInstruction.InitializeGlobalConfig;
    this.usdc_mint = props.usdc_mint.toBytes();
  }
}

// Borsh schema
const initializeGlobalConfigSchema = new Map([
  [
    InitializeGlobalConfigData,
    {
      kind: 'struct',
      fields: [
        ['instruction', 'u8'],
        ['usdc_mint', [32]],
      ],
    },
  ],
]);

async function initializeGlobalConfig() {
  console.log('======================================');
  console.log('1024 API Key Vault Program 初始化');
  console.log('======================================\n');

  // 创建连接（禁用 WebSocket）
  const connection = new Connection(RPC_URL, {
    commitment: 'confirmed',
    disableRetryOnRateLimit: false,
    httpHeaders: {},
  });

  console.log('📋 配置信息:');
  console.log(`   Program ID: ${PROGRAM_ID.toBase58()}`);
  console.log(`   USDC Mint: ${USDC_MINT.toBase58()}`);
  console.log(`   Admin: ${payerKeypair.publicKey.toBase58()}`);
  console.log(`   RPC URL: ${RPC_URL}\n`);

  // 派生 GlobalConfig PDA
  const version = 1;
  
  console.log('🔍 调试信息:');
  console.log(`   Program ID 来自 config: ${config.program_id}`);
  console.log(`   PROGRAM_ID 对象: ${PROGRAM_ID.toBase58()}`);
  console.log(`   Version: ${version}`);
  console.log('');
  
  const [globalConfigPDA, bump] = PublicKey.findProgramAddressSync(
    [Buffer.from('global'), Buffer.from([version])],
    PROGRAM_ID
  );

  console.log(`🔑 GlobalConfig PDA: ${globalConfigPDA.toBase58()}`);
  console.log(`   Bump: ${bump}\n`);

  // 检查是否已初始化
  const accountInfo = await connection.getAccountInfo(globalConfigPDA);
  if (accountInfo && accountInfo.data.length > 0) {
    console.log('⚠️  GlobalConfig 已经初始化过了');
    console.log(`   Account size: ${accountInfo.data.length} bytes`);
    return;
  }

  // 构造指令数据
  const instructionData = new InitializeGlobalConfigData({
    usdc_mint: USDC_MINT,
  });

  const serialized = borsh.serialize(
    initializeGlobalConfigSchema,
    instructionData
  );

  // 构造交易指令
  const instruction = new TransactionInstruction({
    keys: [
      { pubkey: globalConfigPDA, isSigner: false, isWritable: true },
      { pubkey: payerKeypair.publicKey, isSigner: true, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
    ],
    programId: PROGRAM_ID,
    data: Buffer.from(serialized),
  });

  // 创建并发送交易
  const transaction = new Transaction().add(instruction);

  console.log('🚀 发送初始化交易...\n');

  try {
    // 发送交易
    const signature = await connection.sendTransaction(transaction, [payerKeypair], {
      skipPreflight: false,
      preflightCommitment: 'confirmed',
    });
    
    console.log(`📝 交易已发送: ${signature}`);
    console.log('⏳ 等待确认...\n');
    
    // 等待确认（使用轮询而非 WebSocket）
    const latestBlockhash = await connection.getLatestBlockhash('confirmed');
    await connection.confirmTransaction(
      {
        signature,
        blockhash: latestBlockhash.blockhash,
        lastValidBlockHeight: latestBlockhash.lastValidBlockHeight,
      },
      'confirmed'
    );

    console.log('======================================');
    console.log('✅ 初始化成功!');
    console.log('======================================\n');
    console.log(`📋 交易签名: ${signature}`);
    console.log(`🔗 查看交易: https://testnet-scan.1024chain.com/tx/${signature}\n`);
    console.log(`📋 GlobalConfig PDA: ${globalConfigPDA.toBase58()}`);
    console.log(`   USDC Mint: ${USDC_MINT.toBase58()}`);
    console.log(`   Admin: ${payerKeypair.publicKey.toBase58()}\n`);
  } catch (error) {
    console.error('❌ 初始化失败:', error);
    throw error;
  }
}

// 运行
initializeGlobalConfig().catch(console.error);

