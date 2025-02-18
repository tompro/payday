use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use bitcoin::{Address, Amount, Network};

use fedimint_tonic_lnd::{
    lnrpc::{GetTransactionsRequest, Transaction},
    Client,
};
use payday_btc::to_address;
use payday_core::{
    api::on_chain_api::{
        GetOnChainBalanceApi, OnChainBalance, OnChainInvoiceApi, OnChainPaymentApi,
        OnChainPaymentResult, OnChainStreamApi, OnChainTransaction, OnChainTransactionApi,
        OnChainTransactionEvent, OnChainTransactionEventProcessorApi,
    },
    Result,
};
use tokio::{sync::Mutex, task::JoinHandle};
use tokio_stream::StreamExt;

use crate::wrapper::LndRpcWrapper;

pub struct Lnd {
    config: LndConfig,
    client: LndRpcWrapper,
}

impl Lnd {
    pub async fn new(config: LndConfig) -> Result<Self> {
        let client = LndRpcWrapper::new(config.clone()).await?;
        Ok(Self { config, client })
    }
}

#[async_trait]
impl GetOnChainBalanceApi for Lnd {
    async fn get_onchain_balance(&self) -> Result<OnChainBalance> {
        let res = self.client.get_onchain_balance().await?;
        Ok(OnChainBalance {
            total_balance: to_amount(res.total_balance),
            unconfirmed_balance: to_amount(res.unconfirmed_balance),
            confirmed_balance: to_amount(res.confirmed_balance),
        })
    }
}

#[async_trait]
impl OnChainInvoiceApi for Lnd {
    async fn new_address(&self) -> Result<Address> {
        self.client.new_address().await
    }
}

#[async_trait]
impl OnChainPaymentApi for Lnd {
    fn validate_address(&self, address: &str) -> Result<Address> {
        to_address(address, self.config.network)
    }

    async fn estimate_fee(
        &self,
        target_conf: i32,
        outputs: HashMap<String, Amount>,
    ) -> Result<Amount> {
        let out = outputs
            .iter()
            .map(|p| (p.0.to_owned(), p.1.to_sat() as i64))
            .collect();
        let fee = self.client.estimate_fee(target_conf, out).await?;
        Ok(fee)
    }

    async fn send(
        &self,
        amount: Amount,
        address: String,
        sats_per_vbyte: Amount,
    ) -> Result<OnChainPaymentResult> {
        let tx_id = self
            .client
            .send_coins(amount, &address, sats_per_vbyte)
            .await?;

        Ok(OnChainPaymentResult {
            tx_id,
            amounts: HashMap::from([(address.to_owned(), amount.to_owned())]),
            fee: sats_per_vbyte,
        })
    }

    async fn batch_send(
        &self,
        outputs: HashMap<String, Amount>,
        sats_per_vbyte: Amount,
    ) -> Result<OnChainPaymentResult> {
        let out = outputs
            .iter()
            .flat_map(|(k, v)| {
                to_address(k, self.config.network)
                    .ok()
                    .map(|a| (a, v.to_sat() as i64))
            })
            .collect();
        let tx_id = self.client.batch_send(out, sats_per_vbyte).await?;
        Ok(OnChainPaymentResult {
            tx_id,
            amounts: outputs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_owned()))
                .collect(),
            fee: sats_per_vbyte,
        })
    }
}

#[async_trait]
impl OnChainTransactionApi for Lnd {
    async fn get_onchain_transactions(
        &self,
        start_height: i32,
        end_height: i32,
    ) -> Result<Vec<OnChainTransactionEvent>> {
        let result = self
            .client
            .get_transactions(start_height, end_height)
            .await?
            .iter()
            .flat_map(|tx| to_on_chain_events(tx, self.config.network))
            .flatten()
            .collect();
        Ok(result)
    }
}

#[derive(Debug, Clone)]
pub struct LndConfig {
    pub name: String,
    pub address: String,
    pub cert_path: String,
    pub macaroon_file: String,
    pub network: Network,
}

/// Converts a satoshi amount to an Amount
fn to_amount(sats: i64) -> Amount {
    if sats < 0 {
        Amount::ZERO
    } else {
        Amount::from_sat(sats.unsigned_abs())
    }
}

