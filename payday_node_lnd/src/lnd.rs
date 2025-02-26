use std::collections::HashMap;

use async_trait::async_trait;
use bitcoin::{hex::DisplayHex, Address, Network};
use tracing::error;

use crate::to_address;
use fedimint_tonic_lnd::{
    lnrpc::{GetTransactionsRequest, InvoiceSubscription, Transaction},
    Client,
};
use lightning_invoice::Bolt11Invoice;
use payday_core::{
    api::{
        lightining_api::{
            ChannelBalance, GetLightningBalanceApi, LightningInvoiceApi, LightningPaymentApi,
            LightningTransaction, LightningTransactionEvent, LightningTransactionStreamApi,
            LnInvoice, NodeBalance,
        },
        on_chain_api::{
            GetOnChainBalanceApi, OnChainBalance, OnChainInvoiceApi, OnChainPaymentApi,
            OnChainPaymentResult, OnChainTransaction, OnChainTransactionApi,
            OnChainTransactionEvent, OnChainTransactionStreamApi,
        },
    },
    payment::amount::Amount,
    Error, Result,
};
use tokio::{sync::mpsc::Sender, task::JoinHandle};
use tokio_stream::StreamExt;

use crate::wrapper::LndRpcWrapper;

// The numeric state that LND indicates a settled invoice with.
const LND_SETTLED: i32 = 1;

#[derive(Clone)]
pub struct Lnd {
    client: LndRpcWrapper,
    pub(super) node_id: String,
    network: Network,
}

