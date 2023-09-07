// SPDX-License-Identifier: CAL
pragma solidity =0.8.19;

import "lib/forge-std/src/Test.sol";
import "lib/openzeppelin-contracts/contracts/proxy/Clones.sol";
import "lib/openzeppelin-contracts/contracts/token/ERC20/ERC20.sol";

import "test/util/lib/LibTestConstants.sol";
import "test/util/lib/LibGenericPoolOrderBookV3FlashBorrowerConstants.sol";
import "test/util/concrete/FlashLendingMockOrderBook.sol";

import "src/concrete/GenericPoolOrderBookV3FlashBorrower.sol";
import "src/interface/unstable/IOrderBookV3.sol";

contract Token is ERC20 {
    constructor() ERC20("Token", "TKN") {}

    function mint(address receiver_, uint256 amount_) external {
        _mint(receiver_, amount_);
    }
}

contract Mock0xProxy {
    fallback() external {
        Address.sendValue(payable(msg.sender), address(this).balance);
    }
}

contract GenericPoolOrderBookV3FlashBorrowerTest is Test {
    address immutable deployer;
    address immutable implementation;

    constructor() {
        deployer = address(uint160(uint256(keccak256("deployer.rain.test"))));
        // All non-mocked calls will revert.
        vm.etch(deployer, REVERTING_MOCK_BYTECODE);
        vm.mockCall(
            deployer,
            abi.encodeWithSelector(IExpressionDeployerV2.deployExpression.selector),
            abi.encode(address(0), address(0), address(0))
        );
        bytes memory meta = vm.readFileBinary(GENERIC_POOL_ORDER_BOOK_FLASH_BORROWER_META_PATH);
        console2.log("GenericPoolOrderBookV3FlashBorrowerTest meta hash:");
        console2.logBytes32(keccak256(meta));
        implementation = address(
            new GenericPoolOrderBookV3FlashBorrower(DeployerDiscoverableMetaV2ConstructionConfig(
            deployer,
            meta
            ))
        );
    }

    function testTakeOrdersSender(Order memory order, uint256 inputIOIndex, uint256 outputIOIndex) public {
        vm.assume(order.validInputs.length > 0);
        inputIOIndex = bound(inputIOIndex, 0, order.validInputs.length - 1);
        vm.assume(order.validOutputs.length > 0);
        outputIOIndex = bound(outputIOIndex, 0, order.validOutputs.length - 1);

        FlashLendingMockOrderBook ob = new FlashLendingMockOrderBook();
        Mock0xProxy proxy = new Mock0xProxy();

        Token takerInput = new Token();
        Token takerOutput = new Token();

        GenericPoolOrderBookV3FlashBorrower arb = GenericPoolOrderBookV3FlashBorrower(Clones.clone(implementation));
        arb.initialize(
            abi.encode(
                OrderBookV3FlashBorrowerConfigV2(
                    address(ob), EvaluableConfigV2(IExpressionDeployerV2(address(0)), "", new uint256[](0)), ""
                )
            )
        );

        order.validInputs[inputIOIndex].token = address(takerOutput);
        order.validOutputs[outputIOIndex].token = address(takerInput);

        TakeOrderConfig[] memory orders = new TakeOrderConfig[](1);
        orders[0] = TakeOrderConfig(order, inputIOIndex, outputIOIndex, new SignedContextV1[](0));

        arb.arb(
            TakeOrdersConfigV2(0, type(uint256).max, type(uint256).max, orders, ""),
            0,
            abi.encode(address(proxy), address(proxy), "")
        );
    }

    function testMinimumOutput(
        Order memory order,
        uint256 inputIOIndex,
        uint256 outputIOIndex,
        uint256 minimumOutput,
        uint256 mintAmount
    ) public {
        vm.assume(order.validInputs.length > 0);
        inputIOIndex = bound(inputIOIndex, 0, order.validInputs.length - 1);
        vm.assume(order.validOutputs.length > 0);
        outputIOIndex = bound(outputIOIndex, 0, order.validOutputs.length - 1);

        vm.assume(minimumOutput > mintAmount);
        FlashLendingMockOrderBook ob = new FlashLendingMockOrderBook();
        Mock0xProxy proxy = new Mock0xProxy();

        Token takerInput = new Token();
        Token takerOutput = new Token();

        GenericPoolOrderBookV3FlashBorrower arb = GenericPoolOrderBookV3FlashBorrower(Clones.clone(implementation));
        arb.initialize(
            abi.encode(
                OrderBookV3FlashBorrowerConfigV2(
                    address(ob), EvaluableConfigV2(IExpressionDeployerV2(address(0)), "", new uint256[](0)), ""
                )
            )
        );

        takerOutput.mint(address(arb), mintAmount);

        order.validInputs[inputIOIndex].token = address(takerOutput);
        order.validOutputs[outputIOIndex].token = address(takerInput);

        TakeOrderConfig[] memory orders = new TakeOrderConfig[](1);
        orders[0] = TakeOrderConfig(order, inputIOIndex, outputIOIndex, new SignedContextV1[](0));

        vm.expectRevert(abi.encodeWithSelector(MinimumOutput.selector, minimumOutput, mintAmount));
        arb.arb(
            TakeOrdersConfigV2(0, type(uint256).max, type(uint256).max, orders, ""),
            minimumOutput,
            abi.encode(address(proxy), address(proxy), "")
        );
    }

    // Allow receiving funds at end of arb.
    fallback() external {}
}
