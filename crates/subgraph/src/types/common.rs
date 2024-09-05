use crate::schema;
use serde::Serialize;
use typeshare::typeshare;

#[derive(cynic::QueryVariables, Debug, Clone)]
#[typeshare]
pub struct IdQueryVariables<'a> {
    pub id: &'a cynic::Id,
}

#[derive(cynic::QueryVariables, Debug, Clone)]
#[typeshare]
pub struct PaginationQueryVariables {
    pub first: Option<i32>,
    pub skip: Option<i32>,
}

#[derive(cynic::QueryVariables, Debug, Clone)]
#[typeshare]
pub struct PaginationWithIdQueryVariables {
    pub first: Option<i32>,
    pub id: Bytes,
    pub skip: Option<i32>,
}

#[derive(cynic::QueryFragment, Debug, Serialize, Clone)]
#[typeshare]
pub struct Orderbook {
    pub id: Bytes,
}

#[typeshare]
pub type RainMetaV1 = Bytes;

#[derive(cynic::QueryFragment, Debug, Serialize, Clone)]
#[typeshare]
pub struct Order {
    pub id: Bytes,
    pub order_bytes: Bytes,
    pub order_hash: Bytes,
    pub owner: Bytes,
    pub outputs: Vec<Vault>,
    pub inputs: Vec<Vault>,
    pub orderbook: Orderbook,
    pub active: bool,
    pub timestamp_added: BigInt,
    pub meta: Option<RainMetaV1>,
    pub add_events: Vec<AddOrder>,
}

#[derive(cynic::QueryFragment, Debug, Serialize, Clone)]
#[cynic(graphql_type = "Order")]
#[typeshare]
pub struct OrderAsIO {
    pub id: Bytes,
    pub order_hash: Bytes,
    pub active: bool,
}

#[derive(cynic::QueryFragment, Debug, Serialize, Clone)]
#[typeshare]
pub struct Vault {
    pub id: Bytes,
    pub owner: Bytes,
    pub vault_id: BigInt,
    pub balance: BigInt,
    pub token: Erc20,
    pub orderbook: Orderbook,
    pub orders_as_output: Vec<OrderAsIO>,
    pub orders_as_input: Vec<OrderAsIO>,
    pub balance_changes: Vec<VaultBalanceChange>,
}

#[derive(cynic::QueryFragment, Debug, Clone, Serialize)]
#[cynic(graphql_type = "Vault")]
#[typeshare]
pub struct VaultBalanceChangeVault {
    pub id: Bytes,
    pub token: Erc20,
}

#[derive(cynic::QueryFragment, Debug, Clone, Serialize)]
#[cynic(graphql_type = "VaultBalanceChange")]
#[typeshare]
pub struct VaultBalanceChangeUnwrapped {
    pub __typename: String,
    pub amount: BigInt,
    pub new_vault_balance: BigInt,
    pub old_vault_balance: BigInt,
    pub vault: VaultBalanceChangeVault,
    pub timestamp: BigInt,
    pub transaction: Transaction,
    pub orderbook: Orderbook,
}

#[derive(cynic::InlineFragments, Debug, Clone, Serialize)]
#[serde(tag = "__typename", content = "data")]
#[typeshare]
pub enum VaultBalanceChange {
    Withdrawal(Withdrawal),
    TradeVaultBalanceChange(TradeVaultBalanceChange),
    Deposit(Deposit),
    #[cynic(fallback)]
    Unknown,
}

#[derive(cynic::QueryFragment, Debug, Clone, Serialize)]
#[typeshare]
pub struct Deposit {
    pub id: Bytes,
    pub __typename: String,
    pub amount: BigInt,
    pub new_vault_balance: BigInt,
    pub old_vault_balance: BigInt,
    pub vault: VaultBalanceChangeVault,
    pub timestamp: BigInt,
    pub transaction: Transaction,
    pub orderbook: Orderbook,
}

#[derive(cynic::QueryFragment, Debug, Clone, Serialize)]
#[typeshare]
pub struct Withdrawal {
    pub id: Bytes,
    pub __typename: String,
    pub amount: BigInt,
    pub new_vault_balance: BigInt,
    pub old_vault_balance: BigInt,
    pub vault: VaultBalanceChangeVault,
    pub timestamp: BigInt,
    pub transaction: Transaction,
    pub orderbook: Orderbook,
}

