mod frequency;
mod importer;
mod yomitan;

use clap::Parser;
use importer::import_words;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "seed", about = "Seed database with Yomitan dictionary")]
struct Args {
    /// Path to Yomitan dictionary ZIP (e.g., jitendex-yomitan.zip)
    #[arg(short, long)]
    dict: PathBuf,

    /// Path to frequency dictionary ZIP (optional)
    #[arg(short, long)]
    freq: Option<PathBuf>,

    /// SQLite database URL
    #[arg(long, env = "DATABASE_URL")]
    database_url: String,

    /// Maximum number of words to import
    #[arg(short, long)]
    limit: Option<usize>,

    /// Only include words with frequency rank <= this value
    #[arg(long)]
    max_rank: Option<u32>,

    /// Clear existing words before import
    #[arg(long)]
    clear: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    println!("Connecting to database...");
    let pool = SqlitePool::connect(&args.database_url).await?;

    // Run migrations to ensure schema exists
    sqlx::migrate!("../../backend/migrations").run(&pool).await?;

    if args.clear {
        println!("Clearing existing words...");
        sqlx::query("DELETE FROM words").execute(&pool).await?;
    }

    println!("Parsing dictionary: {:?}", args.dict);
    let entries = yomitan::parse_dictionary(&args.dict)?;
    println!("Found {} term entries", entries.len());

    let frequency = match &args.freq {
        Some(path) => {
            println!("Parsing frequency data: {:?}", path);
            let freq = frequency::parse_frequency(path)?;
            println!("Found frequency data for {} terms", freq.len());
            freq
        }
        None => {
            println!("No frequency dictionary provided, using empty frequency map");
            HashMap::new()
        }
    };

    println!("Importing words...");
    let stats = import_words(&pool, entries, &frequency, args.limit, args.max_rank).await?;

    println!();
    println!("Import complete:");
    println!("  Filtered (passed all checks): {}", stats.filtered);
    println!("  Inserted into database:       {}", stats.inserted);
    println!("  Skipped (no kanji/freq):      {}", stats.skipped);

    Ok(())
}