impl Lnd {
    pub async fn new(config: LndConfig) -> Result<Self> {
        let client = LndRpcWrapper::new(config.clone()).await?;
        let node_id = config.node_id();
        let network = config.network();
        Ok(Self {
            client,
            node_id,
            network,
        })
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
impl GetLightningBalanceApi for Lnd {
    async fn get_channel_balance(&self) -> Result<ChannelBalance> {
        let res = self.client.get_channel_balance().await?;

        Ok(ChannelBalance {
            local_balance: res
                .local_balance
                .map(|a| Amount::sats(a.sat))
                .get_or_insert(Amount::sats(0))
                .to_owned(),
            remote_balance: res
                .remote_balance
                .map(|a| Amount::sats(a.sat))
                .get_or_insert(Amount::sats(0))
                .to_owned(),
        })
    }

    async fn get_balances(&self) -> Result<NodeBalance> {
        let onchain = self.get_onchain_balance().await?;
        let channel = self.get_channel_balance().await?;
        Ok(NodeBalance { onchain, channel })
    }
}

#[async_trait]
impl OnChainInvoiceApi for Lnd {
    async fn new_address(&self) -> Result<Address> {
        self.client.new_address().await
    }
}

#[async_trait]
impl LightningInvoiceApi for Lnd {
    async fn create_ln_invoice(
        &self,
        amount: Amount,
        memo: Option<String>,
        ttl: Option<i64>,
    ) -> Result<LnInvoice> {
        let amount = bitcoin::Amount::from_sat(amount.cent_amount);
        let invoice = self.client.create_invoice(amount, memo, ttl).await?;
        Ok(invoice)
    }
}

#[async_trait]
impl OnChainPaymentApi for Lnd {
    fn validate_address(&self, address: &str) -> Result<Address> {
        to_address(address, self.network)
    }

    async fn estimate_fee(
        &self,
        target_conf: i32,
        outputs: HashMap<String, bitcoin::Amount>,
    ) -> Result<bitcoin::Amount> {
        let out = outputs
            .iter()
            .map(|p| (p.0.to_owned(), p.1.to_sat() as i64))
            .collect();
        let fee = self.client.estimate_fee(target_conf, out).await?;
        Ok(fee)
    }

    async fn send(
        &self,
        amount: bitcoin::Amount,
        address: String,
        sats_per_vbyte: bitcoin::Amount,
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
        outputs: HashMap<String, bitcoin::Amount>,
        sats_per_vbyte: bitcoin::Amount,
    ) -> Result<OnChainPaymentResult> {
        let out = outputs
            .iter()
            .flat_map(|(k, v)| {
                to_address(k, self.network)
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
impl LightningPaymentApi for Lnd {
    async fn pay_to_node_pub_key(&self, pub_key: String, amount: Amount) -> Result<()> {
        let amt = bitcoin::Amount::from_sat(amount.cent_amount);
        self.client
            .pay_to_node_id(pub_key.parse()?, amt, None)
            .await?;
        Ok(())
    }

    async fn pay_invoice(&self, invoice: Bolt11Invoice) -> Result<()> {
        self.client.pay_invoice(invoice, None, None).await?;
        Ok(())
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
            .flat_map(|tx| to_on_chain_events(tx, self.network, &self.node_id))
            .flatten()
            .collect();
        Ok(result)
    }
}

#[derive(Debug, Clone)]
pub enum LndConfig {
    /// with custom cert file and macaroon binary file
    CertPath {
        node_id: String,
        address: String,
        cert_path: String,
        macaroon_file: String,
        network: Network,
    },
    /// with root cert and macaroon string
    RootCert {
        node_id: String,
        address: String,
        macaroon: String,
        network: Network,
    },
}

impl LndConfig {
    pub fn network(&self) -> Network {
        match self {
            LndConfig::CertPath { network, .. } => *network,
            LndConfig::RootCert { network, .. } => *network,
        }
    }

    pub fn node_id(&self) -> String {
        match self {
            LndConfig::CertPath { node_id, .. } => node_id.to_owned(),
            LndConfig::RootCert { node_id, .. } => node_id.to_owned(),
        }
    }

    pub fn address(&self) -> String {
        match self {
            LndConfig::CertPath { address, .. } => address.to_owned(),
            LndConfig::RootCert { address, .. } => address.to_owned(),
        }
    }
}

pub struct LndPaymentEventStream {
    config: LndConfig,
    node_id: String,
}

impl LndPaymentEventStream {
    pub fn new(config: LndConfig) -> Self {
        let node_id = config.node_id();
        Self { config, node_id }
    }

    /// does fetch potential missing events from the current start_height
    async fn start_subscription(&self, start_height: u64) -> Result<Vec<OnChainTransactionEvent>> {
        let lnd = Lnd::new(self.config.clone()).await?;
        let events = lnd
            .get_onchain_transactions(start_height as i32, -1)
            .await?;
        Ok(events)
    }
}

#[async_trait]
impl OnChainTransactionStreamApi for LndPaymentEventStream {
    fn node_id(&self) -> String {
        self.node_id.to_owned()
    }
    async fn subscribe_on_chain_transactions(
        &self,
        sender: Sender<OnChainTransactionEvent>,
        start_height: Option<u64>,
    ) -> Result<JoinHandle<()>> {
        let config = self.config.clone();
        let start_events = self
            .start_subscription(start_height.unwrap_or_default())
            .await
            .ok()
            .unwrap_or(vec![]);

        // catch up to from start height to now
        for event in start_events {
            if let Err(e) = sender.send(event).await {
                println!(
                    "Failed to send historic on chain transaction event: {:?}",
                    e
                );
            }
        }
        let handle = tokio::spawn(async move {
            let sender = sender.clone();
            let network = config.network();
            let node_id = config.node_id();
            if let Ok(mut lnd) = create_client(config.clone()).await {
                let mut stream = lnd
                    .lightning()
                    .subscribe_transactions(GetTransactionsRequest::default())
                    .await
                    .expect("Failed to subscribe to LND on-chain transaction events")
                    .into_inner()
                    .filter(|tx| tx.is_ok())
                    .map(|tx| tx.unwrap());

                while let Some(event) = stream.next().await {
                    if let Ok(events) = to_on_chain_events(&event, network, &node_id) {
                        for event in events {
                            if let Err(e) = sender.send(event).await {
                                error!("Failed to send on chain transaction event: {:?}", e);
                            }
                        }
                    }
                }
            } else {
                error!(
                    "Failed to connect to LND {} {}",
                    config.node_id(),
                    config.address()
                );
            }
        });
        Ok(handle)
    }
}

#[async_trait]
impl LightningTransactionStreamApi for LndPaymentEventStream {
    async fn subscribe_lightning_transactions(
        &self,
        sender: Sender<LightningTransactionEvent>,
        settle_index: Option<u64>,
    ) -> Result<JoinHandle<()>> {
        let config = self.config.clone();

        let handle = tokio::spawn(async move {
            let sender = sender.clone();
            if let Ok(mut lnd) = create_client(config.clone()).await {
                let mut stream = lnd
                    .lightning()
                    .subscribe_invoices(InvoiceSubscription {
                        settle_index: settle_index.unwrap_or_default(),
                        ..Default::default()
                    })
                    .await
                    .expect("Failed to subscribe to LND lightning transaction events")
                    .into_inner()
                    .filter_map(|tx| tx.ok());

                while let Some(event) = stream.next().await {
                    if event.state == LND_SETTLED {
                        if let Ok(event) = to_lightning_event(event, &config.node_id()) {
                            if let Err(e) = sender.send(event).await {
                                println!("Failed to send lightning transaction event: {:?}", e);
                            }
                        }
                    }
                }
            } else {
                error!(
                    "Failed to connect to LND {} {}",
                    config.node_id(),
                    config.address()
                );
            }
        });
        Ok(handle)
    }
}

fn to_lightning_event(
    event: fedimint_tonic_lnd::lnrpc::Invoice,
    node_id: &str,
) -> Result<LightningTransactionEvent> {
    Ok(LightningTransactionEvent::Settled(LightningTransaction {
        node_id: node_id.to_owned(),
        r_hash: event.r_hash.to_lower_hex_string(),
        invoice: event.payment_request.to_owned(),
        amount: Amount::sats(event.value as u64),
        amount_paid: Amount::sats(event.amt_paid_sat as u64),
        settle_index: event.settle_index,
    }))
}

/// Converts a satoshi amount to an Amount
fn to_amount(sats: i64) -> bitcoin::Amount {
    if sats < 0 {
        bitcoin::Amount::ZERO
    } else {
        bitcoin::Amount::from_sat(sats.unsigned_abs())
    }
}

/// Converts a Transaction to a list of OnChainTransactionEvents.
fn to_on_chain_events(
    tx: &Transaction,
    chain: Network,
    node_id: &str,
) -> Result<Vec<OnChainTransactionEvent>> {
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
                    node_id: node_id.to_owned(),
                    confirmations: tx.num_confirmations,
                    amount: bitcoin::Amount::from_sat(tx.amount.unsigned_abs()),
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

pub(crate) async fn create_client(config: LndConfig) -> Result<Client> {
    let lnd: Client = match config {
        LndConfig::RootCert {
            address, macaroon, ..
        } => fedimint_tonic_lnd::connect_root(address.to_string(), macaroon.to_string())
            .await
            .map_err(|e| Error::NodeConnect(e.to_string()))?,
        LndConfig::CertPath {
            address,
            cert_path,
            macaroon_file,
            ..
        } => fedimint_tonic_lnd::connect(
            address.to_string(),
            cert_path.to_string(),
            macaroon_file.to_string(),
        )
        .await
        .map_err(|e| Error::NodeConnect(e.to_string()))?,
    };
    Ok(lnd)
}