/// Converts a Transaction to a list of OnChainTransactionEvents.
fn to_on_chain_events(tx: &Transaction, chain: Network) -> Result<Vec<OnChainTransactionEvent>> {
    let received = tx.amount > 0;
    let confirmed = tx.num_confirmations > 0;

    let res = tx
        .output_details
        .iter()
        .filter(|d| {
            if received {
                d.is_our_address
            } else {
                !d.is_our_address
            }
        })
        .flat_map(|d| {
            let address = to_address(&d.address, chain);
            if let Ok(address) = address {
                let payload = OnChainTransaction {
                    tx_id: tx.tx_hash.to_owned(),
                    block_height: tx.block_height,
                    confirmations: tx.num_confirmations,
                    amount: Amount::from_sat(tx.amount.unsigned_abs()),
                    address,
                };

                match (confirmed, received) {
                    (true, true) => Some(OnChainTransactionEvent::ReceivedConfirmed(payload)),
                    (true, false) => Some(OnChainTransactionEvent::SentConfirmed(payload)),
                    (false, true) => Some(OnChainTransactionEvent::ReceivedUnconfirmed(payload)),
                    (false, false) => Some(OnChainTransactionEvent::SentUnconfirmed(payload)),
                }
            } else {
                None
            }
        })
        .collect();
    Ok(res)
}

pub struct LndTransactionStream {
    config: LndConfig,
    handler: Arc<Mutex<dyn OnChainTransactionEventProcessorApi>>,
    start_height: Option<i32>,
}

impl LndTransactionStream {
    pub fn new(
        config: LndConfig,
        handler: Arc<Mutex<dyn OnChainTransactionEventProcessorApi>>,
        start_height: Option<i32>,
    ) -> Self {
        Self {
            config,
            handler,
            start_height,
        }
    }

    /// does fetch potential missing events from the current start_height
    async fn start_subscription(&self) -> Result<Vec<OnChainTransactionEvent>> {
        let lnd = Lnd::new(self.config.clone()).await?;
        let start_height = match self.start_height {
            Some(start_height) => start_height,
            None => self.handler.lock().await.get_block_height().await?,
        };

        let events = lnd.get_onchain_transactions(start_height, -1).await?;
        Ok(events)
    }
}

#[async_trait]
impl OnChainStreamApi for LndTransactionStream {
    async fn process_events(&self) -> Result<JoinHandle<()>> {
        let start_events = self.start_subscription().await.ok().unwrap_or(vec![]);
        for event in start_events {
            self.handler.lock().await.process_event(event).await?;
        }
        let service = self.handler.clone();
        let config = self.config.clone();

        let handle = tokio::spawn(async move {
            let mut lnd: Client = fedimint_tonic_lnd::connect(
                config.address.to_string(),
                config.cert_path.to_string(),
                config.macaroon_file.to_string(),
            )
            .await
            .expect("Failed to connect to LND on-chain transaction stream");

            let mut stream = lnd
                .lightning()
                .subscribe_transactions(GetTransactionsRequest::default())
                .await
                .expect("Failed to subscribe to LND on-chain transaction events")
                .into_inner()
                .filter(|tx| tx.is_ok())
                .map(|tx| tx.unwrap());

            while let Some(event) = stream.next().await {
                let events = to_on_chain_events(&event, config.network)
                    .expect("Failed to parse LND on-chain transaction");

                for event in events {
                    service
                        .lock()
                        .await
                        .process_event(event)
                        .await
                        .expect("Failed to process LND on chain transaction event");
                }
            }
        });

        Ok(handle)
    }
}

//pub struct LndOnChainPaymentEventStream {
//    config: LndConfig,
//}
//
//impl LndOnChainPaymentEventStream {
//    pub fn new(config: LndConfig) -> Self {
//        Self { config }
//    }
//}
//
//impl OnChainTransactionStreamSubscriber for LndOnChainPaymentEventStream {
//    fn subscribe_events(&self) -> Result<Receiver<OnChainTransactionEvent>> {
//        let config = self.config.clone();
//        let (tx, rx) = tokio::sync::mpsc::channel::<OnChainTransactionEvent>(100);
//
//        tokio::spawn(async move {
//            let sender = tx.clone();
//            let mut lnd: Client = fedimint_tonic_lnd::connect(
//                config.address.to_string(),
//                config.cert_path.to_string(),
//                config.macaroon_file.to_string(),
//            )
//            .await
//            .expect("Failed to connect to LND on-chain transaction stream");
//
//            let mut stream = lnd
//                .lightning()
//                .subscribe_transactions(GetTransactionsRequest::default())
//                .await
//                .expect("Failed to subscribe to LND on-chain transaction events")
//                .into_inner()
//                .filter(|tx| tx.is_ok())
//                .map(|tx| tx.unwrap());
//
//            while let Some(event) = stream.next().await {
//                let events = to_on_chain_events(&event, config.network)
//                    .expect("Failed to parse LND on-chain transaction");
//
//                for event in events {
//                    sender.send(event).await.expect("stream closed");
//                }
//            }
//        });
//        Ok(rx)
//    }
//}
