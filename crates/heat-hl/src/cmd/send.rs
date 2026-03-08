use clap::Args;
use heat_core::ctx::Ctx;
use heat_core::error::HeatError;
use heat_core::output::OutputFormat;
use heat_core::safety::DryRunPreview;
use hypersdk::hypercore::types::{AssetTarget, SendAsset, SendToken, UsdSend};
use rust_decimal::Decimal;
use serde::Serialize;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use super::client_from_ctx;
use crate::signer;

#[derive(Args)]
pub struct SendArgs {
    /// Destination address
    pub destination: String,
    /// Amount to send
    pub amount: String,
    /// Token to send (default: USDC)
    #[arg(long, default_value = "USDC")]
    pub token: String,
}

#[derive(Serialize)]
struct SendResult {
    destination: String,
    amount: String,
    token: String,
    status: String,
}

pub async fn run(args: SendArgs, ctx: &Ctx) -> Result<(), HeatError> {
    let client = client_from_ctx(ctx)?;
    let amount = Decimal::from_str(&args.amount).map_err(|_| {
        HeatError::validation("invalid_amount", format!("Invalid amount: {}", args.amount))
    })?;
    if amount <= Decimal::ZERO {
        return Err(HeatError::validation(
            "non_positive_amount",
            "Amount must be positive",
        ));
    }

    let destination: alloy::primitives::Address = args.destination.parse().map_err(|_| {
        HeatError::validation(
            "invalid_address",
            format!("Invalid destination address: {}", args.destination),
        )
    })?;

    if ctx.dry_run {
        DryRunPreview::new("hl", "send")
            .param("destination", &format!("{destination}"))
            .param("amount", &amount.to_string())
            .param("token", &args.token)
            .display();
        return Ok(());
    }

    ctx.confirm_dangerous(&format!("send {} {} to {destination}", amount, args.token))?;

    let s = signer::resolve_signer(ctx)?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    // USDC send is a special case with its own endpoint
    if args.token.eq_ignore_ascii_case("USDC") {
        let send = UsdSend {
            destination,
            amount,
            time: nonce,
        };
        client
            .send_usdc(&s, send, nonce)
            .await
            .map_err(|e| HeatError::protocol("send_failed", format!("Send failed: {e}")))?;
    } else {
        // For non-USDC tokens, use send_asset
        let tokens = client.spot_tokens().await.map_err(|e| {
            HeatError::network("tokens_fetch", format!("Failed to fetch tokens: {e}"))
        })?;
        let token = tokens
            .iter()
            .find(|t| t.name.eq_ignore_ascii_case(&args.token))
            .ok_or_else(|| {
                HeatError::validation(
                    "token_not_found",
                    format!("Token not found: {}", args.token),
                )
            })?;

        let send = SendAsset {
            destination,
            source_dex: AssetTarget::Spot,
            destination_dex: AssetTarget::Spot,
            token: SendToken(token.clone()),
            amount,
            from_sub_account: String::new(),
            nonce,
        };
        client
            .send_asset(&s, send, nonce)
            .await
            .map_err(|e| HeatError::protocol("send_failed", format!("Send failed: {e}")))?;
    }

    let result = SendResult {
        destination: format!("{destination}"),
        amount: amount.to_string(),
        token: args.token.clone(),
        status: "ok".to_string(),
    };

    match ctx.output.format {
        OutputFormat::Json | OutputFormat::Ndjson => {
            ctx.output.write_data(&result, None).map_err(io_err)?;
        }
        OutputFormat::Pretty => {
            println!("Sent {} {} to {destination}", amount, args.token);
        }
        OutputFormat::Quiet => {}
    }
    Ok(())
}

fn io_err(e: std::io::Error) -> HeatError {
    HeatError::internal("output", format!("Write failed: {e}"))
}
