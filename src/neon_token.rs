//! Faucet NEON token module.

use eyre::{eyre, Result};
use tracing::info;

use crate::{config, ethereum, id::ReqId, solana};

/// Represents packet of information needed for single airdrop operation.
#[derive(Debug, serde::Deserialize)]
pub struct Airdrop {
    /// Ethereum address of the recipient.
    wallet: String,
    /// Amount of a token to be received.
    amount: u64,
    /// Specifies amount in whole tokens (false, default) or in 10E-9 fractions (true).
    #[serde(default)]
    pub in_fractions: bool,
}

/// Processes the airdrop: sends needed transactions into Solana.
pub async fn airdrop(id: &ReqId, params: Airdrop) -> Result<()> {
    info!("{} Processing NEON {:?}...", id, params);

    if config::solana_account_seed_version() == 0 {
        config::load_neon_params().await?;
        check_token_account(id).await?;
    }

    let limit = if !params.in_fractions {
        config::solana_max_amount()
    } else {
        solana::convert_whole_to_fractions(config::solana_max_amount())?
    };

    if params.amount > limit {
        return Err(eyre!(
            "Requested value {} exceeds the limit {}",
            params.amount,
            limit
        ));
    }

    let operator = config::solana_operator_keypair()
        .map_err(|e| eyre!("config::solana_operator_keypair: {:?}", e))?;
    let ether_address = ethereum::address_from_str(&params.wallet)
        .map_err(|e| eyre!("ethereum::address_from_str({}): {:?}", &params.wallet, e))?;
    solana::deposit_token(
        id,
        operator,
        ether_address,
        params.amount,
        params.in_fractions,
    )
    .await
    .map_err(|e| {
        eyre!(
            "solana::deposit_token(operator, {}): {:?}",
            ether_address,
            e
        )
    })?;
    Ok(())
}

/// Checks existence and balance of the operator's token account.
async fn check_token_account(id: &ReqId) -> Result<()> {
    use eyre::WrapErr as _;
    use solana_account_decoder::parse_token::UiTokenAmount;
    use solana_client::client_error::Result as ClientResult;
    use solana_client::rpc_client::RpcClient;
    use solana_sdk::pubkey::Pubkey;
    use solana_sdk::signature::Signer as _;
    use std::str::FromStr as _;

    let operator = config::solana_operator_keypair()
        .map_err(|e| eyre!("config::solana_operator_keypair: {:?}", e))?;
    let operator_pubkey = operator.pubkey();

    let token_mint_id = Pubkey::from_str(&config::solana_token_mint_id()).wrap_err_with(|| {
        eyre!(
            "config::solana_token_mint_id returns {}",
            &config::solana_token_mint_id(),
        )
    })?;

    let operator_token_pubkey = spl_associated_token_account::get_associated_token_address(
        &operator_pubkey,
        &token_mint_id,
    );

    info!("{} Token account: {}", id, operator_token_pubkey);
    let r = tokio::task::spawn_blocking(move || -> ClientResult<UiTokenAmount> {
        let client =
            RpcClient::new_with_commitment(config::solana_url(), config::solana_commitment());
        client.get_token_account_balance(&operator_token_pubkey)
    })
    .await??;

    let amount = r.ui_amount.unwrap_or_default();
    if amount <= f64::default() {
        return Err(eyre!(
            "Account {} has zero token balance {}",
            operator_token_pubkey,
            amount
        ));
    }

    Ok(())
}
