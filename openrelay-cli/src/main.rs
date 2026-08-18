use clap::{Parser, Subcommand};
use ed25519_dalek::{Signer, SigningKey};
use openrelay_crypto::identity::NodeIdentity;
use std::fs;
use std::path::Path;

#[derive(Parser)]
#[command(name = "openrelay-cli")]
#[command(about = "OpenRelay Command Line Protocol Client", version = "0.3.0")]
struct Cli {
    #[arg(short, long, default_value = "http://127.0.0.1:8080")]
    daemon_url: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a self-sovereign cryptographic identity on disk
    InitId {
        #[arg(short, long, default_value = "node_identity.json")]
        output: String,
    },
    /// Check the status of the local daemon node
    Status,
    /// Generate a cryptographic package routing label PDF
    GenerateLabel {
        #[arg(long)]
        seal: String,
        #[arg(long)]
        next: String,
        #[arg(long, default_value = "label.pdf")]
        output: String,
    },
    /// Submit a receiver pickup request to the network
    CreateRequest {
        #[arg(long, default_value = "STORE_PICKUP")]
        request_type: String,
        #[arg(long)]
        pickup: String,
        #[arg(long)]
        dropoff: String,
        #[arg(long)]
        item: String,
        #[arg(long)]
        amount: f64,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let client = reqwest::Client::new();

    match cli.command {
        Commands::InitId { output } => {
            if Path::new(&output).exists() {
                println!("[!] Identity file '{}' already exists. Aborting to prevent overwrite.", output);
                return Ok(());
            }

            let id = NodeIdentity::generate();
            let secret_bytes = id.signing_key.to_bytes();
            let hex_secret = hex::encode(secret_bytes);

            let json_data = serde_json::json!({
                "node_id": id.node_id(),
                "signing_key_hex": hex_secret
            });

            fs::write(&output, serde_json::to_string_pretty(&json_data)?)?;
            println!("[+] Successfully generated self-sovereign identity!");
            println!("[+] Account ID / Node ID: {}", id.node_id());
            println!("[+] Saved securely to: {}", output);
        }
        Commands::Status => {
            let url = format!("{}/v1/status", cli.daemon_url);
            let res = client.get(&url).send().await?.text().await?;
            println!("{}", res);
        }
        Commands::GenerateLabel { seal, next, output } => {
            let url = format!("{}/v1/label/pdf", cli.daemon_url);
            let payload = serde_json::json!({
                "commitment_hex": hex::encode(blake3::hash(seal.as_bytes()).as_bytes()),
                "seal_serial": seal,
                "next_staging_point": next,
                "deadline": 1800000000u64
            });

            let res = client.post(&url).json(&payload).send().await?;
            let json: serde_json::Value = res.json().await?;

            if let Some(b64) = json["pdf_base64"].as_str() {
                let bytes = data_encoding::BASE64.decode(b64.as_bytes())?;
                fs::write(&output, bytes)?;
                println!("[+] Successfully rendered and saved routing slip to: {}", output);
            } else {
                println!("[-] Error rendering label: {:?}", json);
            }
        }
        Commands::CreateRequest { request_type, pickup, dropoff, item, amount } => {
            let url = format!("{}/v1/requests", cli.daemon_url);
            
            let mut node_id_sig = "UNSIGNED_CLI_USER".to_string();
            if Path::new("node_identity.json").exists() {
                if let Ok(file_content) = fs::read_to_string("node_identity.json") {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&file_content) {
                        if let Some(hex_key) = json["signing_key_hex"].as_str() {
                            if let Ok(bytes) = hex::decode(hex_key) {
                                if bytes.len() == 32 {
                                    let mut arr = [0u8; 32];
                                    arr.copy_from_slice(&bytes);
                                    let sk = SigningKey::from_bytes(&arr);
                                    let signature = sk.sign(item.as_bytes());
                                    node_id_sig = format!("SIG-ED25519:{}", hex::encode(signature.to_bytes()));
                                }
                            }
                        }
                    }
                }
            }

            let payload = serde_json::json!({
                "request_type": request_type,
                "dropoff_mode": "IN_PERSON_HANDOFF",
                "pickup_location": pickup,
                "item_description": item,
                "dropoff_location": dropoff,
                "payment_spec": {
                    "amount_prompt": format!("${:.2}", amount),
                    "accepted_methods": ["CashOnHandoff"],
                    "is_settled": false
                },
                "payment_amount_num": amount,
                "ttl_seconds": 3600,
                "signature": node_id_sig
            });

            let res = client.post(&url).json(&payload).send().await?.text().await?;
            println!("{}", res);
        }
    }

    Ok(())
}
