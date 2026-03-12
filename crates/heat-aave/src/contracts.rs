/// Minimal Alloy `sol!` interfaces for Aave V3 contracts.
///
/// Only the methods Heat actually calls are declared here.
/// Contract addresses are resolved at runtime via `PoolAddressesProvider`
/// (see `resolver.rs`).
use alloy::sol;

// ---------------------------------------------------------------------------
// IPoolAddressesProvider — canonical market registry
// ---------------------------------------------------------------------------

sol! {
    #[allow(missing_docs)]
    #[sol(rpc)]
    interface IPoolAddressesProvider {
        function getPool() external view returns (address);
        function getPoolDataProvider() external view returns (address);
        function getPriceOracle() external view returns (address);
        function getMarketId() external view returns (string memory);
    }
}

// ---------------------------------------------------------------------------
// IPool — core read/write
// ---------------------------------------------------------------------------

sol! {
    #[allow(missing_docs)]
    #[sol(rpc)]
    interface IPool {
        /// Supply assets into the pool.
        function supply(address asset, uint256 amount, address onBehalfOf, uint16 referralCode) external;

        /// Withdraw assets from the pool. Returns the final withdrawn amount.
        function withdraw(address asset, uint256 amount, address to) external returns (uint256);

        /// Account-level health summary.
        function getUserAccountData(address user) external view returns (
            uint256 totalCollateralBase,
            uint256 totalDebtBase,
            uint256 availableBorrowsBase,
            uint256 currentLiquidationThreshold,
            uint256 ltv,
            uint256 healthFactor
        );

        /// List all reserve underlying addresses.
        function getReservesList() external view returns (address[] memory);
    }
}

// ---------------------------------------------------------------------------
// IWrappedTokenGatewayV3 — native ETH supply/withdraw via WETH wrapping
// ---------------------------------------------------------------------------

sol! {
    #[allow(missing_docs)]
    #[sol(rpc)]
    interface IWrappedTokenGatewayV3 {
        /// Wrap ETH and supply WETH to the Pool in one transaction.
        function depositETH(address pool, address onBehalfOf, uint16 referralCode) external payable;

        /// Withdraw WETH from the Pool and unwrap to native ETH.
        function withdrawETH(address pool, uint256 amount, address to) external;
    }
}

// ---------------------------------------------------------------------------
// IPoolDataProvider (AaveProtocolDataProvider) — reserve config & user data
// ---------------------------------------------------------------------------

sol! {
    #[allow(missing_docs)]
    #[sol(rpc)]
    interface IPoolDataProvider {
        struct TokenData {
            string symbol;
            address tokenAddress;
        }

        function getAllReservesTokens() external view returns (TokenData[] memory);

        function getReserveConfigurationData(address asset) external view returns (
            uint256 decimals,
            uint256 ltv,
            uint256 liquidationThreshold,
            uint256 liquidationBonus,
            uint256 reserveFactor,
            bool usageAsCollateralEnabled,
            bool borrowingEnabled,
            bool stableBorrowRateEnabled,
            bool isActive,
            bool isFrozen
        );

        function getReserveData(address asset) external view returns (
            uint256 unbacked,
            uint256 accruedToTreasuryScaled,
            uint256 totalAToken,
            uint256 totalStableDebt,
            uint256 totalVariableDebt,
            uint256 liquidityRate,
            uint256 variableBorrowRate,
            uint256 stableBorrowRate,
            uint256 averageStableBorrowRate,
            uint256 liquidityIndex,
            uint256 variableBorrowIndex,
            uint40 lastUpdateTimestamp
        );

        function getUserReserveData(address asset, address user) external view returns (
            uint256 currentATokenBalance,
            uint256 currentStableDebt,
            uint256 currentVariableDebt,
            uint256 principalStableDebt,
            uint256 scaledVariableDebt,
            uint256 stableBorrowRate,
            uint256 liquidityRate,
            uint40 stableRateLastUpdated,
            bool usageAsCollateralEnabled
        );

        function getReserveTokensAddresses(address asset) external view returns (
            address aTokenAddress,
            address stableDebtTokenAddress,
            address variableDebtTokenAddress
        );

        function getReserveCaps(address asset) external view returns (
            uint256 borrowCap,
            uint256 supplyCap
        );

        function getPaused(address asset) external view returns (bool isPaused);
    }
}
