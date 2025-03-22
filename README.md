# Cross-Chain Bridge Indexer - Setup Guide

This guide will walk you through setting up and running the cross-chain bridge indexer, from environment preparation to using the bridge for cross-chain transfers.

## Prerequisites

### Install Rust

For Linux/macOS:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
Select option 1 for default installation, then:
```bash
source $HOME/.cargo/env
```

For Windows:
- Download and run rustup-init.exe from https://win.rustup.rs/x86_64
- Follow the on-screen instructions

Verify installation:
```bash
rustc --version
cargo --version
```

### Install PostgreSQL

For Ubuntu/Debian:
```bash
sudo apt update
sudo apt install postgresql postgresql-contrib
```

For macOS:
```bash
brew install postgresql
```

For Windows:
- Download the installer from https://www.postgresql.org/download/windows/
- Run the installer and follow the instructions
- Set a password for the postgres user

Verify installation:
```bash
psql --version
```

### Start PostgreSQL Service

Make sure to start the PostgreSQL service after installation:

For Ubuntu/Debian:
```bash
sudo systemctl start postgresql
sudo systemctl enable postgresql
```

For macOS:
```bash
brew services start postgresql
```

For Windows:
- PostgreSQL should start automatically after installation
- If not, open Services (services.msc) and start the PostgreSQL service

### Install Git

For Ubuntu/Debian:
```bash
sudo apt update
sudo apt install git
```

For macOS:
```bash
brew install git
```

For Windows:
- Download and install Git from https://git-scm.com/download/win
- Follow the installation wizard

Verify installation:
```bash
git --version
```

## Environment Setup

Create a project directory:
```bash
mkdir -p ~/projects
cd ~/projects
```

### Getting Blockchain API Keys

You'll need an API key from Alchemy:
- Create a free account at Alchemy
- Copy your API key for use in the .env file

### Wallet Preparation

You need a wallet with funds on both networks:
- Install MetaMask
- Add both networks to MetaMask:

Holesky Testnet:
- Network Name: Holesky Testnet
- RPC URL: https://ethereum-holesky.publicnode.com
- Chain ID: 17000
- Currency Symbol: ETH
- Block Explorer: https://holesky.etherscan.io

Base Sepolia:
- Network Name: Base Sepolia
- RPC URL: https://sepolia.base.org
- Chain ID: 84532
- Currency Symbol: ETH
- Block Explorer: https://sepolia.basescan.org

- Get test ETH for Holesky from Holesky Faucet
- For Base Sepolia ETH, use a bridge to convert Holesky ETH to Base Sepolia ETH

## Database Configuration

Set up the PostgreSQL database:
```bash
# Connect to PostgreSQL as the postgres user
psql -U postgres

# Create the database and user (in the PostgreSQL prompt)
CREATE DATABASE bridge_indexer;
CREATE USER indexer_user WITH ENCRYPTED PASSWORD 'your_secure_password';
GRANT ALL PRIVILEGES ON DATABASE bridge_indexer TO indexer_user;
GRANT ALL ON SCHEMA public TO indexer_user;
\q

# Test connection with the new user
psql -U indexer_user -d bridge_indexer -W
# Enter your password when prompted
```
Replace 'your_secure_password' with a strong password.

## Installing the Indexer

Get the indexer code and set it up:
```bash
# Clone the repository
git clone https://github.com/KyllianGenot/cross-chain-bridge-indexer.git
cd cross-chain-bridge-indexer/bridge-indexer

# Create the .env file
cp .env.example .env
```

Edit the .env file:
```
# Private key to sign transactions
PRIVATE_KEY=0xbba3b23700f47ad01a45ff16207cabcdaa260fbbd49d1268d907315630a680b0
WALLET_ADDRESS=0x9C0f4579e0260a75316019d26Fdd306d854aD2AD

# Holesky network configuration
HOLESKY_RPC_URL=https://eth-holesky.g.alchemy.com/v2/YOUR_ALCHEMY_API_KEY
HOLESKY_WS_URL=wss://eth-holesky.g.alchemy.com/v2/YOUR_ALCHEMY_API_KEY
HOLESKY_TEST_TOKEN=0xCd8b9bc8E9c7Ce2e886ea11afA07Da4d13F78a4E
HOLESKY_BRIDGE_ADDRESS=0xc2d3fF175A41B78d6b3897A778809973bF2978C9
HOLESKY_CONFIRMATION_BLOCKS=12

# Base Sepolia network configuration
TARGET_CHAIN_RPC_URL=https://base-sepolia.g.alchemy.com/v2/YOUR_ALCHEMY_API_KEY
TARGET_CHAIN_WS_URL=wss://base-sepolia.g.alchemy.com/v2/YOUR_ALCHEMY_API_KEY
TARGET_CHAIN_TEST_TOKEN=0xBD3f33605c2aB407e6036d3ad931EEaD01941eb5
TARGET_CHAIN_BRIDGE_ADDRESS=0xd30E3201a1e15C9Ba45F6bA3BCCE53a6a3A0d9ab
TARGET_CHAIN_CONFIRMATION_BLOCKS=12

# Database connection string
DATABASE_URL=postgres://indexer_user:your_secure_password@localhost:5432/bridge_indexer
```

