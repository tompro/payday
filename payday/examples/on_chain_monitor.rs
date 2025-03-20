use std::sync::Arc;

use bitcoin::Network;
use payday::{on_chain_processor::OnChainEventProcessor, on_chain_service::OnChainService};
use payday_core::aggregate::on_chain_aggregate::OnChainInvoice;
use payday_node_lnd::lnd::{LndConfig, LndPaymentEventStream};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    info!("Starting on-chain monitor");

    // db connection
    let pool = payday_postgres::create_postgres_pool(
        "postgres://postgres:password@localhost:5432/default",
    )
    .await
    .expect("DB connection");

    // processes on chain transaction commands
    let aggregate = payday_postgres::create_cqrs::<OnChainInvoice>(pool.clone(), vec![], ())
        .await
        .expect("Aggregate instance");

    // contains the event handler
    let service = OnChainService::new(aggregate);

    let stream = Arc::new(LndPaymentEventStream::new(LndConfig::CertPath {
        node_id: "node1".to_string(),
        address: "https://localhost:10008".to_string(),
        cert_path: "tls.cert".to_string(),
        macaroon_file: "admin.macaroon".to_string(),
        network: Network::Signet,
    }));
    let stream2 = Arc::new(LndPaymentEventStream::new(LndConfig::RootCert {
        node_id: "node2".to_string(),
        address: "https://localhost:10009".to_string(),
        macaroon: "macaroon".to_string(),
        network: Network::Signet,
    }));

    // consumes on chain events from all nodes
    let on_chain_processor =
        OnChainEventProcessor::new(pool, vec![stream, stream2], Arc::new(service));

    // start the event monitor
    let all = on_chain_processor.start().await;
    all.join_all().await;

    info!("On-chain monitor finished");
    Ok(())
}
