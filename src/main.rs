use bitcoin::Network;

use payday_core::error::PaydayResult;
use payday_core::node::node_api::NodeApi;
use payday_node_lnd::lnd::LndRpc;

#[tokio::main]
async fn main() -> PaydayResult<()> {
    let address = "https://localhost:10009".to_string();
    let cert_file = "/home/protom/dev/btc/payday_rs/tls.cert".to_string();
    let macaroon_file = "/home/protom/dev/btc/payday_rs/admin.macaroon".to_string();

    let mut lnd = LndRpc::new(address, cert_file, macaroon_file, Network::Signet).await?;

    let address = lnd.new_address().await?;
    println!("{:?}", address);

    let balance = lnd.get_balance().await?;
    println!("{:?}", balance);

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
