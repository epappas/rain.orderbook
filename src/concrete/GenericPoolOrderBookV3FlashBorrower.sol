// SPDX-License-Identifier: CAL
pragma solidity =0.8.19;

import "src/interface/ierc3156/IERC3156FlashLender.sol";
import "src/interface/ierc3156/IERC3156FlashBorrower.sol";

import "src/abstract/OrderBookV3FlashBorrower.sol";

/// @dev Metadata hash for `DeployerDiscoverableMetaV1`.
/// - ABI for GenericPoolOrderBookFlashBorrower
/// - Interpreter caller metadata V1 for GenericPoolOrderBookFlashBorrower
bytes32 constant CALLER_META_HASH = bytes32(0x3d6909481820fc692906b0477e8f98248e84973bc8b8d5ac935132857d4f4125);

/// @title GenericPoolOrderBookV3FlashBorrower
/// Implements the OrderBookV3FlashBorrower interface for a external liquidity
/// source that behaves vaguely like a standard AMM. The `exchangeData` from
/// `arb` is decoded into a spender, pool and callData. The `callData` is
/// literally the encoded function call to the pool. This allows the `arb`
/// caller to process a trade against any liquidity source that can swap tokens
/// within a single function call.
/// The `spender` is the address that will be approved to spend the input token
/// on `takeOrders`, which is almost always going to be the pool itself. If you
/// are unsure, simply set it to the pool address.
contract GenericPoolOrderBookV3FlashBorrower is OrderBookV3FlashBorrower {
    using SafeERC20 for IERC20;
    using Address for address;

    constructor(DeployerDiscoverableMetaV2ConstructionConfig memory config)
        OrderBookV3FlashBorrower(CALLER_META_HASH, config)
    {}

    /// @inheritdoc OrderBookV3FlashBorrower
    function _exchange(TakeOrdersConfigV2 memory takeOrders, bytes memory exchangeData) internal virtual override {
        (address spender, address pool, bytes memory encodedFunctionCall) =
            abi.decode(exchangeData, (address, address, bytes));

        IERC20(takeOrders.input).safeApprove(spender, 0);
        IERC20(takeOrders.input).safeApprove(spender, type(uint256).max);
        bytes memory returnData = pool.functionCallWithValue(encodedFunctionCall, address(this).balance);
        // Nothing can be done with returnData as 3156 does not support it.
        (returnData);
        IERC20(takeOrders.input).safeApprove(spender, 0);
    }

    /// Allow receiving gas.
    fallback() external onlyNotInitializing {}
}