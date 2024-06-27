use async_trait::async_trait;
use bitcoin::{Address, Amount};

use crate::PaydayResult;
use crate::PaydayStream;

#[async_trait]
pub trait NodeApi {
    /// Get the current balances (onchain and lightning) of the wallet.
    async fn get_balance(&self) -> PaydayResult<Balance>;

    /// Get a new onchain address for the wallet.
    async fn new_address(&self) -> PaydayResult<Address>;

    async fn get_onchain_transactions(
        &self,
        start_height: i32,
        end_height: i32,
    ) -> PaydayResult<Vec<OnChainTransactionEvent>>;

    // async fn estimate_fee(&self, target_conf: u8, addr_to_amount: HashMap<String, u64>) -> u64;

    async fn send_coins(
        &self,
        amount: Amount,
        address: String,
        sats_per_vbyte: Amount,
    ) -> PaydayResult<OnChainTransactionResult>;

    async fn subscribe_onchain_transactions(
        &self,
        start_height: i32,
    ) -> PaydayResult<PaydayStream<OnChainTransactionEvent>>;
}

#[derive(Debug)]
pub struct OnChainBalance {
    pub total_balance: Amount,
    pub unconfirmed_balance: Amount,
    pub confirmed_balance: Amount,
}

#[derive(Debug)]
pub struct ChannelBalance {
    pub local_balance: Amount,
    pub remote_balance: Amount,
}

#[derive(Debug)]
pub struct Balance {
    pub onchain: OnChainBalance,
    pub channel: ChannelBalance,
}

#[derive(Debug, Clone)]
pub struct OnChainTransactionResult {
    pub tx_id: String,
    pub amount: Amount,
    pub fee: Amount,
}

#[derive(Debug)]
pub enum OnChainTransactionEvent {
    Any(String),
    Unconfirmed(OnChainPaymentReceived),
    Confirmed(OnChainPaymentReceived),
}

#[derive(Debug)]
pub struct OnChainPaymentReceived {
    pub tx_id: String,
    pub block_height: u64,
    pub block_hash: String,
    pub address: Address,
    pub amount: Amount,
    pub confirmations: u64,
}

