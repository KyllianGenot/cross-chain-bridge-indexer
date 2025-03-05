use ethers::prelude::*;
use std::fs;

// Load the ABI from TokenBridge.json
pub fn load_token_bridge_abi() -> Abi {
    let abi_json = fs::read_to_string("abis/TokenBridge.json")
        .expect("Failed to read TokenBridge.json");
    abi_json.parse().expect("Failed to parse ABI")
}

// Define the TokenBridge contract interface
abigen!(
    TokenBridge,
    "abis/TokenBridge.json",
    events {
        Deposit(token, from, to, amount, nonce),
        Distribution(token, to, amount, nonce),
    }
);