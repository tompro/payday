use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use bitcoin::{Address, Amount, Network};

use fedimint_tonic_lnd::{
    lnrpc::{GetTransactionsRequest, Transaction},
    Client,
};
use payday_btc::{
    on_chain_api::{
        Balance, ChannelBalance, OnChainApi, OnChainBalance, OnChainPaymentResult,
        OnChainStreamApi, OnChainTransactionStreamSubscriber,
    },
    on_chain_processor::{
        OnChainTransaction, OnChainTransactionEvent, OnChainTransactionEventProcessor,
    },
    to_address,
};
use payday_core::{PaydayResult, PaydayStream};
use tokio::{
    sync::{mpsc::Receiver, Mutex},
    task::JoinHandle,
};
use tokio_stream::StreamExt;

use crate::wrapper::LndRpcWrapper;

pub struct Lnd {
    config: LndConfig,
    client: LndRpcWrapper,
}

impl Lnd {
    pub async fn new(config: LndConfig) -> PaydayResult<Self> {
        let client = LndRpcWrapper::new(config.clone()).await?;
        Ok(Self { config, client })
    }
}

#[async_trait]
impl OnChainApi for Lnd {
    async fn get_balance(&self) -> PaydayResult<Balance> {
        let (on_chain, lightning) = self.client.get_balances().await?;
        Ok(Balance {
            onchain: OnChainBalance {
                total_balance: to_amount(on_chain.total_balance),
                unconfirmed_balance: to_amount(on_chain.unconfirmed_balance),
                confirmed_balance: to_amount(on_chain.confirmed_balance),
            },
            channel: ChannelBalance {
                local_balance: Amount::from_sat(lightning.local_balance.map_or(0, |v| v.sat)),
                remote_balance: Amount::from_sat(lightning.remote_balance.map_or(0, |v| v.sat)),
            },
        })
    }

    async fn new_address(&self) -> PaydayResult<Address> {
        self.client.new_address().await
    }

    fn validate_address(&self, address: &str) -> PaydayResult<Address> {
        to_address(address, self.config.network)
    }

    async fn get_onchain_transactions(
        &self,
        start_height: i32,
        end_height: i32,
    ) -> PaydayResult<Vec<OnChainTransactionEvent>> {
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

    async fn estimate_fee(
        &self,
        target_conf: i32,
        outputs: HashMap<String, Amount>,
    ) -> PaydayResult<Amount> {
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
    ) -> PaydayResult<OnChainPaymentResult> {
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
    ) -> PaydayResult<OnChainPaymentResult> {
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
fn to_on_chain_events(
    tx: &Transaction,
    chain: Network,
) -> PaydayResult<Vec<OnChainTransactionEvent>> {
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
                    amount: to_amount(tx.amount),
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
    handler: Arc<Mutex<dyn OnChainTransactionEventProcessor>>,
}

impl LndTransactionStream {
    pub fn new(
        config: LndConfig,
        handler: Arc<Mutex<dyn OnChainTransactionEventProcessor>>,
    ) -> Self {
        Self { config, handler }
    }
}

impl OnChainStreamApi for LndTransactionStream {
    fn process_events(&self) -> PaydayResult<JoinHandle<()>> {
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

pub struct LndOnChainPaymentEventStream {
    config: LndConfig,
}

impl LndOnChainPaymentEventStream {
    pub fn new(config: LndConfig) -> Self {
        Self { config }
    }
}

impl OnChainTransactionStreamSubscriber for LndOnChainPaymentEventStream {
    fn subscribe_events(&self) -> PaydayResult<Receiver<OnChainTransactionEvent>> {
        let config = self.config.clone();
        let (tx, rx) = tokio::sync::mpsc::channel::<OnChainTransactionEvent>(100);

        tokio::spawn(async move {
            let sender = tx.clone();
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
                    sender.send(event).await.expect("stream closed");
                }
            }
        });
        Ok(rx)
    }
}
