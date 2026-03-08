//! CTF commands — conditional token split, merge, redeem, ID queries.

use clap::Subcommand;
use heat_core::ctx::Ctx;
use heat_core::error::HeatError;
use heat_core::safety::DryRunPreview;
use polymarket_client_sdk::ctf;
use polymarket_client_sdk::ctf::types::{
    CollectionIdRequest, ConditionIdRequest, MergePositionsRequest, PositionIdRequest,
    RedeemNegRiskRequest, RedeemPositionsRequest, SplitPositionRequest,
};
use polymarket_client_sdk::POLYGON;
use serde::Serialize;

use crate::auth;

fn io_err(e: std::io::Error) -> HeatError {
    HeatError::internal("output", format!("Write failed: {e}"))
}

fn ctf_err(e: impl std::fmt::Display) -> HeatError {
    HeatError::protocol("ctf_error", format!("CTF error: {e}"))
}

fn parse_u256(s: &str) -> Result<alloy::primitives::U256, HeatError> {
    s.parse().map_err(|_| {
        HeatError::validation("invalid_u256", format!("Invalid U256 value: {s}"))
    })
}

fn parse_b256(s: &str) -> Result<alloy::primitives::B256, HeatError> {
    s.parse().map_err(|_| {
        HeatError::validation("invalid_b256", format!("Invalid B256 value: {s}"))
    })
}

fn parse_address(s: &str) -> Result<alloy::primitives::Address, HeatError> {
    s.parse().map_err(|_| {
        HeatError::validation("invalid_address", format!("Invalid address: {s}"))
    })
}

fn write_json(ctx: &Ctx, val: serde_json::Value) -> Result<(), HeatError> {
    ctx.output.write_data(&val, None).map_err(io_err)
}

#[derive(Serialize)]
struct TxResult {
    transaction_hash: String,
    block_number: u64,
}

#[derive(Subcommand)]
pub enum CtfSubcommand {
    /// Split position into conditional tokens
    Split {
        /// Collateral token address
        #[arg(long)]
        collateral: String,
        /// Condition ID
        #[arg(long)]
        condition_id: String,
        /// Partition (comma-separated index sets)
        #[arg(long)]
        partition: String,
        /// Amount of collateral to split
        #[arg(long)]
        amount: String,
    },
    /// Merge conditional tokens back into collateral
    Merge {
        /// Collateral token address
        #[arg(long)]
        collateral: String,
        /// Condition ID
        #[arg(long)]
        condition_id: String,
        /// Partition (comma-separated index sets)
        #[arg(long)]
        partition: String,
        /// Amount to merge
        #[arg(long)]
        amount: String,
    },
    /// Redeem resolved positions
    Redeem {
        /// Collateral token address
        #[arg(long)]
        collateral: String,
        /// Condition ID
        #[arg(long)]
        condition_id: String,
        /// Index sets (comma-separated)
        #[arg(long)]
        index_sets: String,
    },
    /// Redeem neg-risk positions
    RedeemNegRisk {
        /// Condition ID
        #[arg(long)]
        condition_id: String,
        /// Amounts (comma-separated)
        #[arg(long)]
        amounts: String,
    },
    /// Compute condition ID
    ConditionId {
        /// Oracle address
        #[arg(long)]
        oracle: String,
        /// Question ID
        #[arg(long)]
        question_id: String,
        /// Number of outcome slots
        #[arg(long, default_value_t = 2)]
        outcome_slots: u64,
    },
    /// Compute collection ID
    CollectionId {
        /// Condition ID
        #[arg(long)]
        condition_id: String,
        /// Index set
        #[arg(long)]
        index_set: u64,
        /// Parent collection ID
        #[arg(long)]
        parent: Option<String>,
    },
    /// Compute position ID
    PositionId {
        /// Collateral token address
        #[arg(long)]
        collateral: String,
        /// Collection ID
        #[arg(long)]
        collection_id: String,
    },
}

