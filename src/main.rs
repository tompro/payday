use bitcoin::{Amount, Network};

use payday_core::api::on_chain_api::{GetOnChainBalanceApi, OnChainInvoiceApi};
use payday_core::Result;
use payday_node_lnd::lnd::{Lnd, LndConfig};
use payday_node_lnd::wrapper::LndRpcWrapper;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestPayload {
    name: String,
    processed: bool,
    sequence: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let lnd_config = LndConfig {
        name: "payday".to_string(),
        address: "https://tbc-mutiny.u.voltageapp.io:10009".to_string(),
        cert_path: "tls.cert".to_string(),
        macaroon_file: "admin.macaroon".to_string(),
        network: Network::Signet,
    };
    let lnd = Lnd::new(lnd_config.clone()).await?;
    let wrapper = LndRpcWrapper::new(lnd_config.clone()).await?;

    let invoice = wrapper
        .create_invoice(
            Amount::from_sat(300_000),
            Some("test invoice".to_string()),
            Some(31535000),
        )
        .await?;

    println!("{:?}", invoice);

    let address = lnd.new_address().await?;
    println!("{:?}", address);

    let balance = lnd.get_onchain_balance().await?;
    println!("{:?}", balance);

    // let db = create_surreal_db("ws://localhost:8000", "payday", "payday").await?;
    // let block_height_store = BlockHeightStore::new(db.clone());
    // let processor = OnChainTransactionProcessor::new(
    //     "lnd",
    //     Box::new(block_height_store),
    //     Box::new(OnChainTransactionPrintHandler),
    // );
    // let stream =
    //     LndTransactionStream::new(lnd_config.clone(), Arc::new(Mutex::new(processor)), None);
    // let handle = stream.process_events().await?;

    //let publisher = EventStream::new(db.clone(), "events");
    // let publisher = SurrealTaskQueue::new(db.clone(), "tasks");
    //let publish_handle = publisher.subscribe().await?;
    // tokio::time::sleep(Duration::from_secs(2)).await;

    // let processor = SurrealTaskProcessor::new(
    //     db.clone(),
    //     "tasks",
    //     vec![Arc::new(Mutex::new(PrintTaskHandler))],
    // );
    //
    // let processor_handle = processor.process().await?;
    //
    // let task_payload = Task::new(
    //     "anewone".to_owned(),
    //     TestPayload {
    //         name: "anothe test".to_string(),
    //         processed: true,
    //         sequence: 1,
    //     },
    // );
    //
    // publisher.publish(task_payload).await?;
    //
    // let retry_task = Task::new(
    //     "a retry task".to_owned(),
    //     TestPayload {
    //         name: "retry task".to_string(),
    //         processed: false,
    //         sequence: 2,
    //     },
    // );
    //
    // publisher.publish(retry_task.clone()).await?;
    // publisher
    //     .retry(retry_task, RetryType::Fixed(3, Duration::from_secs(5)))
    //     .await?;

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
    //let (_, _) = tokio::join!(handle, processor_handle);
    //handle.await.expect("could not subscribe to onchain stream");
    //bind.await.expect("done subscriber");
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
