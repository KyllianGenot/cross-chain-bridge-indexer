-- Enable UUID extension for unique identifiers
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Deposits Table: Stores deposit events from source chains (e.g., Holesky)
CREATE TABLE deposits (
    deposit_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    chain_id VARCHAR(50) NOT NULL, -- e.g., "holesky" or "target_chain"
    transaction_hash VARCHAR(66) NOT NULL UNIQUE, -- Ethereum tx hash (0x + 64 chars)
    block_number BIGINT NOT NULL, -- Block where the event occurred
    token_address VARCHAR(42) NOT NULL, -- Token contract address (0x + 40 chars)
    from_address VARCHAR(42) NOT NULL, -- Sender address
    to_address VARCHAR(42) NOT NULL, -- Recipient address on target chain
    amount NUMERIC(38, 18) NOT NULL, -- High-precision token amount
    nonce BIGINT NOT NULL, -- Unique nonce from Deposit event
    processed BOOLEAN DEFAULT FALSE, -- Whether distribution has occurred
    finality_confirmed BOOLEAN DEFAULT FALSE, -- Whether block finality is reached
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT unique_nonce_chain UNIQUE (nonce, chain_id)
);

-- Distributions Table: Stores distribution events on the target chain
CREATE TABLE distributions (
    distribution_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    deposit_id UUID NOT NULL REFERENCES deposits(deposit_id), -- Links to deposit
    chain_id VARCHAR(50) NOT NULL, -- Target chain ID
    transaction_hash VARCHAR(66) NOT NULL UNIQUE, -- Distribution tx hash
    block_number BIGINT NOT NULL, -- Block where distribution occurred
    token_address VARCHAR(42) NOT NULL, -- Token contract address
    recipient_address VARCHAR(42) NOT NULL, -- Recipient address
    amount NUMERIC(38, 18) NOT NULL, -- Distributed amount
    nonce BIGINT NOT NULL, -- Matches deposit nonce
    status VARCHAR(20) DEFAULT 'pending' CHECK (status IN ('pending', 'completed', 'failed')),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT unique_nonce_chain_dist UNIQUE (nonce, chain_id)
);

-- Indexes for Performance
CREATE INDEX idx_deposits_chain_block_nonce ON deposits(chain_id, block_number, nonce);
CREATE INDEX idx_deposits_processed_finality ON deposits(processed, finality_confirmed);
CREATE INDEX idx_distributions_deposit_id ON distributions(deposit_id);
CREATE INDEX idx_distributions_status ON distributions(status);