use std::str::FromStr;

use async_trait::async_trait;
use bitcoin::{Address, Amount, Network};
use fedimint_tonic_lnd::Client;
use fedimint_tonic_lnd::lnrpc::{
    ChannelBalanceRequest, GetInfoRequest, Transaction, WalletBalanceRequest,
};
use tokio_stream::StreamExt;

use payday_core::{PaydayResult, PaydayStream};
use payday_core::error::PaydayError;
use payday_core::node::node_api::{
    Balance, ChannelBalance, NodeApi, OnChainBalance, OnChainTransactionEvent,
    OnChainTransactionResult,
};

pub struct LndRpc {
    network: Network,
    lnd: Client,
}

impl LndRpc {
    pub async fn new(
        address: String,
        cert_file: String,
        macaroon_file: String,
        allowed_network: Network,
    ) -> PaydayResult<Self> {
        let mut lnd: Client = fedimint_tonic_lnd::connect(address, cert_file, macaroon_file)
            .await
            .map_err(|e| PaydayError::NodeConnectError(e.to_string()))?;

        let network_info = lnd
            .lightning()
            .get_info(GetInfoRequest {})
            .await
            .map_err(|e| PaydayError::NodeApiError(e.to_string()))?
            .into_inner()
            .chains
            .first()
            .expect("no network info found")
            .network
            .to_string();

        let network = Network::from_core_arg(network_info.as_str())?;
        if allowed_network != network {
            return Err(PaydayError::InvalidBitcoinNetwork(network_info).into());
        }
        Ok(Self { network, lnd })
    }
}

impl LndRpc {
    async fn subscribe_transactions(
        &mut self,
        start_height: i32,
    ) -> PaydayResult<PaydayStream<Transaction>> {
        let stream = self
            .lnd
            .lightning()
            .subscribe_transactions(fedimint_tonic_lnd::lnrpc::GetTransactionsRequest {
                start_height,
                end_height: -1,
                ..Default::default()
            })
            .await
            .map_err(|e| PaydayError::NodeApiError(e.to_string()))?
            .into_inner()
            .filter(|tx| tx.is_ok())
            .map(|tx| tx.unwrap());
        Ok(Box::pin(stream))
    }

    async fn get_transactions(
        &mut self,
        start_height: i32,
        end_height: i32,
    ) -> PaydayResult<Vec<Transaction>> {
        Ok(self
            .lnd
            .lightning()
            .get_transactions(fedimint_tonic_lnd::lnrpc::GetTransactionsRequest {
                start_height,
                end_height,
                ..Default::default()
            })
            .await
            .map_err(|e| PaydayError::NodeApiError(e.to_string()))?
            .into_inner()
            .transactions)
    }
}

#[async_trait]
impl NodeApi for LndRpc {
    async fn get_balance(&mut self) -> PaydayResult<Balance> {
        let on_chain = self
            .lnd
            .lightning()
            .wallet_balance(WalletBalanceRequest {})
            .await
            .map_err(|e| PaydayError::NodeApiError(e.to_string()))?
            .into_inner();

        let lightning = self
            .lnd
            .lightning()
            .channel_balance(ChannelBalanceRequest {})
            .await
            .map_err(|e| PaydayError::NodeApiError(e.to_string()))?
            .into_inner();

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

    async fn new_address(&mut self) -> PaydayResult<Address> {
        let addr = self
            .lnd
            .lightning()
            .new_address(fedimint_tonic_lnd::lnrpc::NewAddressRequest {
                ..Default::default()
            })
            .await
            .map_err(|e| PaydayError::NodeApiError(e.to_string()))?
            .into_inner()
            .address;
        Ok(to_address(&addr, self.network.clone())?)
    }

    async fn get_onchain_transactions(
        &mut self,
        start_height: i32,
        end_height: i32,
    ) -> PaydayResult<Vec<OnChainTransactionEvent>> {
        Ok(self
            .get_transactions(start_height, end_height)
            .await?
            .iter()
            .map(|tx| OnChainTransactionEvent::Any(format!("{:?}", tx)))
            .collect())
    }

    async fn send_coins(
        &mut self,
        amount: Amount,
        address: String,
        sats_per_vbyte: Amount,
    ) -> PaydayResult<OnChainTransactionResult> {
        let checked_address = to_address(&address, self.network.clone())?;
        let send_coins = self
            .lnd
            .lightning()
            .send_coins(fedimint_tonic_lnd::lnrpc::SendCoinsRequest {
                addr: checked_address.to_string(),
                amount: amount.to_sat() as i64,
                sat_per_vbyte: sats_per_vbyte.to_sat(),
                ..Default::default()
            })
            .await
            .map_err(|e| PaydayError::NodeApiError(e.to_string()))?
            .into_inner();

        Ok(OnChainTransactionResult {
            tx_id: send_coins.txid,
            amount,
            fee: sats_per_vbyte,
        })
    }
    async fn subscribe_onchain_transactions(
        &mut self,
        start_height: i32,
    ) -> PaydayResult<PaydayStream<OnChainTransactionEvent>> {
        let stream = self
            .subscribe_transactions(start_height)
            .await?
            .map(|tx| OnChainTransactionEvent::Any(format!("{:?}", tx)));
        Ok(Box::pin(stream))
    }
}

fn to_amount(sats: i64) -> Amount {
    if sats < 0 {
        Amount::ZERO
    } else {
        Amount::from_sat(sats.unsigned_abs())
    }
}

fn to_address(addr: &str, network: Network) -> PaydayResult<Address> {
    Ok(Address::from_str(&addr)?.require_network(network)?)
}
