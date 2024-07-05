use std::sync::Arc;

use bitcoin::Network;

use payday_btc::{
    on_chain_api::{OnChainApi, OnChainStreamApi},
    on_chain_processor::OnChainTransactionEventPrinter,
};
use payday_core::{persistence::block_height::BlockHeightStoreApi, PaydayResult};
use payday_node_lnd::lnd::{Lnd, LndConfig, LndTransactionStream};
use payday_postgres::{block_height::BlockHeightStore, create_postgres_pool};
use tokio::sync::Mutex;

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> PaydayResult<()> {
    let lnd_config = LndConfig {
        name: "payday".to_string(),
        address: "https://localhost:10009".to_string(),
        cert_path: "/home/protom/dev/btc/payday_rs/tls.cert".to_string(),
        macaroon_file: "/home/protom/dev/btc/payday_rs/admin.macaroon".to_string(),
        network: Network::Signet,
    };

    let lnd_stream = LndTransactionStream::new(
        lnd_config.clone(),
        Arc::new(Mutex::new(OnChainTransactionEventPrinter)),
    );

    let handle = lnd_stream
        .process_events()
        .expect("Could not process LND on-chain transaction stream");

    let lnd = Lnd::new(lnd_config).await?;

    let address = lnd.new_address().await?;
    println!("{:?}", address);

    let balance = lnd.get_balance().await?;
    println!("{:?}", balance);

    let pool = create_postgres_pool("postgresql://postgres:password@localhost:5432/payday").await?;
    let block_height_store = BlockHeightStore::new(pool);

    //let db = create_surreal_db("rocksdb://./data", "test", "test").await?;
    //let block_height_store = BlockHeightStore::new(db);

    let block_height = block_height_store.get_block_height("lnd").await?;
    println!("{:?}", block_height);
    block_height_store
        .set_block_height("lnd", block_height.block_height + 1)
        .await?;
    let block_height = block_height_store.get_block_height("lnd").await?;
    println!("{:?}", block_height);

    //let outputs = HashMap::from([
    //    (
    //        "tb1p96rerkjw5e5ul4fxatc8xjg0jhu7hy4ue57s7jwgyxj2c6shsxystfrxk4".to_string(),
    //        Amount::from_sat(250_000),
    //    ),
    //    (
    //        "tb1pwrwjsyhgurspa7k7eqlvkphxllqh4yvz2w37hzcv0rpfnq749j2svganhr".to_string(),
    //        Amount::from_sat(250_000),
    //    ),
    //]);

    //let sent_coins = lnd.batch_send(outputs, Amount::from_sat(2)).await?;
    //println!("Sent: {:?}", sent_coins);

    // let send_coins = lnd
    //     .send_coins(
    //         Amount::from_sat(250000),
    //         "tb1pwrwjsyhgurspa7k7eqlvkphxllqh4yvz2w37hzcv0rpfnq749j2svganhr".to_string(),
    //         Amount::from_sat(1),
    //     )
    //     .await?;
    // println!("{:?}", send_coins);

    //let pending = lnd.get_onchain_transactions(1190000, -1).await?;
    //for event in pending {
    //    println!("Pending: {:?}", event);
    //}

    handle.await.expect("could not subscribe to onchain stream");
    println!("Done");

    // let subscription = lnd.subscribe_onchain_transactions(1190000).await?;
    // println!("Subscribing to onchain transactions");
    // tokio::pin!(subscription);
    // while let Some(event) = subscription.next().await {
    //     println!("Subscription: {:?}", event);
    // }

    //let fee_estimates = client
    //    .lightning()
    //    .estimate_fee(fedimint_tonic_lnd::lnrpc::EstimateFeeRequest {
    //        target_conf: 1,
    //        addr_to_amount: HashMap::from([(
    //            "tb1pwrwjsyhgurspa7k7eqlvkphxllqh4yvz2w37hzcv0rpfnq749j2svganhr".to_string(),
    //            250000i64,
    //        )]),
    //        ..Default::default()
    //    })
    //    .await
    //    .unwrap();
    // println!("{:?}", fee_estimates);

    Ok(())
}
