use async_trait::async_trait;
use payday_btc::{
    on_chain_aggregate::{BtcOnChainInvoice, OnChainInvoiceCommand},
    on_chain_api::OnChainInvoiceApi,
    on_chain_processor::OnChainTransactionEvent,
};
use payday_core::{
    payment::{
        amount::Amount,
        currency::Currency,
        invoice::{Invoice, InvoiceId, PaymentProcessorApi, PaymentType},
    },
    PaydayError, PaydayResult,
};
use postgres_es::PostgresCqrs;
use serde_json::Value;

//pub struct OnChainProcessor {
//    name: String,
//    supported_payment_type: PaymentType,
//    on_chain_api: Box<dyn OnChainInvoiceApi>,
//    tx_stream: Box<dyn OnChainTransactionStreamSubscriber>,
//    cqrs: PostgresCqrs<BtcOnChainInvoice>,
//}
//
//impl OnChainProcessor {
//    pub fn new(
//        name: String,
//        supported_payment_type: PaymentType,
//        on_chain_api: Box<dyn OnChainInvoiceApi>,
//        tx_stream: Box<dyn OnChainTransactionStreamSubscriber>,
//        cqrs: PostgresCqrs<BtcOnChainInvoice>,
//    ) -> Self {
//        Self {
//            name,
//            supported_payment_type,
//            on_chain_api,
//            tx_stream,
//            cqrs,
//        }
//    }
//}
//
//#[async_trait]
//impl PaymentProcessorApi for OnChainProcessor {
//    fn name(&self) -> String {
//        self.name.to_owned()
//    }
//
//    fn supported_payment_type(&self) -> PaymentType {
//        self.supported_payment_type.to_owned()
//    }
//
//    async fn create_invoice(
//        &self,
//        invoice_id: InvoiceId,
//        amount: Amount,
//        _memo: Option<String>,
//    ) -> PaydayResult<Invoice> {
//        let address = self.on_chain_api.new_address().await?;
//        self.cqrs
//            .execute(
//                &address.to_string(),
//                OnChainInvoiceCommand::CreateInvoice {
//                    invoice_id: invoice_id.to_string(),
//                    amount,
//                    address: address.to_string(),
//                },
//            )
//            .await
//            .map_err(|e| PaydayError::DbError(e.to_string()))?;
//        Ok(Invoice {
//            service_name: self.name(),
//            invoice_id,
//            amount,
//            payment_type: self.supported_payment_type(),
//            payment_info: Value::String(address.to_string()),
//        })
//    }
//
//    async fn process_payment_events(&self) -> PaydayResult<()> {
//        let mut subscriber = self.tx_stream.subscribe_events()?;
//        while let Some(event) = subscriber.recv().await {
//            let (aggregate_id, command) = match event {
//                OnChainTransactionEvent::ReceivedConfirmed(tx) => (
//                    tx.address,
//                    OnChainInvoiceCommand::SetConfirmed {
//                        confirmations: tx.confirmations as u64,
//                        amount: Amount::new(Currency::Btc, tx.amount.to_sat()),
//                        transaction_id: tx.tx_id.to_owned(),
//                    },
//                ),
//                OnChainTransactionEvent::ReceivedUnconfirmed(tx) => (
//                    tx.address,
//                    OnChainInvoiceCommand::SetPending {
//                        amount: Amount::new(Currency::Btc, tx.amount.to_sat()),
//                    },
//                ),
//                OnChainTransactionEvent::SentConfirmed(tx) => (
//                    tx.address,
//                    OnChainInvoiceCommand::SetConfirmed {
//                        confirmations: tx.confirmations as u64,
//                        amount: Amount::new(Currency::Btc, tx.amount.to_sat()),
//                        transaction_id: tx.tx_id.to_owned(),
//                    },
//                ),
//                OnChainTransactionEvent::SentUnconfirmed(tx) => (
//                    tx.address,
//                    OnChainInvoiceCommand::SetPending {
//                        amount: Amount::new(Currency::Btc, tx.amount.to_sat()),
//                    },
//                ),
//            };
//            self.cqrs
//                .execute(&aggregate_id.to_string(), command)
//                .await
//                .map_err(|e| PaydayError::DbError(e.to_string()))?;
//        }
//        Ok(())
//    }
//}
