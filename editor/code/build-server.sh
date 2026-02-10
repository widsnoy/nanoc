#!/bin/bash

# Airyc Language Server 构建脚本
# 用于构建语言服务器并复制到插件目录
# 支持交叉编译到 Windows 和 Linux 平台

set -e

# 获取脚本所在目录（editor/code）
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# 项目根目录
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# 颜色输出
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Airyc Language Server Builder ===${NC}"

# 检查参数
if [ $# -lt 2 ]; then
    echo "Usage: $0 <target-platform> <build-mode>"
    echo ""
    echo "Target platforms:"
    echo "  linux   - Build for Linux (x86_64-unknown-linux-gnu)"
    echo "  windows - Build for Windows (x86_64-pc-windows-gnu)"
    echo ""
    echo "Build modes:"
    echo "  debug   - Build debug version"
    echo "  release - Build release version"
    echo ""
    echo "Examples:"
    echo "  $0 linux debug"
    echo "  $0 windows release"
    exit 1
fi

TARGET_PLATFORM=$1
BUILD_MODE=$2

# 验证目标平台
if [ "$TARGET_PLATFORM" != "linux" ] && [ "$TARGET_PLATFORM" != "windows" ]; then
    echo -e "${RED}Error: Invalid target platform '$TARGET_PLATFORM'${NC}"
    echo "Must be 'linux' or 'windows'"
    exit 1
fi

# 验证构建模式
if [ "$BUILD_MODE" != "debug" ] && [ "$BUILD_MODE" != "release" ]; then
    echo -e "${RED}Error: Invalid build mode '$BUILD_MODE'${NC}"
    echo "Must be 'debug' or 'release'"
    exit 1
fi

# 根据目标平台设置 Rust target 和可执行文件名
if [ "$TARGET_PLATFORM" == "windows" ]; then
    RUST_TARGET="x86_64-pc-windows-gnu"
    EXECUTABLE_NAME="airyc-server.exe"
else
    RUST_TARGET="x86_64-unknown-linux-gnu"
    EXECUTABLE_NAME="airyc-server"
fi

echo -e "${BLUE}Target platform:${NC} $TARGET_PLATFORM"
echo -e "${BLUE}Rust target:${NC} $RUST_TARGET"
echo -e "${BLUE}Build mode:${NC} $BUILD_MODE"
echo -e "${BLUE}Project root:${NC} $PROJECT_ROOT"

# 检查是否需要安装 target
echo -e "\n${BLUE}Checking Rust target...${NC}"
if ! rustup target list --installed | grep -q "$RUST_TARGET"; then
    echo -e "${YELLOW}Target $RUST_TARGET not installed, installing...${NC}"
    exit 1 
fi

# 构建语言服务器
echo -e "\n${BLUE}Building language server...${NC}"
cd "$PROJECT_ROOT"

if [ "$BUILD_MODE" == "release" ]; then
    cargo build --release --target "$RUST_TARGET" --bin airyc-server
    SOURCE_PATH="$PROJECT_ROOT/target/$RUST_TARGET/release/$EXECUTABLE_NAME"
else
    cargo build --target "$RUST_TARGET" --bin airyc-server
    SOURCE_PATH="$PROJECT_ROOT/target/$RUST_TARGET/debug/$EXECUTABLE_NAME"
fi

# 检查构建是否成功
if [ ! -f "$SOURCE_PATH" ]; then
    echo -e "${RED}Error: Build failed, executable not found at $SOURCE_PATH${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Build successful${NC}"

# 创建目标目录
TARGET_DIR="$SCRIPT_DIR/server/$TARGET_PLATFORM/$BUILD_MODE"
mkdir -p "$TARGET_DIR"

# 复制可执行文件
echo -e "\n${BLUE}Copying executable to plugin directory...${NC}"
cp "$SOURCE_PATH" "$TARGET_DIR/$EXECUTABLE_NAME"

echo -e "${GREEN}✓ Copied to: $TARGET_DIR/$EXECUTABLE_NAME${NC}"

# 显示文件信息
FILE_SIZE=$(du -h "$TARGET_DIR/$EXECUTABLE_NAME" | cut -f1)
echo -e "\n${GREEN}=== Build Complete ===${NC}"
echo -e "Target: $TARGET_PLATFORM"
echo -e "Mode: $BUILD_MODE"
echo -e "Executable: $TARGET_DIR/$EXECUTABLE_NAME"
echo -e "Size: $FILE_SIZE"
