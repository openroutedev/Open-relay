use clap::{Parser, Subcommand};
use serde_json::json;

#[derive(Parser)]
#[command(name = "openrelay-cli")]
#[command(about = "OpenRelay v0.3 CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Status,
    GenerateLabel {
        #[arg(long)]
        seal: String,
        #[arg(long)]
        next: String,
        #[arg(long)]
        output: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let client = reqwest::Client::new();

    match cli.command {
        Commands::Status => {
            let res = client.get("http://127.0.0.1:8080/v1/status").send().await?;
            let body: serde_json::Value = res.json().await?;
            println!("{}", serde_json::to_string_pretty(&body)?);
        }
        Commands::GenerateLabel { seal, next, output } => {
            let payload = json!({
                "commitment_hex": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                "seal_serial": seal,
                "next_staging_point": next,
                "deadline": 1770005000u64,
            });

            let res = client
                .post("http://127.0.0.1:8080/v1/label/pdf")
                .json(&payload)
                .send()
                .await?;

            let body: serde_json::Value = res.json().await?;
            if let Some(b64) = body["pdf_base64"].as_str() {
                let bytes = data_encoding::BASE64.decode(b64.as_bytes())?;
                std::fs::write(&output, bytes)?;
                println!("[✓] Saved packing slip to {}", output);
            }
        }
    }

    Ok(())
}
