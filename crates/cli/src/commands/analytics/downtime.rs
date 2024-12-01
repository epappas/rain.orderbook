use crate::execute::Execute;
use anyhow::{anyhow, Result};
use chrono::NaiveDate;
use clap::Args;
use humantime::Duration;
use rain_orderbook_analytics::Analytics as OrderbookAnalytics;
use rain_orderbook_subgraph_client::OrderbookSubgraphClient;
use reqwest::Url;

#[derive(Args, Clone)]
pub struct DowntimeArgs {
    #[clap(long)]
    start: Option<String>,
    #[clap(long)]
    end: Option<String>,
    #[clap(long, required = true)]
    subgraph_url: String,
    #[clap(
        long,
        default_value = "10m",
        help = "Minimum time between trades to consider (e.g. 30s, 5m, 2h, 1d)"
    )]
    threshold: Duration,
}

impl DowntimeArgs {
    fn parse_date(date_str: &str) -> Result<u64> {
        let date = NaiveDate::parse_from_str(date_str, "%d-%m-%Y")
            .map_err(|e| anyhow!("Invalid date '{}': {}", date_str, e))?;
        let datetime = date.and_hms_opt(0, 0, 0).unwrap();
        Ok(datetime.and_utc().timestamp() as u64)
    }
}

impl Execute for DowntimeArgs {
    async fn execute(&self) -> Result<()> {
        let start_timestamp = match &self.start {
            Some(start_str) => Some(Self::parse_date(start_str)?),
            None => None,
        };

        let end_timestamp = match &self.end {
            Some(end_str) => Some(Self::parse_date(end_str)?),
            None => None,
        };

        let period = match (start_timestamp, end_timestamp) {
            (Some(start), Some(end)) => Some((start, end)),
            _ => None,
        };

        let client = OrderbookSubgraphClient::new(Url::parse(&self.subgraph_url)?);
        let analytics = OrderbookAnalytics::new(client);

        let threshold_secs = self.threshold.as_secs();
        let (avg, min, max, count, total) = analytics
            .calculate_downtime_between_trades(period, threshold_secs)
            .await;

        println!("Average downtime: {:.2} seconds", avg);
        println!("Minimum downtime: {:.2} seconds", min);
        println!("Maximum downtime: {:.2} seconds", max);
        println!("Number of occurrences: {}", count);
        println!("Total downtime: {:.2} seconds", total);

        Ok(())
    }
}