#[derive(cynic::QueryFragment, Debug, Clone, Serialize)]
#[typeshare]
pub struct TradeVaultBalanceChange {
    pub id: Bytes,
    pub __typename: String,
    pub amount: BigInt,
    pub new_vault_balance: BigInt,
    pub old_vault_balance: BigInt,
    pub vault: VaultBalanceChangeVault,
    pub timestamp: BigInt,
    pub transaction: Transaction,
    pub orderbook: Orderbook,
}

#[derive(cynic::QueryFragment, Debug, Clone, Serialize)]
#[typeshare]
pub struct TradeEvent {
    pub transaction: Transaction,
    pub sender: Bytes,
}

#[derive(cynic::QueryFragment, Debug, Clone, Serialize)]
#[typeshare]
pub struct Trade {
    pub id: Bytes,
    pub trade_event: TradeEvent,
    pub output_vault_balance_change: TradeVaultBalanceChange,
    pub order: Order,
    pub input_vault_balance_change: TradeVaultBalanceChange,
    pub timestamp: BigInt,
    pub orderbook: Orderbook,
}

#[derive(cynic::QueryFragment, Debug, Serialize, Clone)]
#[cynic(graphql_type = "ERC20")]
#[typeshare]
pub struct Erc20 {
    pub id: Bytes,
    pub address: Bytes,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub decimals: Option<BigInt>,
}

#[derive(cynic::QueryFragment, Debug, Serialize, Clone)]
#[typeshare]
pub struct Transaction {
    pub id: Bytes,
    pub from: Bytes,
    pub block_number: BigInt,
    pub timestamp: BigInt,
}

#[derive(cynic::QueryFragment, Debug, Serialize, Clone)]
#[typeshare]
pub struct AddOrder {
    pub transaction: Transaction,
}

#[derive(cynic::Scalar, Debug, Clone)]
#[typeshare]
pub struct BigInt(pub String);

#[derive(cynic::Scalar, Debug, Clone)]
#[typeshare]
pub struct Bytes(pub String);

#[derive(cynic::Enum, Clone, Copy, Debug)]
#[typeshare]
pub enum OrderDirection {
    #[cynic(rename = "asc")]
    Asc,
    #[cynic(rename = "desc")]
    Desc,
}

#[derive(cynic::Enum, Clone, Copy, Debug)]
#[cynic(graphql_type = "Order_orderBy")]
#[typeshare]
pub enum OrderOrderBy {
    #[cynic(rename = "id")]
    Id,
    #[cynic(rename = "orderbook")]
    Orderbook,
    #[cynic(rename = "orderbook__id")]
    OrderbookId,
    #[cynic(rename = "active")]
    Active,
    #[cynic(rename = "orderHash")]
    OrderHash,
    #[cynic(rename = "owner")]
    Owner,
    #[cynic(rename = "inputs")]
    Inputs,
    #[cynic(rename = "outputs")]
    Outputs,
    #[cynic(rename = "nonce")]
    Nonce,
    #[cynic(rename = "orderBytes")]
    OrderBytes,
    #[cynic(rename = "addEvents")]
    AddEvents,
    #[cynic(rename = "removeEvents")]
    RemoveEvents,
    #[cynic(rename = "trades")]
    Trades,
    #[cynic(rename = "meta")]
    Meta,
    #[cynic(rename = "timestampAdded")]
    TimestampAdded,
}

#[derive(cynic::Enum, Clone, Copy, Debug)]
#[cynic(graphql_type = "AddOrder_orderBy")]
#[typeshare]
pub enum AddOrderOrderBy {
    #[cynic(rename = "id")]
    Id,
    #[cynic(rename = "order")]
    Order,
    #[cynic(rename = "order__id")]
    OrderId,
    #[cynic(rename = "order__active")]
    OrderActive,
    #[cynic(rename = "order__orderHash")]
    OrderOrderHash,
    #[cynic(rename = "order__owner")]
    OrderOwner,
    #[cynic(rename = "order__nonce")]
    OrderNonce,
    #[cynic(rename = "order__orderBytes")]
    OrderOrderBytes,
    #[cynic(rename = "order__meta")]
    OrderMeta,
    #[cynic(rename = "order__timestampAdded")]
    OrderTimestampAdded,
    #[cynic(rename = "orderbook")]
    Orderbook,
    #[cynic(rename = "orderbook__id")]
    OrderbookId,
    #[cynic(rename = "transaction")]
    Transaction,
    #[cynic(rename = "transaction__id")]
    TransactionId,
    #[cynic(rename = "transaction__timestamp")]
    TransactionTimestamp,
    #[cynic(rename = "transaction__blockNumber")]
    TransactionBlockNumber,
    #[cynic(rename = "transaction__from")]
    TransactionFrom,
    #[cynic(rename = "sender")]
    Sender,
}

