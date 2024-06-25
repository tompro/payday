use bitcoin::{Amount, Network};
use futures::stream::StreamExt;

use payday_core::node::node_api::NodeApi;
use payday_core::PaydayResult;
use payday_node_lnd::lnd::LndRpc;

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> PaydayResult<()> {
    let address = "https://localhost:10009".to_string();
    let cert_file = "/home/protom/dev/btc/payday_rs/tls.cert".to_string();
    let macaroon_file = "/home/protom/dev/btc/payday_rs/admin.macaroon".to_string();

    let mut lnd = LndRpc::new(address, cert_file, macaroon_file, Network::Signet).await?;

    let address = lnd.new_address().await?;
    println!("{:?}", address);

    let balance = lnd.get_balance().await?;
    println!("{:?}", balance);

    let send_coins = lnd
        .send_coins(
            Amount::from_sat(250000),
            "tb1pwrwjsyhgurspa7k7eqlvkphxllqh4yvz2w37hzcv0rpfnq749j2svganhr".to_string(),
            Amount::from_sat(1),
        )
        .await?;
    println!("{:?}", send_coins);

    // let missed = lnd.subscribe_missed_transactions(1190000).await?;
    // tokio::pin!(missed);
    // // while let Some(event) = missed.next().await {
    // //     println!("Subscription: {:?}", event);
    // // }
    // // let mut all = missed;
    // while let Some(event) = missed.next().await {
    //     println!("All: {:?}", event);
    // }
    let subscription = lnd.subscribe_onchain_transactions(1190000).await?;
    tokio::pin!(subscription);
    while let Some(event) = subscription.next().await {
        println!("Subscription: {:?}", event);
    }

    // while let Some(event) = subscription.next().await {
    //     println!("Subscription: {:?}", event);
    // }

    // tokio::pin!(all);
    // while let Some(event) = all.next().await {
    //     println!("All: {:?}", event);
    // }

    // let fee_estimates = client
    //     .lightning()
    //     .estimate_fee(fedimint_tonic_lnd::lnrpc::EstimateFeeRequest {
    //         target_conf: 1,
    //         addr_to_amount: HashMap::from([(
    //             "tb1pwrwjsyhgurspa7k7eqlvkphxllqh4yvz2w37hzcv0rpfnq749j2svganhr".to_string(),
    //             250000i64,
    //         )]),
    //         ..Default::default()
    //     })
    //     .await
    //     .unwrap();
    // println!("{:?}", fee_estimates);

    // let send_coins = client.lightning().send_coins(
    //     fedimint_tonic_lnd::lnrpc::SendCoinsRequest {
    //         addr: "tb1pu6cmt6tvdnw44nrm5ddfcl8glrjllrsytmwm66fu0zlfglcdsths6rhpld".to_string(),
    //         amount: 250000,
    //         sat_per_vbyte: 1,
    //         ..Default::default()
    //     }
    // ).await.unwrap();
    // println!("{:?}", send_coins);

    // let mut subscription: Response<Streaming<Transaction>> = client
    //     .lightning()
    //     .subscribe_transactions(fedimint_tonic_lnd::lnrpc::GetTransactionsRequest {
    //         start_height: 1190000,
    //         ..Default::default()
    //     })
    //     .await
    //     .unwrap();
    // while let Some(tx) = subscription.get_mut().next().await {
    //     println!("{:?}", tx);
    // }

    Ok(())
}
