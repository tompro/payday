use std::str::FromStr;

use async_trait::async_trait;
use bitcoin::{Address, Amount, Network};
use fedimint_tonic_lnd::lnrpc::{ChannelBalanceRequest, GetInfoRequest, WalletBalanceRequest};

use payday_core::error::{PaydayError, PaydayResult};
use payday_core::node::node_api::{Balance, ChannelBalance, NodeApi, OnChainBalance};

pub struct LndRpc {
    network: Network,
    lnd: fedimint_tonic_lnd::Client,
}

impl LndRpc {
    pub async fn new(
        address: String,
        cert_file: String,
        macaroon_file: String,
        allowed_network: Network,
    ) -> PaydayResult<Self> {
        let mut lnd = fedimint_tonic_lnd::connect(address, cert_file, macaroon_file)
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

        Ok(Address::from_str(&addr)?.require_network(self.network.clone())?)
    }
}

fn to_amount(sats: i64) -> Amount {
    if sats < 0 {
        Amount::ZERO
    } else {
        Amount::from_sat(sats.unsigned_abs())
    }
}