pub async fn run(sub: CtfSubcommand, ctx: &Ctx) -> Result<(), HeatError> {
    match sub {
        CtfSubcommand::Split { collateral, condition_id, partition, amount } => {
            let amt = parse_u256(&amount)?;
            let coll = parse_address(&collateral)?;
            let cond = parse_b256(&condition_id)?;
            let parts: Result<Vec<alloy::primitives::U256>, _> =
                partition.split(',').map(|s| parse_u256(s.trim())).collect();
            let parts = parts?;

            if ctx.dry_run {
                DryRunPreview::new("pm", "ctf split")
                    .param("collateral", &collateral)
                    .param("condition_id", &condition_id)
                    .param("amount", &amount)
                    .display();
                return Ok(());
            }
            ctx.confirm_dangerous(&format!("split {amount} collateral on {condition_id}"))?;

            let provider = auth::wallet_provider(ctx).await?;
            let client = ctf::Client::new(provider, POLYGON).map_err(ctf_err)?;
            let req = SplitPositionRequest::builder()
                .collateral_token(coll)
                .condition_id(cond)
                .partition(parts)
                .amount(amt)
                .build();
            let result = client.split_position(&req).await.map_err(ctf_err)?;
            let info = TxResult {
                transaction_hash: format!("{}", result.transaction_hash),
                block_number: result.block_number,
            };
            ctx.output.write_data(&info, None).map_err(io_err)
        }
        CtfSubcommand::Merge { collateral, condition_id, partition, amount } => {
            let amt = parse_u256(&amount)?;
            let coll = parse_address(&collateral)?;
            let cond = parse_b256(&condition_id)?;
            let parts: Result<Vec<alloy::primitives::U256>, _> =
                partition.split(',').map(|s| parse_u256(s.trim())).collect();
            let parts = parts?;

            if ctx.dry_run {
                DryRunPreview::new("pm", "ctf merge")
                    .param("collateral", &collateral)
                    .param("condition_id", &condition_id)
                    .param("amount", &amount)
                    .display();
                return Ok(());
            }
            ctx.confirm_dangerous(&format!("merge {amount} tokens on {condition_id}"))?;

            let provider = auth::wallet_provider(ctx).await?;
            let client = ctf::Client::new(provider, POLYGON).map_err(ctf_err)?;
            let req = MergePositionsRequest::builder()
                .collateral_token(coll)
                .condition_id(cond)
                .partition(parts)
                .amount(amt)
                .build();
            let result = client.merge_positions(&req).await.map_err(ctf_err)?;
            let info = TxResult {
                transaction_hash: format!("{}", result.transaction_hash),
                block_number: result.block_number,
            };
            ctx.output.write_data(&info, None).map_err(io_err)
        }
        CtfSubcommand::Redeem { collateral, condition_id, index_sets } => {
            let coll = parse_address(&collateral)?;
            let cond = parse_b256(&condition_id)?;
            let sets: Result<Vec<alloy::primitives::U256>, _> =
                index_sets.split(',').map(|s| parse_u256(s.trim())).collect();
            let sets = sets?;

            if ctx.dry_run {
                DryRunPreview::new("pm", "ctf redeem")
                    .param("condition_id", &condition_id)
                    .display();
                return Ok(());
            }
            ctx.confirm_dangerous(&format!("redeem positions on {condition_id}"))?;

            let provider = auth::wallet_provider(ctx).await?;
            let client = ctf::Client::new(provider, POLYGON).map_err(ctf_err)?;
            let req = RedeemPositionsRequest::builder()
                .collateral_token(coll)
                .condition_id(cond)
                .index_sets(sets)
                .build();
            let result = client.redeem_positions(&req).await.map_err(ctf_err)?;
            let info = TxResult {
                transaction_hash: format!("{}", result.transaction_hash),
                block_number: result.block_number,
            };
            ctx.output.write_data(&info, None).map_err(io_err)
        }
        CtfSubcommand::RedeemNegRisk { condition_id, amounts } => {
            let cond = parse_b256(&condition_id)?;
            let amts: Result<Vec<alloy::primitives::U256>, _> =
                amounts.split(',').map(|s| parse_u256(s.trim())).collect();
            let amts = amts?;

            if ctx.dry_run {
                DryRunPreview::new("pm", "ctf redeem-neg-risk")
                    .param("condition_id", &condition_id)
                    .display();
                return Ok(());
            }
            ctx.confirm_dangerous(&format!("redeem neg-risk on {condition_id}"))?;

            let provider = auth::wallet_provider(ctx).await?;
            let client = ctf::Client::with_neg_risk(provider, POLYGON).map_err(ctf_err)?;
            let req = RedeemNegRiskRequest::builder()
                .condition_id(cond)
                .amounts(amts)
                .build();
            let result = client.redeem_neg_risk(&req).await.map_err(ctf_err)?;
            let info = TxResult {
                transaction_hash: format!("{}", result.transaction_hash),
                block_number: result.block_number,
            };
            ctx.output.write_data(&info, None).map_err(io_err)
        }
        CtfSubcommand::ConditionId { oracle, question_id, outcome_slots } => {
            let oracle = parse_address(&oracle)?;
            let qid = parse_b256(&question_id)?;
            let slots = alloy::primitives::U256::from(outcome_slots);

            let provider = auth::readonly_provider().await?;
            let client = ctf::Client::new(provider, POLYGON).map_err(ctf_err)?;
            let req = ConditionIdRequest::builder()
                .oracle(oracle)
                .question_id(qid)
                .outcome_slot_count(slots)
                .build();
            let result = client.condition_id(&req).await.map_err(ctf_err)?;
            write_json(ctx, serde_json::json!({ "condition_id": format!("{}", result.condition_id) }))
        }
        CtfSubcommand::CollectionId { condition_id, index_set, parent } => {
            let cond = parse_b256(&condition_id)?;
            let parent_id = match &parent {
                Some(p) => parse_b256(p)?,
                None => alloy::primitives::B256::ZERO,
            };

            let provider = auth::readonly_provider().await?;
            let client = ctf::Client::new(provider, POLYGON).map_err(ctf_err)?;
            let req = CollectionIdRequest::builder()
                .condition_id(cond)
                .index_set(alloy::primitives::U256::from(index_set))
                .parent_collection_id(parent_id)
                .build();
            let result = client.collection_id(&req).await.map_err(ctf_err)?;
            write_json(ctx, serde_json::json!({ "collection_id": format!("{}", result.collection_id) }))
        }
        CtfSubcommand::PositionId { collateral, collection_id } => {
            let coll = parse_address(&collateral)?;
            let cid = parse_b256(&collection_id)?;

            let provider = auth::readonly_provider().await?;
            let client = ctf::Client::new(provider, POLYGON).map_err(ctf_err)?;
            let req = PositionIdRequest::builder()
                .collateral_token(coll)
                .collection_id(cid)
                .build();
            let result = client.position_id(&req).await.map_err(ctf_err)?;
            write_json(ctx, serde_json::json!({ "position_id": result.position_id.to_string() }))
        }
    }
}