#[derive(cynic::Enum, Clone, Copy, Debug)]
#[cynic(graphql_type = "Trade_orderBy")]
#[typeshare]
pub enum TradeOrderBy {
    #[cynic(rename = "id")]
    Id,
    #[cynic(rename = "orderbook")]
    Orderbook,
    #[cynic(rename = "orderbook__id")]
    OrderbookId,
    #[cynic(rename = "order")]
    Order,
    #[cynic(rename = "order__id")]
    OrderId,
    #[cynic(rename = "order__active")]
    OrderActive,
    #[cynic(rename = "order__orderHash")]
    OrderOrderHash,
    #[cynic(rename = "order__owner")]
    OrderOwner,
    #[cynic(rename = "order__nonce")]
    OrderNonce,
    #[cynic(rename = "order__orderBytes")]
    OrderOrderBytes,
    #[cynic(rename = "order__meta")]
    OrderMeta,
    #[cynic(rename = "order__timestampAdded")]
    OrderTimestampAdded,
    #[cynic(rename = "inputVaultBalanceChange")]
    InputVaultBalanceChange,
    #[cynic(rename = "inputVaultBalanceChange__id")]
    InputVaultBalanceChangeId,
    #[cynic(rename = "inputVaultBalanceChange__amount")]
    InputVaultBalanceChangeAmount,
    #[cynic(rename = "inputVaultBalanceChange__oldVaultBalance")]
    InputVaultBalanceChangeOldVaultBalance,
    #[cynic(rename = "inputVaultBalanceChange__newVaultBalance")]
    InputVaultBalanceChangeNewVaultBalance,
    #[cynic(rename = "inputVaultBalanceChange__timestamp")]
    InputVaultBalanceChangeTimestamp,
    #[cynic(rename = "outputVaultBalanceChange")]
    OutputVaultBalanceChange,
    #[cynic(rename = "outputVaultBalanceChange__id")]
    OutputVaultBalanceChangeId,
    #[cynic(rename = "outputVaultBalanceChange__amount")]
    OutputVaultBalanceChangeAmount,
    #[cynic(rename = "outputVaultBalanceChange__oldVaultBalance")]
    OutputVaultBalanceChangeOldVaultBalance,
    #[cynic(rename = "outputVaultBalanceChange__newVaultBalance")]
    OutputVaultBalanceChangeNewVaultBalance,
    #[cynic(rename = "outputVaultBalanceChange__timestamp")]
    OutputVaultBalanceChangeTimestamp,
    #[cynic(rename = "tradeEvent")]
    TradeEvent,
    #[cynic(rename = "tradeEvent__id")]
    TradeEventId,
    #[cynic(rename = "tradeEvent__sender")]
    TradeEventSender,
    #[cynic(rename = "timestamp")]
    Timestamp,
}

#[derive(cynic::Enum, Clone, Copy, Debug)]
#[cynic(graphql_type = "Vault_orderBy")]
#[typeshare]
pub enum VaultOrderBy {
    #[cynic(rename = "id")]
    Id,
    #[cynic(rename = "orderbook")]
    Orderbook,
    #[cynic(rename = "orderbook__id")]
    OrderbookId,
    #[cynic(rename = "token")]
    Token,
    #[cynic(rename = "token__id")]
    TokenId,
    #[cynic(rename = "token__address")]
    TokenAddress,
    #[cynic(rename = "token__name")]
    TokenName,
    #[cynic(rename = "token__symbol")]
    TokenSymbol,
    #[cynic(rename = "token__decimals")]
    TokenDecimals,
    #[cynic(rename = "owner")]
    Owner,
    #[cynic(rename = "vaultId")]
    VaultId,
    #[cynic(rename = "ordersAsInput")]
    OrdersAsInput,
    #[cynic(rename = "ordersAsOutput")]
    OrdersAsOutput,
    #[cynic(rename = "balance")]
    Balance,
    #[cynic(rename = "balanceChanges")]
    BalanceChanges,
}
