//! Wrapper for LND RPC client.
//!
//! This module provides a wrapper around the LND RPC client. It
//! handles connection and network checks, maps errors to project
//! specific errors, and provides a convenient interface for the
//! operations needed for invoicing.
use std::{collections::HashMap, sync::Arc, time::Duration};

use bitcoin::{hex::DisplayHex, Address, Amount, Network, PublicKey};
use fedimint_tonic_lnd::{
    lnrpc::{
        payment::PaymentStatus, ChannelBalanceRequest, ChannelBalanceResponse, GetInfoRequest,
        GetTransactionsRequest, Invoice, SendCoinsRequest, SendManyRequest, Transaction,
        WalletBalanceRequest, WalletBalanceResponse,
    },
    Client,
};
use lightning_invoice::Bolt11Invoice;
use payday_btc::to_address;
use payday_core::{api::lightining_api::LnInvoice, Error, PaydayStream, Result};
use tokio::sync::{Mutex, MutexGuard};
use tokio_stream::StreamExt;

use crate::lnd::LndConfig;

#[derive(Clone)]
pub struct LndRpcWrapper {
    config: LndConfig,
    client: Arc<Mutex<Client>>,
}

impl LndRpcWrapper {
    /// Create a new LND RPC wrapper. Creates an RPC connection and
    /// checks whether the RPC server is serving the expected network.
    pub async fn new(config: LndConfig) -> Result<Self> {
        let mut lnd: Client = fedimint_tonic_lnd::connect(
            config.address.to_string(),
            config.cert_path.to_string(),
            config.macaroon_file.to_string(),
        )
        .await
        .map_err(|e| Error::NodeConnectError(e.to_string()))?;

        let network_info = lnd
            .lightning()
            .get_info(GetInfoRequest {})
            .await
            .map_err(|e| Error::NodeApiError(e.to_string()))?
            .into_inner()
            .chains
            .first()
            .expect("no network info found")
            .network
            .to_string();

        if config.network != network_from_str(&network_info)? {
            return Err(Error::InvalidBitcoinNetwork(network_info));
        }
        Ok(Self {
            config,
            client: Arc::new(Mutex::new(lnd)),
        })
    }

    /// Get the unique name of the LND server. Names are used to
    /// identify the server in logs and associated addresses and invoices.
    pub fn get_name(&self) -> String {
        self.config.node_id.to_string()
    }

    async fn client(&self) -> MutexGuard<Client> {
        self.client.lock().await
    }

    pub async fn get_onchain_balance(&self) -> Result<WalletBalanceResponse> {
        let mut lnd = self.client().await;
        Ok(lnd
            .lightning()
            .wallet_balance(WalletBalanceRequest {})
            .await
            .map_err(|e| Error::NodeApiError(e.to_string()))?
            .into_inner())
    }

    pub async fn get_channel_balance(&self) -> Result<ChannelBalanceResponse> {
        let mut lnd = self.client().await;
        Ok(lnd
            .lightning()
            .channel_balance(ChannelBalanceRequest {})
            .await
            .map_err(|e| Error::NodeApiError(e.to_string()))?
            .into_inner())
    }

    /// Get the current balances (onchain and lightning) of the wallet.
    pub async fn get_balances(&self) -> Result<(WalletBalanceResponse, ChannelBalanceResponse)> {
        let on_chain = self.get_onchain_balance().await?;
        let lightning = self.get_channel_balance().await?;
        Ok((on_chain, lightning))
    }

    /// Get a new onchain address for the wallet. Address is parsed and
    /// validated for the configure network.
    pub async fn new_address(&self) -> Result<Address> {
        let addr = self
            .client()
            .await
            .lightning()
            .new_address(fedimint_tonic_lnd::lnrpc::NewAddressRequest {
                ..Default::default()
            })
            .await
            .map_err(|e| Error::NodeApiError(e.to_string()))?
            .into_inner()
            .address;
        let address = to_address(&addr, self.config.network)?;
        Ok(address)
    }

    /// Send coins to an address. Address is parsed and validated for the configure network.
    /// Returns the transaction id.
    pub async fn send_coins(
        &self,
        amount: Amount,
        address: &str,
        sats_per_vbyte: Amount,
    ) -> Result<String> {
        let checked_address = to_address(address, self.config.network)?;
        let txid = self
            .client()
            .await
            .lightning()
            .send_coins(SendCoinsRequest {
                addr: checked_address.to_string(),
                amount: amount.to_sat() as i64,
                sat_per_vbyte: sats_per_vbyte.to_sat(),
                ..Default::default()
            })
            .await
            .map_err(|e| Error::NodeApiError(e.to_string()))?
            .into_inner()
            .txid;

        Ok(txid.to_string())
    }

