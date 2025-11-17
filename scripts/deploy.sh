#!/bin/bash
set -e

# 1024 API Key Vault Program - 部署脚本
# 部署到 1024Chain Testnet

echo "======================================"
echo "1024 API Key Vault Program 部署脚本"
echo "======================================"
echo ""

# 配置
PROGRAM_NAME="vault_program"
PROGRAM_SO="target/deploy/${PROGRAM_NAME}.so"
KEYPAIR_PATH="scripts/deploy-keypair.json"
RPC_URL="https://testnet-rpc.1024chain.com/rpc/"

# 检查 program 文件
if [ ! -f "$PROGRAM_SO" ]; then
    echo "❌ 错误: 找不到 program 文件: $PROGRAM_SO"
    echo "请先运行: cargo build-sbf"
    exit 1
fi

echo "✅ 找到 program 文件: $PROGRAM_SO"
PROGRAM_SIZE=$(ls -lh "$PROGRAM_SO" | awk '{print $5}')
echo "   大小: $PROGRAM_SIZE"
echo ""

# 生成部署密钥对（如果不存在）
if [ ! -f "$KEYPAIR_PATH" ]; then
    echo "📝 生成部署密钥对..."
    solana-keygen new --no-bip39-passphrase --outfile "$KEYPAIR_PATH"
    echo ""
fi

# 获取 program ID
PROGRAM_ID=$(solana-keygen pubkey "$KEYPAIR_PATH")
echo "📋 Program ID: $PROGRAM_ID"
echo ""

# 配置 Solana CLI
echo "🔧 配置 Solana CLI..."
solana config set --url "$RPC_URL"
echo ""

# 检查余额
echo "💰 检查部署账户余额..."
PAYER_ADDRESS=$(solana address)
BALANCE=$(solana balance "$PAYER_ADDRESS" 2>/dev/null || echo "0 N1024")
echo "   部署账户: $PAYER_ADDRESS"
echo "   余额: $BALANCE"
echo ""

# 估算部署成本
echo "📊 估算部署成本..."
PROGRAM_SIZE_BYTES=$(stat -f%z "$PROGRAM_SO" 2>/dev/null || stat -c%s "$PROGRAM_SO")
ESTIMATED_COST=$(echo "scale=2; $PROGRAM_SIZE_BYTES / 1000000 * 2" | bc)
echo "   Program 大小: $PROGRAM_SIZE_BYTES bytes"
echo "   估算成本: ~$ESTIMATED_COST N1024"
echo ""

# 确认部署
read -p "🚀 确认部署到 1024Chain Testnet? (y/N): " -n 1 -r
echo ""
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "❌ 部署已取消"
    exit 0
fi

echo ""
echo "🚀 开始部署..."
echo ""

# 部署 program
solana program deploy \
    --program-id "$KEYPAIR_PATH" \
    --url "$RPC_URL" \
    "$PROGRAM_SO"

echo ""
echo "======================================"
echo "✅ 部署成功!"
echo "======================================"
echo ""
echo "📋 Program 信息:"
echo "   Program ID: $PROGRAM_ID"
echo "   RPC URL: $RPC_URL"
echo "   Keypair: $KEYPAIR_PATH"
echo ""
echo "🔍 验证部署:"
echo "   solana program show $PROGRAM_ID --url $RPC_URL"
echo ""
echo "📝 下一步:"
echo "   1. 运行 scripts/initialize.sh 初始化 GlobalConfig"
echo "   2. 运行 scripts/verify.sh 验证功能"
echo ""