/*
Any("Ok(Transaction { tx_hash: \"fb107cada18e0fe3697c8d64bb1865d32abded17baa1f3412c928b35c3dd32c8\", amount: 250000, num_confirmations: 0, block_hash: \"\", block_height: 0, time_stamp: 1719235831, total_fees: 0, dest_addresses: [\"tb1q6xm2qgh5r83lvmmu0v7c3d4wrd9k2uxu3sgcr4\", \"tb1pynnqj5yzfzwsc2f9e87h7a3zjntqc5mur3fyu9xpwl8wzuzl69pq5nenx2\"], output_details: [OutputDetail { output_type: ScriptTypeWitnessV0PubkeyHash, address: \"tb1q6xm2qgh5r83lvmmu0v7c3d4wrd9k2uxu3sgcr4\", pk_script: \"0014d1b6a022f419e3f66f7c7b3d88b6ae1b4b6570dc\", output_index: 0, amount: 250000, is_our_address: true }, OutputDetail { output_type: 9, address: \"tb1pynnqj5yzfzwsc2f9e87h7a3zjntqc5mur3fyu9xpwl8wzuzl69pq5nenx2\", pk_script: \"512024e6095082489d0c2925c9fd7f762294d60c537c1c524e14c177cee1705fd142\", output_index: 1, amount: 9749647, is_our_address: false }], raw_tx_hex: \"01000000000102da86841888297a3d92ab141ee41161fbbc1d96b7cf336afd0adfa317479a021c0100000000fdffffffb06600cf115dd061f03c24e43349ab92456910c44d883b95a0be4925bda548b60000000000fdffffff0290d0030000000000160014d1b6a022f419e3f66f7c7b3d88b6ae1b4b6570dc8fc494000000000022512024e6095082489d0c2925c9fd7f762294d60c537c1c524e14c177cee1705fd1420140d963ebc3555fcecbedd6f2195c8a8ae08564d5cfa79568891c770438b0c37cea334b711c8b88ed07d64d5d305019e08abcba65df888724ce0dc4d365081977780140516bdb467c2f8048fd1f79a825306f8af18cdd6c258f0190e842c81fe3bb9dbc5fc2d485de4040a60c3bb565323dbbb002791bf3c7a81c8d7d8a51e8540e1aab41591200\", label: \"\", previous_outpoints: [PreviousOutPoint { outpoint: \"1c029a4717a3df0afd6a33cfb7961dbcfb6111e41e14ab923d7a2988188486da:1\", is_our_output: false }, PreviousOutPoint { outpoint: \"b648a5bd2549bea0953b884dc410694592ab4933e4243cf061d05d11cf0066b0:0\", is_our_output: false }] })")
Any("Ok(Transaction { tx_hash: \"fb107cada18e0fe3697c8d64bb1865d32abded17baa1f3412c928b35c3dd32c8\", amount: 250000, num_confirmations: 1, block_hash: \"000001f05c42eb18cea11610c8b0e2c490616a88904429c7b70cd0a842afd6db\", block_height: 1202498, time_stamp: 1719235833, total_fees: 0, dest_addresses: [\"tb1q6xm2qgh5r83lvmmu0v7c3d4wrd9k2uxu3sgcr4\", \"tb1pynnqj5yzfzwsc2f9e87h7a3zjntqc5mur3fyu9xpwl8wzuzl69pq5nenx2\"], output_details: [OutputDetail { output_type: ScriptTypeWitnessV0PubkeyHash, address: \"tb1q6xm2qgh5r83lvmmu0v7c3d4wrd9k2uxu3sgcr4\", pk_script: \"0014d1b6a022f419e3f66f7c7b3d88b6ae1b4b6570dc\", output_index: 0, amount: 250000, is_our_address: true }, OutputDetail { output_type: 9, address: \"tb1pynnqj5yzfzwsc2f9e87h7a3zjntqc5mur3fyu9xpwl8wzuzl69pq5nenx2\", pk_script: \"512024e6095082489d0c2925c9fd7f762294d60c537c1c524e14c177cee1705fd142\", output_index: 1, amount: 9749647, is_our_address: false }], raw_tx_hex: \"01000000000102da86841888297a3d92ab141ee41161fbbc1d96b7cf336afd0adfa317479a021c0100000000fdffffffb06600cf115dd061f03c24e43349ab92456910c44d883b95a0be4925bda548b60000000000fdffffff0290d0030000000000160014d1b6a022f419e3f66f7c7b3d88b6ae1b4b6570dc8fc494000000000022512024e6095082489d0c2925c9fd7f762294d60c537c1c524e14c177cee1705fd1420140d963ebc3555fcecbedd6f2195c8a8ae08564d5cfa79568891c770438b0c37cea334b711c8b88ed07d64d5d305019e08abcba65df888724ce0dc4d365081977780140516bdb467c2f8048fd1f79a825306f8af18cdd6c258f0190e842c81fe3bb9dbc5fc2d485de4040a60c3bb565323dbbb002791bf3c7a81c8d7d8a51e8540e1aab41591200\", label: \"\", previous_outpoints: [PreviousOutPoint { outpoint: \"1c029a4717a3df0afd6a33cfb7961dbcfb6111e41e14ab923d7a2988188486da:1\", is_our_output: false }, PreviousOutPoint { outpoint: \"b648a5bd2549bea0953b884dc410694592ab4933e4243cf061d05d11cf0066b0:0\", is_our_output: false }] })")

Any("Transaction { tx_hash: \"cf01d4515ec40dbadd157cb945e732c58e185d7e6621cadd61218215cfa4a64a\", amount: -250156, num_confirmations: 1, block_hash: \"000001464f8a24e28e6c1cdcbd135574f06a105e047a81c8e2044edfa63db5ca\", block_height: 1204457, time_stamp: 1719296497, total_fees: 156, dest_addresses: [\"tb1pemck9xrr6s97zjn94wry22978cuc4ed9ssmav9cr3kwn5k3xakls86heem\", \"tb1pwrwjsyhgurspa7k7eqlvkphxllqh4yvz2w37hzcv0rpfnq749j2svganhr\"], output_details: [OutputDetail { output_type: 9, address: \"tb1pemck9xrr6s97zjn94wry22978cuc4ed9ssmav9cr3kwn5k3xakls86heem\", pk_script: \"5120cef1629863d40be14a65ab864528be3e398ae5a58437d617038d9d3a5a26edbf\", output_index: 0, amount: 9499688, is_our_address: true }, OutputDetail { output_type: 9, address: \"tb1pwrwjsyhgurspa7k7eqlvkphxllqh4yvz2w37hzcv0rpfnq749j2svganhr\", pk_script: \"512070dd2812e8e0e01efadec83ecb06e6ffc17a918253a3eb8b0c78c29983d52c95\", output_index: 1, amount: 250000, is_our_address: false }], raw_tx_hex: \"01000000000101b06600cf115dd061f03c24e43349ab92456910c44d883b95a0be4925bda548b60100000000ffffffff0228f4900000000000225120cef1629863d40be14a65ab864528be3e398ae5a58437d617038d9d3a5a26edbf90d003000000000022512070dd2812e8e0e01efadec83ecb06e6ffc17a918253a3eb8b0c78c29983d52c950140c25ba785486cd268911c20701db02328a39b79745423db813e4db8a3a3955d846e4955ca644a02377f7b70ca3e068322a6081cd680aae8b7293b4a8463bdc39e00000000\", label: \"external\", previous_outpoints: [PreviousOutPoint { outpoint: \"b648a5bd2549bea0953b884dc410694592ab4933e4243cf061d05d11cf0066b0:1\", is_our_output: true }] }")
 */