    /// Send coins to multiple addresses.
    pub async fn batch_send(
        &self,
        outputs: HashMap<Address, i64>,
        sats_per_vbyte: Amount,
    ) -> Result<String> {
        let out = outputs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_owned()))
            .collect();
        let txid = self
            .client()
            .await
            .lightning()
            .send_many(SendManyRequest {
                addr_to_amount: out,
                sat_per_vbyte: sats_per_vbyte.to_sat(),
                ..Default::default()
            })
            .await
            .map_err(|e| Error::NodeApiError(e.to_string()))?
            .into_inner()
            .txid;

        Ok(txid.to_owned())
    }

    /// Estimate the fee for a transaction.
    pub async fn estimate_fee(
        &self,
        target_conf: i32,
        outputs: HashMap<String, i64>,
    ) -> Result<Amount> {
        let fee = self
            .client()
            .await
            .lightning()
            .estimate_fee(fedimint_tonic_lnd::lnrpc::EstimateFeeRequest {
                target_conf,
                addr_to_amount: outputs,
                ..Default::default()
            })
            .await
            .map_err(|e| Error::NodeApiError(e.to_string()))?
            .into_inner()
            .sat_per_vbyte;

        Ok(Amount::from_sat(fee))
    }

    /// Creates a lightning invoice.
    pub async fn create_invoice(
        &self,
        amount: Amount,
        memo: Option<String>,
        ttl: Option<i64>,
    ) -> Result<LnInvoice> {
        let mut lnd = self.client().await;
        let invoice = lnd
            .lightning()
            .add_invoice(Invoice {
                value: amount.to_sat() as i64,
                memo: memo.unwrap_or("ln invoice".to_string()),
                expiry: ttl.unwrap_or(3600i64),
                ..Default::default()
            })
            .await
            .map_err(|e| Error::NodeApiError(e.to_string()))?
            .into_inner();

        Ok(LnInvoice {
            invoice: invoice.payment_request,
            r_hash: invoice.r_hash.as_hex().to_string(),
            add_index: invoice.add_index,
        })
    }

    /// Pay a given bolt11 invoice. The fee limit is optional and defaults to 0 (no limit) the
    /// optional timeout defaults to 60 seconds.
    pub async fn send_lightning_payment(
        &self,
        request: fedimint_tonic_lnd::routerrpc::SendPaymentRequest,
    ) -> Result<fedimint_tonic_lnd::lnrpc::Payment> {
        let mut lnd = self.client().await;
        let result = lnd
            .router()
            .send_payment_v2(request)
            .await
            .map_err(|e| Error::NodeApiError(e.to_string()))?;

        // subscribe until the first non-inflight payment is received
        match result
            .into_inner()
            .filter_map(|r| match r.ok() {
                Some(p) if p.status() != PaymentStatus::InFlight => Some(p),
                _ => None,
            })
            .next()
            .await
        {
            Some(p) if p.status() == PaymentStatus::Succeeded => Ok(p),
            Some(p) => Err(Error::LightningPaymentFailed(format!(
                "Lightning payment failed: Status: {}, Reason:{}",
                p.status().as_str_name(),
                p.failure_reason().as_str_name()
            ))),
            _ => Err(Error::LightningPaymentFailed(
                "Lightning payment failed without response".to_string(),
            )),
        }
    }

    /// Pay a given bolt11 invoice. The fee limit is optional and defaults to 0 (no limit) the
    /// optional timeout defaults to 60 seconds.
    pub async fn pay_invoice(
        &self,
        invoice: Bolt11Invoice,
        fee_limit_sat: Option<i64>,
        timeout: Option<Duration>,
    ) -> Result<fedimint_tonic_lnd::lnrpc::Payment> {
        let timeout_seconds = timeout.map(|t| t.as_secs() as i32).unwrap_or(60);
        let result = self
            .send_lightning_payment(fedimint_tonic_lnd::routerrpc::SendPaymentRequest {
                payment_request: invoice.to_string(),
                timeout_seconds,
                fee_limit_sat: fee_limit_sat.unwrap_or(0),
                no_inflight_updates: true,
                ..Default::default()
            })
            .await?;
        Ok(result)
    }

    /// Pay a specified amount to a node id. The optional timeout defaults to 60 seconds.
    pub async fn pay_to_node_id(
        &self,
        node_id: PublicKey,
        amount: Amount,
        timeout: Option<Duration>,
    ) -> Result<fedimint_tonic_lnd::lnrpc::Payment> {
        let timeout_seconds = timeout.map(|t| t.as_secs() as i32).unwrap_or(60);
        let result = self
            .send_lightning_payment(fedimint_tonic_lnd::routerrpc::SendPaymentRequest {
                dest: node_id.to_bytes(),
                amt: amount.to_sat() as i64,
                timeout_seconds,
                ..Default::default()
            })
            .await?;
        Ok(result)
    }

    /// Get a stream of onchain transactions relevant to the wallet. As LND RPC does not handle
    /// the request arguments, we do not provide any on this method to avoid confusion.
    pub async fn subscribe_transactions(&self) -> Result<PaydayStream<Transaction>> {
        let mut lnd = self.client().await;
        let stream = lnd
            .lightning()
            .subscribe_transactions(GetTransactionsRequest::default())
            .await
            .map_err(|e| Error::NodeApiError(e.to_string()))?
            .into_inner()
            .filter(|tx| tx.is_ok())
            .map(|tx| tx.unwrap());
        Ok(Box::pin(stream))
    }

    /// Get a list of onchain transactions between the given start and end heights.
    pub async fn get_transactions(
        &self,
        start_height: i32,
        end_height: i32,
    ) -> Result<Vec<Transaction>> {
        let mut lnd = self.client().await;
        Ok(lnd
            .lightning()
            .get_transactions(GetTransactionsRequest {
                start_height,
                end_height,
                ..Default::default()
            })
            .await
            .map_err(|e| Error::NodeApiError(e.to_string()))?
            .into_inner()
            .transactions)
    }
}

fn network_from_str(s: &str) -> Result<Network> {
    let net = match s {
        "mainnet" => Network::Bitcoin,
        "testnet" => Network::Testnet,
        "regtest" => Network::Regtest,
        "signet" => Network::Signet,
        _ => Err(Error::InvalidBitcoinNetwork(s.to_string()))?,
    };
    Ok(net)
}
