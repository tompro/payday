use std::{collections::HashMap, sync::Arc};

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
        lightning_api::{
            ChannelBalance, GetLightningBalanceApi, LightningInvoiceApi, LightningPaymentApi,
            LightningTransaction, LightningTransactionApi, LightningTransactionEvent,
            LightningTransactionStreamApi, LnInvoice, NodeBalance,
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

use crate::wrapper::{LndApi, LndRpcWrapper};

// The numeric state that LND indicates a settled invoice with.
const LND_SETTLED: i32 = 1;

#[derive(Clone)]
pub struct Lnd {
    client: Arc<dyn LndApi>,
    pub(super) node_id: String,
    network: Network,
}

impl Lnd {
    pub async fn new(config: LndConfig) -> Result<Self> {
        let client = Arc::new(LndRpcWrapper::new(config.clone()).await?);
        let node_id = config.node_id();
        let network = config.network();
        Ok(Self {
            client,
            node_id,
            network,
        })
    }

    pub async fn with_lnd_api(config: LndConfig, lnd: Arc<dyn LndApi>) -> Result<Self> {
        let node_id = config.node_id();
        let network = config.network();
        Ok(Self {
            client: lnd,
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

#[async_trait]
impl LightningTransactionApi for Lnd {
    /// Get history of lightning transactions between start date timestamp and end date timestamp.
    async fn get_lightning_transactions(
        &self,
        from: i64,
        to: i64,
        limit: i64,
        index: i64,
    ) -> Result<Vec<LightningTransaction>> {
        let result = self
            .client
            .get_invoices(from, to, limit, index)
            .await?
            .invoices
            .into_iter()
            .map(|e| to_lightning_transaction(e, &self.node_id))
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
                println!("Failed to send historic on chain transaction event: {e:?}");
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
                                error!("Failed to send on chain transaction event: {e:?}");
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
                                println!("Failed to send lightning transaction event: {e:?}");
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
    Ok(LightningTransactionEvent::Settled(
        to_lightning_transaction(event, node_id),
    ))
}

fn to_lightning_transaction(
    event: fedimint_tonic_lnd::lnrpc::Invoice,
    node_id: &str,
) -> LightningTransaction {
    LightningTransaction {
        node_id: node_id.to_owned(),
        r_hash: event.r_hash.to_lower_hex_string(),
        invoice: event.payment_request.to_owned(),
        amount: Amount::sats(event.value as u64),
        amount_paid: Amount::sats(event.amt_paid_sat as u64),
        settle_index: event.settle_index,
        create_date: event.creation_date as u64,
        settle_date: event.settle_date as u64,
        memo: if event.memo.is_empty() {
            None
        } else {
            Some(event.memo.to_owned())
        },
    }
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

#[cfg(test)]
mod tests {

    use std::{collections::HashMap, str::FromStr, sync::Arc};

    use bitcoin::{Address, Amount as BitcoinAmount, Network};
    use fedimint_tonic_lnd::lnrpc::Amount as LndAmount;
    use lightning_invoice::Bolt11Invoice;
    use mockall::predicate::*;

    use crate::lnd::{Lnd, LndConfig};
    use crate::wrapper::MockLndApi;
    use fedimint_tonic_lnd::lnrpc::{ChannelBalanceResponse, Invoice, OutputDetail, Transaction};
    use payday_core::{
        api::{
            lightning_api::{
                GetLightningBalanceApi, LightningInvoiceApi, LightningPaymentApi,
                LightningTransactionEvent, LnInvoice,
            },
            on_chain_api::{
                GetOnChainBalanceApi, OnChainInvoiceApi, OnChainPaymentApi, OnChainTransactionApi,
                OnChainTransactionEvent,
            },
        },
        payment::amount::Amount,
    };

    fn create_test_config() -> LndConfig {
        LndConfig::RootCert {
            node_id: "test_node_id".to_string(),
            address: "localhost:10001".to_string(),
            macaroon: "test_macaroon".to_string(),
            network: Network::Testnet,
        }
    }

    fn create_test_txn(amount: i64, is_confirmed: bool, is_our_address: bool) -> Transaction {
        Transaction {
            tx_hash: "abcdef1234567890".to_string(),
            amount,
            num_confirmations: if is_confirmed { 3 } else { 0 },
            block_height: if is_confirmed { 600_000 } else { 0 },
            time_stamp: 1609459200, // 2021-01-01
            total_fees: 1000,
            output_details: vec![OutputDetail {
                address: "2N3oefVeg6stiTb5Kh3ozCSkaqmx91FDbsm".to_string(),
                amount: amount.abs(),
                is_our_address,
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    fn create_test_invoice(value: i64, settled: bool) -> Invoice {
        Invoice {
            memo: "test memo".to_string(),
            r_preimage: vec![1, 2, 3],
            r_hash: vec![4, 5, 6],
            value,
            value_msat: value * 1000,
            creation_date: 1609459200, // 2021-01-01
            settle_date: if settled { 1609459300 } else { 0 },
            payment_request: "lnbcrt1p0abcdefghijklmnopqrstuvwxyz".to_string(),
            state: if settled { 1 } else { 0 },
            amt_paid_sat: if settled { value } else { 0 },
            settle_index: if settled { 42 } else { 0 },
            ..Default::default()
        }
    }

    fn create_test_bolt11_invoice() -> Bolt11Invoice {
        Bolt11Invoice::from_str(
            "lntbs3m1pnf36h3pp5dm63f7meus5thxd3h23uqkfuydw340nrf6v8y398ga7tqjfrpnfsdq5w3jhxapqd9h8vmmfvdjscqzzsxq97ztucsp5yle6azm0tpy7h3dh0d6kmpzzzpyvzqkck476l96z5p5leqaraumq9qyyssqghpt4k54rrutwumlq6hav5wdjghlrxnyxe5dde37e5t4wwz4kkq3r5284l3rcnyzzqvry6xz4s8mq42npq8fzr7j9tvvuyh32xmh97gq0h8hdp"
        ).expect("valid invoice")
    }

    #[tokio::test]
    async fn test_get_onchain_balance() {
        let mut mock = MockLndApi::new();
        mock.expect_get_onchain_balance().times(1).returning(|| {
            Ok(fedimint_tonic_lnd::lnrpc::WalletBalanceResponse {
                total_balance: 100_000,
                confirmed_balance: 90_000,
                unconfirmed_balance: 10_000,
                ..Default::default()
            })
        });

        let lnd = Lnd::with_lnd_api(create_test_config(), Arc::new(mock))
            .await
            .unwrap();
        let balance = lnd.get_onchain_balance().await.unwrap();

        assert_eq!(balance.total_balance.to_sat(), 100_000);
        assert_eq!(balance.confirmed_balance.to_sat(), 90_000);
        assert_eq!(balance.unconfirmed_balance.to_sat(), 10_000);
    }

    #[tokio::test]
    async fn test_get_channel_balance() {
        let mut mock = MockLndApi::new();
        mock.expect_get_channel_balance().times(1).returning(|| {
            Ok(ChannelBalanceResponse {
                local_balance: Some(LndAmount {
                    sat: 50_000,
                    msat: 50_000_000,
                }),
                remote_balance: Some(LndAmount {
                    sat: 30_000,
                    msat: 30_000_000,
                }),
                ..Default::default()
            })
        });

        let lnd = Lnd::with_lnd_api(create_test_config(), Arc::new(mock))
            .await
            .unwrap();
        let balance = lnd.get_channel_balance().await.unwrap();

        assert_eq!(balance.local_balance.cent_amount, 50_000);
        assert_eq!(balance.remote_balance.cent_amount, 30_000);
    }

    #[tokio::test]
    async fn test_get_balances() {
        let mut mock = MockLndApi::new();
        mock.expect_get_onchain_balance().times(1).returning(|| {
            Ok(fedimint_tonic_lnd::lnrpc::WalletBalanceResponse {
                total_balance: 100_000,
                confirmed_balance: 90_000,
                unconfirmed_balance: 10_000,
                ..Default::default()
            })
        });

        mock.expect_get_channel_balance().times(1).returning(|| {
            Ok(ChannelBalanceResponse {
                local_balance: Some(LndAmount {
                    sat: 50_000,
                    msat: 50_000_000,
                }),
                remote_balance: Some(LndAmount {
                    sat: 30_000,
                    msat: 30_000_000,
                }),
                ..Default::default()
            })
        });

        let lnd = Lnd::with_lnd_api(create_test_config(), Arc::new(mock))
            .await
            .unwrap();
        let balance = lnd.get_balances().await.unwrap();

        assert_eq!(balance.onchain.total_balance.to_sat(), 100_000);
        assert_eq!(balance.channel.local_balance.cent_amount, 50_000);
        assert_eq!(balance.channel.remote_balance.cent_amount, 30_000);
    }

    #[tokio::test]
    async fn test_new_address() {
        let mut mock = MockLndApi::new();
        let expected_address = Address::from_str("2N3oefVeg6stiTb5Kh3ozCSkaqmx91FDbsm")
            .unwrap()
            .require_network(Network::Testnet)
            .unwrap();

        mock.expect_new_address()
            .times(1)
            .returning(move || Ok(expected_address.clone()));

        let lnd = Lnd::with_lnd_api(create_test_config(), Arc::new(mock))
            .await
            .unwrap();
        let address = lnd.new_address().await.unwrap();

        assert_eq!(address.to_string(), "2N3oefVeg6stiTb5Kh3ozCSkaqmx91FDbsm");
    }

    #[tokio::test]
    async fn test_create_ln_invoice() {
        let mut mock = MockLndApi::new();
        let expected_invoice = LnInvoice {
            r_hash: "0405060708".to_string(),
            add_index: 1,
            invoice: create_test_bolt11_invoice(),
        };

        mock.expect_create_invoice()
            .with(
                eq(BitcoinAmount::from_sat(10000)),
                eq(Some("test memo".to_string())),
                eq(Some(3600)),
            )
            .times(1)
            .returning(move |_, _, _| Ok(expected_invoice.clone()));

        let lnd = Lnd::with_lnd_api(create_test_config(), Arc::new(mock))
            .await
            .unwrap();
        let invoice = lnd
            .create_ln_invoice(
                Amount::sats(10000),
                Some("test memo".to_string()),
                Some(3600),
            )
            .await
            .unwrap();

        assert_eq!(invoice.add_index, 1);
        assert_eq!(invoice.invoice, create_test_bolt11_invoice());
    }

    #[tokio::test]
    async fn test_validate_address() {
        let config = create_test_config();
        let mock = MockLndApi::new();

        let lnd = Lnd::with_lnd_api(config, Arc::new(mock)).await.unwrap();
        let address = lnd
            .validate_address("2N3oefVeg6stiTb5Kh3ozCSkaqmx91FDbsm")
            .unwrap();

        assert_eq!(address.to_string(), "2N3oefVeg6stiTb5Kh3ozCSkaqmx91FDbsm");

        // Test invalid address
        let result = lnd.validate_address("invalid_address");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_estimate_fee() {
        let mut mock = MockLndApi::new();
        let mut outputs = HashMap::new();
        outputs.insert(
            "2N3oefVeg6stiTb5Kh3ozCSkaqmx91FDbsm".to_string(),
            BitcoinAmount::from_sat(10000),
        );

        mock.expect_estimate_fee()
            .with(
                eq(2),
                eq(HashMap::from([(
                    "2N3oefVeg6stiTb5Kh3ozCSkaqmx91FDbsm".to_string(),
                    10000,
                )])),
            )
            .times(1)
            .returning(|_, _| Ok(BitcoinAmount::from_sat(500)));

        let lnd = Lnd::with_lnd_api(create_test_config(), Arc::new(mock))
            .await
            .unwrap();
        let fee = lnd.estimate_fee(2, outputs).await.unwrap();

        assert_eq!(fee.to_sat(), 500);
    }

    #[tokio::test]
    async fn test_send() {
        let mut mock = MockLndApi::new();
        let expected_txid = "abcdef1234567890".to_string();

        mock.expect_send_coins()
            .with(
                eq(BitcoinAmount::from_sat(10000)),
                eq("2N3oefVeg6stiTb5Kh3ozCSkaqmx91FDbsm"),
                eq(BitcoinAmount::from_sat(2)),
            )
            .times(1)
            .returning(|_, _, _| Ok("abcdef1234567890".to_string()));

        let lnd = Lnd::with_lnd_api(create_test_config(), Arc::new(mock))
            .await
            .unwrap();
        let result = lnd
            .send(
                BitcoinAmount::from_sat(10000),
                "2N3oefVeg6stiTb5Kh3ozCSkaqmx91FDbsm".to_string(),
                BitcoinAmount::from_sat(2),
            )
            .await
            .unwrap();

        assert_eq!(result.tx_id, expected_txid);
        assert_eq!(result.fee.to_sat(), 2);
        assert_eq!(
            result.amounts,
            HashMap::from([(
                "2N3oefVeg6stiTb5Kh3ozCSkaqmx91FDbsm".to_string(),
                BitcoinAmount::from_sat(10000)
            )])
        );
    }

    #[tokio::test]
    async fn test_batch_send() {
        let mut mock = MockLndApi::new();
        let expected_txid = "abcdef1234567890".to_string();

        let mut outputs = HashMap::new();
        outputs.insert(
            "2N3oefVeg6stiTb5Kh3ozCSkaqmx91FDbsm".to_string(),
            BitcoinAmount::from_sat(10000),
        );
        outputs.insert(
            "miSR5gDi63WoBVo3mDcHKhEXxP7sR9FrQg".to_string(),
            BitcoinAmount::from_sat(20000),
        );

        let address1 = Address::from_str("2N3oefVeg6stiTb5Kh3ozCSkaqmx91FDbsm")
            .unwrap()
            .require_network(Network::Testnet)
            .unwrap();
        let address2 = Address::from_str("miSR5gDi63WoBVo3mDcHKhEXxP7sR9FrQg")
            .unwrap()
            .require_network(Network::Testnet)
            .unwrap();

        mock.expect_batch_send()
            .with(
                eq(HashMap::from([
                    (address1.clone(), 10000),
                    (address2.clone(), 20000),
                ])),
                eq(BitcoinAmount::from_sat(3)),
            )
            .times(1)
            .returning(|_, _| Ok("abcdef1234567890".to_string()));

        let lnd = Lnd::with_lnd_api(create_test_config(), Arc::new(mock))
            .await
            .unwrap();
        let result = lnd
            .batch_send(outputs.clone(), BitcoinAmount::from_sat(3))
            .await
            .unwrap();

        assert_eq!(result.tx_id, expected_txid);
        assert_eq!(result.fee.to_sat(), 3);
        assert_eq!(result.amounts, outputs);
    }

    #[tokio::test]
    async fn test_pay_to_node_pub_key() {
        let mut mock = MockLndApi::new();

        mock.expect_pay_to_node_id()
            .times(1)
            .returning(|_, _, _| Ok(fedimint_tonic_lnd::lnrpc::Payment::default()));

        let lnd = Lnd::with_lnd_api(create_test_config(), Arc::new(mock))
            .await
            .unwrap();
        let result = lnd
            .pay_to_node_pub_key(
                "02eadbd9e7557375161df8b646776a547c5cbc2e95b3071ec81553f8ec2cea3b8c".to_string(),
                Amount::sats(10000),
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_pay_invoice() {
        let mut mock = MockLndApi::new();
        let invoice = create_test_bolt11_invoice();

        mock.expect_pay_invoice()
            .times(1)
            .returning(|_, _, _| Ok(fedimint_tonic_lnd::lnrpc::Payment::default()));

        let lnd = Lnd::with_lnd_api(create_test_config(), Arc::new(mock))
            .await
            .unwrap();
        let result = lnd.pay_invoice(invoice).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_onchain_transactions() {
        let mut mock = MockLndApi::new();

        let tx1 = create_test_txn(10000, true, true); // received confirmed
        let tx2 = create_test_txn(-5000, true, false); // sent confirmed
        let tx3 = create_test_txn(3000, false, true); // received unconfirmed
        let tx4 = create_test_txn(-3000, false, false); // sent unconfirmed

        mock.expect_get_transactions()
            .with(eq(100), eq(200))
            .times(1)
            .returning(move |_, _| Ok(vec![tx1.clone(), tx2.clone(), tx3.clone(), tx4.clone()]));

        let lnd = Lnd::with_lnd_api(create_test_config(), Arc::new(mock))
            .await
            .unwrap();
        let events = lnd.get_onchain_transactions(100, 200).await.unwrap();

        // We should get 3 events, one for each transaction
        assert_eq!(events.len(), 4);

        // Check that we have the correct event types
        let event_types = events
            .iter()
            .map(|e| match e {
                OnChainTransactionEvent::ReceivedConfirmed(_) => "ReceivedConfirmed",
                OnChainTransactionEvent::SentConfirmed(_) => "SentConfirmed",
                OnChainTransactionEvent::ReceivedUnconfirmed(_) => "ReceivedUnconfirmed",
                OnChainTransactionEvent::SentUnconfirmed(_) => "SentUnconfirmed",
            })
            .collect::<Vec<&str>>();

        assert!(event_types.contains(&"ReceivedConfirmed"));
        assert!(event_types.contains(&"SentConfirmed"));
        assert!(event_types.contains(&"ReceivedUnconfirmed"));
        assert!(event_types.contains(&"SentUnconfirmed"));
    }

    #[test]
    fn test_config_methods() {
        let config = create_test_config();

        assert_eq!(config.node_id(), "test_node_id");
        assert_eq!(config.address(), "localhost:10001");
        assert_eq!(config.network(), Network::Testnet);

        // Test CertPath config
        let cert_config = LndConfig::CertPath {
            node_id: "cert_node_id".to_string(),
            address: "localhost:10002".to_string(),
            cert_path: "/path/to/cert".to_string(),
            macaroon_file: "/path/to/macaroon".to_string(),
            network: Network::Bitcoin,
        };

        assert_eq!(cert_config.node_id(), "cert_node_id");
        assert_eq!(cert_config.address(), "localhost:10002");
        assert_eq!(cert_config.network(), Network::Bitcoin);
    }

    #[test]
    fn test_to_amount() {
        use crate::lnd::to_amount;

        // Test positive amount
        assert_eq!(to_amount(1000).to_sat(), 1000);

        // Test zero
        assert_eq!(to_amount(0).to_sat(), 0);

        // Test negative (should return zero)
        assert_eq!(to_amount(-1000).to_sat(), 0);
    }

    #[test]
    fn test_to_on_chain_events() {
        use crate::lnd::to_on_chain_events;

        let node_id = "test_node_id";
        let network = Network::Testnet;

        // Test received confirmed
        let tx1 = create_test_txn(10000, true, true);
        let events1 = to_on_chain_events(&tx1, network, node_id).unwrap();
        assert_eq!(events1.len(), 1);
        match &events1[0] {
            OnChainTransactionEvent::ReceivedConfirmed(tx) => {
                assert_eq!(tx.tx_id, "abcdef1234567890");
                assert_eq!(tx.amount.to_sat(), 10000);
                assert_eq!(tx.node_id, "test_node_id");
            }
            _ => panic!("Expected ReceivedConfirmed event"),
        }

        // Test sent confirmed
        let tx2 = create_test_txn(-5000, true, false);
        let events2 = to_on_chain_events(&tx2, network, node_id).unwrap();
        assert_eq!(events2.len(), 1);
        match &events2[0] {
            OnChainTransactionEvent::SentConfirmed(tx) => {
                assert_eq!(tx.tx_id, "abcdef1234567890");
                assert_eq!(tx.amount.to_sat(), 5000);
                assert_eq!(tx.node_id, "test_node_id");
            }
            _ => panic!("Expected SentConfirmed event"),
        }

        // Test received unconfirmed
        let tx3 = create_test_txn(3000, false, true);
        let events3 = to_on_chain_events(&tx3, network, node_id).unwrap();
        assert_eq!(events3.len(), 1);
        match &events3[0] {
            OnChainTransactionEvent::ReceivedUnconfirmed(tx) => {
                assert_eq!(tx.tx_id, "abcdef1234567890");
                assert_eq!(tx.amount.to_sat(), 3000);
                assert_eq!(tx.node_id, "test_node_id");
            }
            _ => panic!("Expected ReceivedUnconfirmed event"),
        }

        // Test sent unconfirmed
        let tx4 = create_test_txn(-2000, false, false);
        let events4 = to_on_chain_events(&tx4, network, node_id).unwrap();
        assert_eq!(events4.len(), 1);
        match &events4[0] {
            OnChainTransactionEvent::SentUnconfirmed(tx) => {
                assert_eq!(tx.tx_id, "abcdef1234567890");
                assert_eq!(tx.amount.to_sat(), 2000);
                assert_eq!(tx.node_id, "test_node_id");
            }
            _ => panic!("Expected SentUnconfirmed event"),
        }
    }

    #[test]
    fn test_to_lightning_event() {
        use crate::lnd::to_lightning_event;

        let node_id = "test_node_id";

        // Test settled invoice
        let invoice = create_test_invoice(10000, true);
        let event = to_lightning_event(invoice, node_id).unwrap();

        match event {
            LightningTransactionEvent::Settled(tx) => {
                assert_eq!(tx.node_id, "test_node_id");
                assert_eq!(tx.r_hash, "040506");
                assert_eq!(tx.invoice, "lnbcrt1p0abcdefghijklmnopqrstuvwxyz");
                assert_eq!(tx.amount.cent_amount, 10000);
                assert_eq!(tx.amount_paid.cent_amount, 10000);
                assert_eq!(tx.settle_index, 42);
            }
        }
    }
}