Replace:
- YOUR_ALCHEMY_API_KEY: Your Alchemy API key
- YOUR_ETHERSCAN_API_KEY: Your Etherscan API key
- your_secure_password: The password you set for indexer_user

## Initialize Database Schema

Initialize the database:
```bash
# Connect to the database and apply the initialization script
psql -U indexer_user -d bridge_indexer -h localhost -f init.sql
```

## Build the Indexer

Compile the indexer:
```bash
# Build the project
cargo build
```

## Running the Indexer

Before starting the indexer, make sure to load the environment variables:
```bash
# Load environment variables
source .env

# Start the indexer
cargo run
```

The indexer will connect to both blockchain networks, subscribe to deposit events, and process them as they occur.

## Using the Bridge

### Contract Addresses

Holesky Network:
- Test Token: 0xCd8b9bc8E9c7Ce2e886ea11afA07Da4d13F78a4E
- Bridge Contract: 0xc2d3fF175A41B78d6b3897A778809973bF2978C9

Base Sepolia Network:
- Test Token: 0xBD3f33605c2aB407e6036d3ad931EEaD01941eb5
- Bridge Contract: 0xd30E3201a1e15C9Ba45F6bA3BCCE53a6a3A0d9ab

### Bridging Tokens from Holesky to Base Sepolia

To bridge tokens (either the default test tokens or your own) from Holesky to Base Sepolia:

