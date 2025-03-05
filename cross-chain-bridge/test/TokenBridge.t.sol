// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Test.sol";
import "../src/TokenBridge.sol";
import "../src/TestToken.sol";

contract TokenBridgeTest is Test {
    TokenBridge public bridge;
    TestToken public token;
    
    address public owner = address(this);
    address public user = makeAddr("user");
    address public distributor = makeAddr("distributor");
    
    function setUp() public {
        // Deploy test token
        token = new TestToken();
        
        // Deploy bridge contract
        bridge = new TokenBridge();
        
        vm.startPrank(owner);
        // Add test token to supported tokens
        bridge.addSupportedToken(address(token));
        vm.stopPrank();
        
        // Transfer some tokens to user for testing
        token.transfer(user, 1000 ether);
    }
    
    function testDeposit() public {
        // Set user as msg.sender
        vm.startPrank(user);
        
        // Approve bridge to spend tokens
        token.approve(address(bridge), 100 ether);
        
        // Deposit tokens
        bridge.deposit(address(token), 100 ether, user);
        
        // Stop being user
        vm.stopPrank();
        
        // Check if tokens were transferred to the bridge
        assertEq(token.balanceOf(address(bridge)), 100 ether);
    }
    
    function testDistributeByOwner() public {
        // First deposit tokens to the bridge as user
        vm.startPrank(user);
        token.approve(address(bridge), 100 ether);
        bridge.deposit(address(token), 100 ether, user);
        vm.stopPrank();
        
        // Remember the user's balance after deposit
        uint256 userBalanceAfterDeposit = token.balanceOf(user);
        
        // Now distribute tokens as owner
        bridge.distribute(
            address(token),
            user,
            100 ether,
            0 // Nonce from the deposit
        );
        
        // Check if tokens were transferred to the user
        assertEq(token.balanceOf(user), userBalanceAfterDeposit + 100 ether);
    }
    
    function testDistributeByNonOwnerFails() public {
        // Try to distribute tokens as a non-owner
        vm.startPrank(user);
        
        vm.expectRevert("Bridge: not distributor");
        bridge.distribute(
            address(token),
            user,
            100 ether,
            0
        );
        
        vm.stopPrank();
    }
    
    // Add more tests for other functionality
}