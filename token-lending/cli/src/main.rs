use spl_token_lending::state::LendingMarket;
use {
    clap::{
        crate_description, crate_name, crate_version, value_t, value_t_or_exit, App, AppSettings,
        Arg, ArgGroup, SubCommand,
    },
    solana_clap_utils::{
        input_parsers::pubkey_of,
        input_validators::{is_amount, is_keypair, is_parsable, is_pubkey, is_url},
        keypair::signer_from_path,
    },
    solana_client::rpc_client::RpcClient,
    solana_program::{
        borsh::get_packed_len, instruction::Instruction, program_pack::Pack, pubkey::Pubkey,
    },
    solana_sdk::{
        commitment_config::CommitmentConfig,
        native_token::{self, Sol},
        signature::{Keypair, Signer},
        system_instruction,
        transaction::Transaction,
    },
    spl_token_lending::{self},
    std::process::exit,
};

struct Config {
    rpc_client: RpcClient,
    verbose: bool,
    payer: Box<dyn Signer>,
    dry_run: bool,
}

type Error = Box<dyn std::error::Error>;
type CommandResult = Result<(), Error>;

fn check_payer_balance(config: &Config, required_balance: u64) -> Result<(), Error> {
    let balance = config.rpc_client.get_balance(&config.payer.pubkey())?;
    if balance < required_balance {
        Err(format!(
            "Fee payer, {}, has insufficient balance: {} required, {} available",
            config.payer.pubkey(),
            Sol(required_balance),
            Sol(balance)
        )
        .into())
    } else {
        Ok(())
    }
}

fn send_transaction(
    config: &Config,
    transaction: Transaction,
) -> solana_client::client_error::Result<()> {
    if config.dry_run {
        let result = config.rpc_client.simulate_transaction(&transaction)?;
        println!("Simulate result: {:?}", result);
    } else {
        let signature = config
            .rpc_client
            .send_and_confirm_transaction_with_spinner(&transaction)?;
        println!("Signature: {}", signature);
    }
    Ok(())
}

fn command_create_lending_market(
    config: &Config,
    lending_market_owner: Pubkey,
    quote_token_mint: Pubkey,
) -> CommandResult {
    let lending_market_keypair = Keypair::new();
    println!(
        "Creating lending market {}",
        lending_market_keypair.pubkey()
    );

    let lending_market_balance = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(LendingMarket::LEN)?;

    let mut transaction = Transaction::new_with_payer(
        &[
            // Account for the lending market
            system_instruction::create_account(
                &config.payer.pubkey(),
                &lending_market_keypair.pubkey(),
                lending_market_balance,
                LendingMarket::LEN as u64,
                &spl_token_lending::id(),
            ),
            // Initialize lending market account
            spl_token_lending::instruction::init_lending_market(
                spl_token_lending::id(),
                lending_market_keypair.pubkey(),
                lending_market_owner,
                quote_token_mint,
            ),
        ],
        Some(&config.payer.pubkey()),
    );

    let (recent_blockhash, fee_calculator) = config.rpc_client.get_recent_blockhash()?;
    check_payer_balance(
        config,
        lending_market_balance + fee_calculator.calculate_fee(&transaction.message()),
    )?;
    transaction.sign(
        &vec![config.payer.as_ref(), &lending_market_keypair],
        recent_blockhash,
    );
    send_transaction(&config, transaction)?;
    Ok(())
}

const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

fn main() {
    solana_logger::setup_with_default("solana=info");

    let matches = App::new(crate_name!())
        .about(crate_description!())
        .version(crate_version!())
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .arg({
            let arg = Arg::with_name("config_file")
                .short("C")
                .long("config")
                .value_name("PATH")
                .takes_value(true)
                .global(true)
                .help("Configuration file to use");
            if let Some(ref config_file) = *solana_cli_config::CONFIG_FILE {
                arg.default_value(&config_file)
            } else {
                arg
            }
        })
        .arg(
            Arg::with_name("verbose")
                .long("verbose")
                .short("v")
                .takes_value(false)
                .global(true)
                .help("Show additional information"),
        )
        .arg(
            Arg::with_name("dry_run")
                .long("dry-run")
                .takes_value(false)
                .global(true)
                .help("Simulate transaction instead of executing"),
        )
        .arg(
            Arg::with_name("json_rpc_url")
                .long("url")
                .value_name("URL")
                .takes_value(true)
                .validator(is_url)
                .help("JSON RPC URL for the cluster.  Default from the configuration file."),
        )
        .arg(
            Arg::with_name("payer")
                .long("payer")
                .value_name("KEYPAIR")
                .validator(is_keypair)
                .takes_value(true)
                .help(
                    "Specify the payer account. \
                     This may be a keypair file, or the ASK keyword. \
                     Defaults to the client keypair.",
                ),
        )
        .subcommand(
            SubCommand::with_name("create-market")
                .about("Create a new lending market")
                .arg(
                    Arg::with_name("lending_market_owner")
                        .index(1)
                        .long("owner")
                        .short("o")
                        .validator(is_pubkey)
                        .value_name("OWNER_ADDRESS")
                        .takes_value(true)
                        .required(true)
                        .help("Owner required to sign when adding reserves to the lending market"),
                )
                .arg(
                    Arg::with_name("quote_token_mint")
                        .index(2)
                        .long("quote")
                        .short("q")
                        .validator(is_pubkey)
                        .value_name("MINT_ADDRESS")
                        .takes_value(true)
                        .required(true)
                        .default_value(USDC_MINT)
                        .help("SPL Token mint that reserve currency prices are quoted against, defaulting to USDC"),
                ),
        )
        .get_matches();

    let mut wallet_manager = None;
    let config = {
        let cli_config = if let Some(config_file) = matches.value_of("config_file") {
            solana_cli_config::Config::load(config_file).unwrap_or_default()
        } else {
            solana_cli_config::Config::default()
        };
        let json_rpc_url = value_t!(matches, "json_rpc_url", String)
            .unwrap_or_else(|_| cli_config.json_rpc_url.clone());

        let payer = signer_from_path(
            &matches,
            &cli_config.keypair_path,
            "payer",
            &mut wallet_manager,
        )
        .unwrap_or_else(|e| {
            eprintln!("error: {}", e);
            exit(1);
        });
        let verbose = matches.is_present("verbose");
        let dry_run = matches.is_present("dry_run");

        Config {
            rpc_client: RpcClient::new_with_commitment(json_rpc_url, CommitmentConfig::confirmed()),
            verbose,
            payer,
            dry_run,
        }
    };

    let _ = match matches.subcommand() {
        ("create-market", Some(arg_matches)) => {
            let lending_market_owner = pubkey_of(arg_matches, "lending_market_owner").unwrap();
            let quote_token_mint = pubkey_of(arg_matches, "quote_token_mint").unwrap();
            command_create_lending_market(&config, lending_market_owner, quote_token_mint)
        }
        _ => unreachable!(),
    }
    .map_err(|err| {
        eprintln!("{}", err);
        exit(1);
    });
}
