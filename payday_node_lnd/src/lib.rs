pub mod lnd;
pub mod wrapper;

use async_trait::async_trait;
use std::str::FromStr;

use bitcoin::{Address, Network};
use payday_core::{
    api::{
        lightining_api::LightningInvoiceApi, node_api::NodeApi, on_chain_api::OnChainInvoiceApi,
    },
    payment::{
        invoice::{InvoiceDetails, LightningInvoiceDetails, OnChainInvoiceDetails, PaymentType},
        Amount,
    },
    Error, Result,
};

pub use lnd::Lnd;

/// Given a Bitcoin address string and a network, parses and validates the address.
/// Returns a checked address result.
pub fn to_address(addr: &str, network: Network) -> Result<Address> {
    Ok(Address::from_str(addr)?.require_network(network)?)
}

#[async_trait]
impl NodeApi for Lnd {
    fn node_id(&self) -> String {
        self.config.node_id.to_owned()
    }

    fn supports_payment_types(&self, payment_type: PaymentType) -> bool {
        matches!(
            payment_type,
            PaymentType::BitcoinLightning | PaymentType::BitcoinOnChain
        )
    }

    async fn create_invoice(
        &self,
        payment_type: PaymentType,
        amount: Amount,
        memo: Option<String>,
        ttl: Option<u64>,
    ) -> Result<InvoiceDetails> {
        match payment_type {
            PaymentType::BitcoinLightning => {
                let invoice = self
                    .create_ln_invoice(amount, memo, ttl.map(|t| t as i64))
                    .await?;
                Ok(InvoiceDetails::Lightning(LightningInvoiceDetails {
                    invoice: invoice.invoice,
                }))
            }
            PaymentType::BitcoinOnChain => {
                let address = self.new_address().await?;
                Ok(InvoiceDetails::OnChain(OnChainInvoiceDetails {
                    address: address.to_string(),
                }))
            }
            #[allow(unreachable_patterns)]
            _ => Err(Error::InvalidPaymentType(format!(
                "Invalid payment type: {:?} for LND node",
                payment_type
            ))),
        }
    }
}
