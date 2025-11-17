#!/bin/bash
set -e

# 1024 API Key Vault Program - 初始化脚本
# 初始化 GlobalConfig

echo "======================================"
echo "1024 API Key Vault Program 初始化"
echo "======================================"
echo ""

# 配置
RPC_URL="https://testnet-rpc.1024chain.com/rpc/"
PROGRAM_ID_FILE="scripts/deploy-keypair.json"

# 检查 program ID
if [ ! -f "$PROGRAM_ID_FILE" ]; then
    echo "❌ 错误: 找不到 program keypair: $PROGRAM_ID_FILE"
    echo "请先运行: scripts/deploy.sh"
    exit 1
fi

PROGRAM_ID=$(solana-keygen pubkey "$PROGRAM_ID_FILE")
echo "📋 Program ID: $PROGRAM_ID"
echo ""

# 配置 Solana CLI
echo "🔧 配置 Solana CLI..."
solana config set --url "$RPC_URL"
echo ""

# USDC Mint (这里使用一个测试 mint，你需要替换为实际的 USDC mint)
# 在 1024Chain testnet 上创建一个测试 USDC token
echo "💰 创建测试 USDC mint..."
USDC_MINT=$(spl-token create-token --decimals 6 2>&1 | grep "Creating token" | awk '{print $3}')

if [ -z "$USDC_MINT" ]; then
    echo "❌ 创建 USDC mint 失败"
    exit 1
fi

echo "✅ USDC Mint 创建成功: $USDC_MINT"
echo ""

# 这里需要构造并发送初始化交易
# 由于我们没有 TypeScript SDK，我们先记录信息
echo "======================================"
echo "✅ 准备完成"
echo "======================================"
echo ""
echo "📋 配置信息:"
echo "   Program ID: $PROGRAM_ID"
echo "   USDC Mint: $USDC_MINT"
echo "   RPC URL: $RPC_URL"
echo ""
echo "⚠️  注意:"
echo "   需要使用 SDK 或客户端调用 InitializeGlobalConfig 指令"
echo "   参数: { usdc_mint: '$USDC_MINT' }"
echo ""
echo "💾 保存配置到 scripts/config.json"
cat > scripts/config.json <<EOF
{
  "program_id": "$PROGRAM_ID",
  "usdc_mint": "$USDC_MINT",
  "rpc_url": "$RPC_URL",
  "network": "1024chain-testnet"
}
EOF

echo "✅ 配置已保存"
echo ""

