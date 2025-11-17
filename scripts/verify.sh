#!/bin/bash
set -e

# 1024 API Key Vault Program - 验证脚本

echo "======================================"
echo "1024 API Key Vault Program 验证"
echo "======================================"
echo ""

# 配置
RPC_URL="https://testnet-rpc.1024chain.com/rpc/"
CONFIG_FILE="scripts/config.json"

# 检查配置文件
if [ ! -f "$CONFIG_FILE" ]; then
    echo "❌ 错误: 找不到配置文件: $CONFIG_FILE"
    echo "请先运行: scripts/initialize.sh"
    exit 1
fi

# 读取配置
PROGRAM_ID=$(cat "$CONFIG_FILE" | grep -o '"program_id": "[^"]*"' | cut -d'"' -f4)
USDC_MINT=$(cat "$CONFIG_FILE" | grep -o '"usdc_mint": "[^"]*"' | cut -d'"' -f4)

echo "📋 配置信息:"
echo "   Program ID: $PROGRAM_ID"
echo "   USDC Mint: $USDC_MINT"
echo "   RPC URL: $RPC_URL"
echo ""

# 配置 CLI
solana config set --url "$RPC_URL"

echo "======================================"
echo "1. 验证 Program 部署"
echo "======================================"
echo ""

# 检查 program 是否存在
if solana program show "$PROGRAM_ID" &> /dev/null; then
    echo "✅ Program 已部署"
    solana program show "$PROGRAM_ID"
    echo ""
else
    echo "❌ Program 未找到"
    exit 1
fi

echo "======================================"
echo "2. 验证 USDC Mint"
echo "======================================"
echo ""

# 检查 USDC mint
if spl-token display "$USDC_MINT" &> /dev/null; then
    echo "✅ USDC Mint 存在"
    spl-token display "$USDC_MINT"
    echo ""
else
    echo "❌ USDC Mint 未找到"
    exit 1
fi

echo "======================================"
echo "3. 网络状态"
echo "======================================"
echo ""

# 检查网络状态
echo "📊 Epoch 信息:"
solana epoch-info
echo ""

echo "📊 当前 Slot:"
solana slot
echo ""

echo "======================================"
echo "✅ 验证完成"
echo "======================================"
echo ""
echo "📋 下一步:"
echo "   1. 使用 SDK 初始化 GlobalConfig"
echo "   2. 创建测试 Vault"
echo "   3. 测试存款/提款功能"
echo ""

