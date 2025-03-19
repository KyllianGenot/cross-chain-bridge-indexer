use ethers::prelude::*;

// Use the abigen! macro to generate the contract bindings
abigen!(TokenBridge, "abis/TokenBridge.json");