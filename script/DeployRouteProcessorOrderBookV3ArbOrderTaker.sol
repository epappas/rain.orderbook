// SPDX-License-Identifier: CAL
pragma solidity =0.8.19;

import {Script, console2} from "forge-std/Script.sol";
import {
    RouteProcessorOrderBookV3ArbOrderTaker,
    DeployerDiscoverableMetaV3ConstructionConfig
} from "src/concrete/RouteProcessorOrderBookV3ArbOrderTaker.sol";
import {I9R_DEPLOYER} from "./DeployConstants.sol";

/// @title DeployRouteProcessorOrderBookV3ArbOrderTaker
/// @notice A script that deploys a `RouteProcessorOrderBookV3ArbOrderTaker`. This
/// is intended to be run on every commit by CI to a testnet such as mumbai, then
/// cross chain deployed to whatever mainnet is required, by users.
contract DeployRouteProcessorOrderBookV3ArbOrderTaker is Script {
    /// We are avoiding using ffi here, instead forcing the script runner to
    /// provide the built metadata. On CI this is achieved by using the rain cli.
    function run(bytes memory meta) external {
        uint256 deployerPrivateKey = vm.envUint("DEPLOYMENT_KEY");

        console2.log("RouteProcessorOrderBookV3ArbOrderTaker meta hash:");
        console2.logBytes32(keccak256(meta));

        vm.startBroadcast(deployerPrivateKey);
        RouteProcessorOrderBookV3ArbOrderTaker deployed =
            new RouteProcessorOrderBookV3ArbOrderTaker(DeployerDiscoverableMetaV3ConstructionConfig(I9R_DEPLOYER, meta));
        (deployed);
        vm.stopBroadcast();
    }
}
