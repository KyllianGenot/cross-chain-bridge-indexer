# Cross-Chain Bridge Indexer

This guide provides instructions for setting up and running the cross-chain bridge indexer for token transfers between Holesky and Base Sepolia networks.

## Quick Start Guide

### Prerequisites Checklist

Before starting, make sure you have:
- [ ] PostgreSQL password (existing or you'll create one)
- [ ] Alchemy API key (single key works for both networks)

### Quick Setup Commands

```bash
# Clone repository
git clone https://github.com/KyllianGenot/cross-chain-bridge-indexer.git
cd cross-chain-bridge-indexer/bridge-indexer
```

```bash
# Setup PostgreSQL database
psql -U postgres
```

In PostgreSQL prompt this to create the database and the user:
```sql
CREATE DATABASE bridge_indexer;
CREATE USER indexer_user WITH ENCRYPTED PASSWORD 'your_secure_password';
GRANT ALL PRIVILEGES ON DATABASE bridge_indexer TO indexer_user;

-- Connect to the bridge_indexer database
\c bridge_indexer
```
Then, prompt this to set the permissions:
```sql
-- Set all necessary permissions
ALTER SCHEMA public OWNER TO indexer_user;
GRANT ALL ON SCHEMA public TO indexer_user;
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO indexer_user;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO indexer_user;
GRANT ALL PRIVILEGES ON ALL FUNCTIONS IN SCHEMA public TO indexer_user;

-- Set default privileges for future objects
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TABLES TO indexer_user;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON SEQUENCES TO indexer_user;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON FUNCTIONS TO indexer_user;
\q
```

```bash
# Configure environment
cp .env.example .env
```

Edit .env and update:
- Replace `YOUR_ALCHEMY_API_KEY` with your Alchemy API key
- Update `your_secure_password` in the DATABASE_URL

```bash
# Initialize database schema
psql -U indexer_user -d bridge_indexer -h localhost -f init.sql
```

```bash
# Build and run
cargo build
source .env
cargo run
```

### Check Token Balances

Open a new terminal window to run these commands while keeping the indexer running in the original terminal:

```bash
# Load environment variables in the new terminal
source .env

# Check token balance on Holesky 
cast call $HOLESKY_TEST_TOKEN "balanceOf(address)(uint256)" $WALLET_ADDRESS --rpc-url $HOLESKY_RPC_URL
```

```bash
# Check token balance on Base Sepolia
cast call $TARGET_CHAIN_TEST_TOKEN "balanceOf(address)(uint256)" $WALLET_ADDRESS --rpc-url $TARGET_CHAIN_RPC_URL
```

### Bridge Tokens (Holesky to Base Sepolia)

In your second terminal:

```bash
# Approve tokens
cast send $HOLESKY_TEST_TOKEN "approve(address,uint256)" $HOLESKY_BRIDGE_ADDRESS 1000000000000000000 --rpc-url $HOLESKY_RPC_URL --private-key $PRIVATE_KEY
```

```bash
# Deposit tokens
cast send $HOLESKY_BRIDGE_ADDRESS "deposit(address,uint256,address)" $HOLESKY_TEST_TOKEN 1000000000000000000 $WALLET_ADDRESS --rpc-url $HOLESKY_RPC_URL --private-key $PRIVATE_KEY
```

Switch back to your first terminal to observe the indexer processing the transaction. You should see log messages indicating that the deposit event was detected.

After ~2-5 minutes, check the token balance in your second terminal:

```bash
# Verify receipt
cast call $TARGET_CHAIN_TEST_TOKEN "balanceOf(address)(uint256)" $WALLET_ADDRESS --rpc-url $TARGET_CHAIN_RPC_URL
```

### Bridge Tokens (Base Sepolia to Holesky)

In your second terminal:

```bash
# Approve tokens
cast send $TARGET_CHAIN_TEST_TOKEN "approve(address,uint256)" $TARGET_CHAIN_BRIDGE_ADDRESS 1000000000000000000 --rpc-url $TARGET_CHAIN_RPC_URL --private-key $PRIVATE_KEY
```

```bash
# Deposit tokens
cast send $TARGET_CHAIN_BRIDGE_ADDRESS "deposit(address,uint256,address)" $TARGET_CHAIN_TEST_TOKEN 1000000000000000000 $WALLET_ADDRESS --rpc-url $TARGET_CHAIN_RPC_URL --private-key $PRIVATE_KEY
```

Switch back to your first terminal to observe the indexer processing the transaction. You should see log messages indicating that the deposit event was detected.

After ~2-5 minutes, check the token balance in your second terminal:

```bash
# Verify receipt
cast call $HOLESKY_TEST_TOKEN "balanceOf(address)(uint256)" $WALLET_ADDRESS --rpc-url $HOLESKY_RPC_URL
```

## Complete Setup Guide

### Prerequisites

#### Install Rust

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

#### Install PostgreSQL

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

#### Start PostgreSQL Service

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

#### Install Git

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

### Environment Setup

Create a project directory:
```bash
mkdir -p ~/projects
cd ~/projects
```

#### Getting Blockchain API Keys

You'll need an API key from Alchemy:
- Create a free account at Alchemy
- Copy your API key for use in the .env file (same key works for both networks)

### Database Configuration

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

### Installing the Indexer

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
- YOUR_ALCHEMY_API_KEY: Your Alchemy API key (same key for both networks)
- your_secure_password: The password you set for indexer_user

### Initialize Database Schema

Initialize the database:
```bash
# Connect to the database and apply the initialization script
psql -U indexer_user -d bridge_indexer -h localhost -f init.sql
```

### Build the Indexer

Compile the indexer:
```bash
# Build the project
cargo build
```

### Running the Indexer

Before starting the indexer, make sure to load the environment variables:
```bash
# Load environment variables
source .env

# Start the indexer
cargo run
```

The indexer will connect to both blockchain networks, subscribe to deposit events, and process them as they occur. Keep this terminal window open and running while you perform bridge operations in a new terminal.

## Using the Bridge

### Contract Addresses

Holesky Network:
- Test Token: 0xCd8b9bc8E9c7Ce2e886ea11afA07Da4d13F78a4E
- Bridge Contract: 0xc2d3fF175A41B78d6b3897A778809973bF2978C9

Base Sepolia Network:
- Test Token: 0xBD3f33605c2aB407e6036d3ad931EEaD01941eb5
- Bridge Contract: 0xd30E3201a1e15C9Ba45F6bA3BCCE53a6a3A0d9ab

### Checking Token Balances

Open a new terminal window for these operations, keeping the indexer running in the original terminal:

```bash
# Navigate to project directory in the new terminal
cd cross-chain-bridge-indexer/bridge-indexer

# Load environment variables in the new terminal
source .env

# Check Holesky token balance
cast call $HOLESKY_TEST_TOKEN "balanceOf(address)(uint256)" $WALLET_ADDRESS --rpc-url $HOLESKY_RPC_URL
```

```bash
# Check Base Sepolia token balance
cast call $TARGET_CHAIN_TEST_TOKEN "balanceOf(address)(uint256)" $WALLET_ADDRESS --rpc-url $TARGET_CHAIN_RPC_URL
```

### Bridging Tokens from Holesky to Base Sepolia

In your second terminal:

1. **Approve the Bridge to Spend Your Tokens:**
   ```bash
   cast send $HOLESKY_TEST_TOKEN "approve(address,uint256)" $HOLESKY_BRIDGE_ADDRESS 1000000000000000000 --rpc-url $HOLESKY_RPC_URL --private-key $PRIVATE_KEY
   ```
   - Replace $HOLESKY_TEST_TOKEN with YOUR_HOLESKY_TOKEN_ADDRESS if using your own token.
   - 1000000000000000000 = 1 token (adjust based on your token's decimals).

2. **Deposit Tokens into the Bridge:**
   ```bash
   cast send $HOLESKY_BRIDGE_ADDRESS "deposit(address,uint256,address)" $HOLESKY_TEST_TOKEN 1000000000000000000 $WALLET_ADDRESS --rpc-url $HOLESKY_RPC_URL --private-key $PRIVATE_KEY
   ```

3. **Switch back to the first terminal** to observe the indexer processing the transaction. You should see log messages indicating that a deposit event was detected and processed.

4. **Verify Receipt on Base Sepolia** (back in the second terminal):
   ```bash
   # Wait ~2-5 minutes for processing
   cast call $TARGET_CHAIN_TEST_TOKEN "balanceOf(address)(uint256)" $WALLET_ADDRESS --rpc-url $TARGET_CHAIN_RPC_URL
   ```

### Bridging Tokens from Base Sepolia to Holesky

In your second terminal:

1. **Approve the Bridge to Spend Your Tokens:**
   ```bash
   cast send $TARGET_CHAIN_TEST_TOKEN "approve(address,uint256)" $TARGET_CHAIN_BRIDGE_ADDRESS 1000000000000000000 --rpc-url $TARGET_CHAIN_RPC_URL --private-key $PRIVATE_KEY
   ```

2. **Deposit Tokens into the Bridge:**
   ```bash
   cast send $TARGET_CHAIN_BRIDGE_ADDRESS "deposit(address,uint256,address)" $TARGET_CHAIN_TEST_TOKEN 1000000000000000000 $WALLET_ADDRESS --rpc-url $TARGET_CHAIN_RPC_URL --private-key $PRIVATE_KEY
   ```

3. **Switch back to the first terminal** to observe the indexer processing the transaction. You should see log messages indicating that a deposit event was detected and processed.

4. **Verify Receipt on Holesky** (back in the second terminal):
   ```bash
   # Wait ~2-5 minutes for processing
   cast call $HOLESKY_TEST_TOKEN "balanceOf(address)(uint256)" $WALLET_ADDRESS --rpc-url $HOLESKY_RPC_URL
   ```

## Troubleshooting

If you encounter database connection issues:
```bash
# Check if PostgreSQL is running
sudo systemctl status postgresql  # Linux
brew services list  # macOS

# Test the connection
psql -U indexer_user -d bridge_indexer -h localhost -c "SELECT 1"
```

If the indexer can't connect to a blockchain:
```bash
# Test an RPC endpoint
curl -X POST -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' \
  $HOLESKY_RPC_URL
```

If the indexer is running but not detecting events:
- Check that both terminals have loaded the environment variables using `source .env`
- Verify that your Alchemy API key is correctly set in the .env file
- Make sure you're using the correct contract addresses
- Check that your wallet has sufficient tokens and ETH for gas

Happy bridging!