1. **Approve the Bridge to Spend Your Tokens:**
   - Using cast:
     ```bash
     cast send $HOLESKY_TEST_TOKEN "approve(address,uint256)" $HOLESKY_BRIDGE_ADDRESS 1000000000000000000 --rpc-url $HOLESKY_RPC_URL --private-key $PRIVATE_KEY
     ```
     - Replace $HOLESKY_TEST_TOKEN with YOUR_HOLESKY_TOKEN_ADDRESS if using your own token.
     - 1000000000000000000 = 1 token (adjust based on your token's decimals).
   - Via Etherscan:
     - Go to your token contract on Holesky Etherscan.
     - Connect your wallet using the same private key specified in your .env file.
     - Call approve:
       - spender: $HOLESKY_BRIDGE_ADDRESS (0xc2d3fF175A41B78d6b3897A778809973bF2978C9)
       - amount: 1000000000000000000 (1 token in wei)

2. **Deposit Tokens into the Bridge:**
   - Using cast:
     ```bash
     cast send $HOLESKY_BRIDGE_ADDRESS "deposit(address,uint256,address)" $HOLESKY_TEST_TOKEN 1000000000000000000 $WALLET_ADDRESS --rpc-url $HOLESKY_RPC_URL --private-key $PRIVATE_KEY
     ```
     - Replace $HOLESKY_TEST_TOKEN with YOUR_HOLESKY_TOKEN_ADDRESS if using your own token.
   - Via Etherscan:
     - Go to the bridge contract on Holesky Etherscan.
     - Ensure you're connected with the wallet using the private key from your .env file.
     - Call deposit:
       - token: $HOLESKY_TEST_TOKEN (0xCd8b9bc8E9c7Ce2e886ea11afA07Da4d13F78a4E) or YOUR_HOLESKY_TOKEN_ADDRESS
       - amount: 1000000000000000000 (1 token in wei)
       - recipient: YOUR_WALLET_ADDRESS

3. **Wait for Indexer to Process:**
   - The indexer listens for the Deposit event on Holesky.
   - After $HOLESKY_CONFIRMATION_BLOCKS (12 blocks), it triggers distribute on Base Sepolia using the bridge owner's private key.
   - The corresponding tokens (e.g., $TARGET_CHAIN_TEST_TOKEN or YOUR_BASE_SEPOLIA_TOKEN_ADDRESS) are transferred to YOUR_WALLET_ADDRESS on Base Sepolia.

4. **Check Receipt on Base Sepolia:**
   - Monitor your wallet balance on Base Sepolia for the tokens.
   - Use Basescan to verify the Distribution event: Bridge Contract.

### Bridging Tokens from Base Sepolia to Holesky

To bridge tokens back from Base Sepolia to Holesky:

1. **Approve the Bridge to Spend Your Tokens:**
   - Using cast:
     ```bash
     cast send $TARGET_CHAIN_TEST_TOKEN "approve(address,uint256)" $TARGET_CHAIN_BRIDGE_ADDRESS 1000000000000000000 --rpc-url $TARGET_CHAIN_RPC_URL --private-key $PRIVATE_KEY
     ```
     - Replace $TARGET_CHAIN_TEST_TOKEN with YOUR_BASE_SEPOLIA_TOKEN_ADDRESS if using your own token.
   - Via Basescan:
     - Go to your token contract on Basescan.
     - Connect your wallet using the same private key specified in your .env file.
     - Call approve:
       - spender: $TARGET_CHAIN_BRIDGE_ADDRESS (0xd30E3201a1e15C9Ba45F6bA3BCCE53a6a3A0d9ab)
       - amount: 1000000000000000000

2. **Deposit Tokens into the Bridge:**
   - Using cast:
     ```bash
     cast send $TARGET_CHAIN_BRIDGE_ADDRESS "deposit(address,uint256,address)" $TARGET_CHAIN_TEST_TOKEN 1000000000000000000 $WALLET_ADDRESS --rpc-url $TARGET_CHAIN_RPC_URL --private-key $PRIVATE_KEY
     ```
     - Replace $TARGET_CHAIN_TEST_TOKEN with YOUR_BASE_SEPOLIA_TOKEN_ADDRESS if using your own token.
   - Via Basescan:
     - Go to the bridge contract on Basescan.
     - Ensure you're connected with the wallet using the private key from your .env file.
     - Call deposit:
       - token: $TARGET_CHAIN_TEST_TOKEN (0xBD3f33605c2aB407e6036d3ad931EEaD01941eb5) or YOUR_BASE_SEPOLIA_TOKEN_ADDRESS
       - amount: 1000000000000000000
       - recipient: YOUR_WALLET_ADDRESS

3. **Wait for Indexer to Process:**
   - The indexer detects the Deposit event on Base Sepolia.
   - After $TARGET_CHAIN_CONFIRMATION_BLOCKS (12 blocks), it triggers distribute on Holesky.
   - Tokens are transferred to YOUR_WALLET_ADDRESS on Holesky.

4. **Check Receipt on Holesky:**
   - Verify your wallet balance on Holesky.
   - Check the Distribution event on Holesky Etherscan.

## Quick Command Reference

Here's a consolidated list of all necessary commands for setting up and running the indexer:

# PostgreSQL Setup

```bash
# Start PostgreSQL (for Linux)
sudo systemctl start postgresql
sudo systemctl enable postgresql
```

```bash
# Start PostgreSQL (for macOS)
brew services start postgresql
```

```bash
# Connect to PostgreSQL as postgres user to create database and user
psql -U postgres
```

After running the command above, execute these SQL commands in the PostgreSQL prompt:
```sql
CREATE DATABASE bridge_indexer;
CREATE USER indexer_user WITH ENCRYPTED PASSWORD 'your_secure_password';
GRANT ALL PRIVILEGES ON DATABASE bridge_indexer TO indexer_user;
GRANT ALL ON SCHEMA public TO indexer_user;
\q
```

```bash
# Test connection with the new user (you'll be prompted for password)
psql -U indexer_user -d bridge_indexer -W
```

# Indexer Setup

```bash
# Create a project directory
mkdir -p ~/projects
cd ~/projects
```

```bash
# Clone repository
git clone https://github.com/KyllianGenot/cross-chain-bridge-indexer.git
cd cross-chain-bridge-indexer/bridge-indexer
```

```bash
# Copy the example environment file
cp .env.example .env
```

At this point, edit the .env file with your details (private keys, API keys, etc.)

```bash
# Initialize database schema
psql -U indexer_user -d bridge_indexer -h localhost -f init.sql
```

```bash
# Build the indexer
cargo build
```

# Running the Indexer

```bash
# Load environment variables and run the indexer
source .env
cargo run
```

# Using the Bridge (Holesky → Base Sepolia)

```bash
# Approve tokens for the bridge to spend
cast send $HOLESKY_TEST_TOKEN "approve(address,uint256)" $HOLESKY_BRIDGE_ADDRESS 1000000000000000000 --rpc-url $HOLESKY_RPC_URL --private-key $PRIVATE_KEY
```

```bash
# Deposit tokens into the bridge
cast send $HOLESKY_BRIDGE_ADDRESS "deposit(address,uint256,address)" $HOLESKY_TEST_TOKEN 1000000000000000000 $WALLET_ADDRESS --rpc-url $HOLESKY_RPC_URL --private-key $PRIVATE_KEY
```

# Using the Bridge (Base Sepolia → Holesky)

```bash
# Approve tokens for the bridge to spend
cast send $TARGET_CHAIN_TEST_TOKEN "approve(address,uint256)" $TARGET_CHAIN_BRIDGE_ADDRESS 1000000000000000000 --rpc-url $TARGET_CHAIN_RPC_URL --private-key $PRIVATE_KEY
```

```bash
# Deposit tokens into the bridge
cast send $TARGET_CHAIN_BRIDGE_ADDRESS "deposit(address,uint256,address)" $TARGET_CHAIN_TEST_TOKEN 1000000000000000000 $WALLET_ADDRESS --rpc-url $TARGET_CHAIN_RPC_URL --private-key $PRIVATE_KEY
```

# Troubleshooting

```bash
# Check if PostgreSQL is running (Linux)
sudo systemctl status postgresql
```

```bash
# Check if PostgreSQL is running (macOS)
brew services list
```

```bash
# Test the database connection
psql -U indexer_user -d bridge_indexer -h localhost -c "SELECT 1"
```

```bash
# Test an RPC endpoint
curl -X POST -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' \
  $HOLESKY_RPC_URL
```

Happy bridging!