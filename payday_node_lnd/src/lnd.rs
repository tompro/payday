use async_trait::async_trait;
use bitcoin::{Address, Amount, Network};

use payday_btc::{
    on_chain_api::{
        Balance, ChannelBalance, OnChainApi, OnChainBalance, OnChainTransactionEvent,
        OnChainTransactionResult,
    },
    to_address,
};
use payday_core::{PaydayResult, PaydayStream};
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
        Ok(self
            .client
            .get_transactions(start_height, end_height)
            .await?
            .iter()
            .map(|tx| OnChainTransactionEvent::Any(format!("{:?}", tx)))
            .collect())
    }

    async fn send_coins(
        &self,
        amount: Amount,
        address: String,
        sats_per_vbyte: Amount,
    ) -> PaydayResult<OnChainTransactionResult> {
        let tx_id = self
            .client
            .send_coins(amount, &address, sats_per_vbyte)
            .await?;

        Ok(OnChainTransactionResult {
            tx_id,
            amount,
            fee: sats_per_vbyte,
        })
    }
    async fn subscribe_onchain_transactions(
        &self,
    ) -> PaydayResult<PaydayStream<OnChainTransactionEvent>> {
        let stream = self
            .client
            .subscribe_transactions()
            .await?
            .map(|tx| OnChainTransactionEvent::Any(format!("{:?}", tx)));
        Ok(Box::pin(stream))
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

fn to_amount(sats: i64) -> Amount {
    if sats < 0 {
        Amount::ZERO
    } else {
        Amount::from_sat(sats.unsigned_abs())
    }
}
