use std::fmt::Result;

use chttp::ResponseExt;
use clap::Parser;
use chrono::prelude::*;
use roxmltree::Document;

#[derive(Parser, Debug)]
#[command(
    name = "csv",
    version = "0.1",
    about = "Fetches exchange rates from HMRC in CSV for a given tax year",
    next_display_order = None
)]
struct Args {
    #[arg(short, long)]
    currency: String,

    #[arg(short, long)]
    year: u16,

    /// Get data for a fiscal yeat (April to April) rather than solar year (January to December)
    #[arg(short, long)]
    fiscal_year: bool,
}

#[tokio::main]
async fn main() -> Result {
    let args: Args = Args::parse();

    let year = args.year;
    let currency = args.currency;
    let use_fiscal_year = args.fiscal_year;

    let month_year_pairs = if use_fiscal_year {
        vec![
            (4, year),
            (5, year),
            (6, year),
            (7, year),
            (8, year),
            (9, year),
            (10, year),
            (11, year),
            (12, year),
            (1, year + 1),
            (2, year + 1),
            (3, year + 1)
        ]
    } else {
        vec![
            (1, year),
            (2, year),
            (3, year),
            (4, year),
            (5, year),
            (6, year),
            (7, year),
            (8, year),
            (9, year),
            (10, year),
            (11, year),
            (12, year)
        ]
    };

    let mut rates: Vec<(String, String)> = Vec::new();

    // tax year begins in April
    for (month, year) in month_year_pairs {
        let dt = Utc.with_ymd_and_hms(year as i32, month, 1, 0, 0, 0).unwrap();
        let endpoint = format!(
            "https://www.trade-tariff.service.gov.uk/api/v2/exchange_rates/files/monthly_xml_{}.xml",
            dt.format("%Y-%m")
        );

        let request = chttp::get_async(endpoint).await;
        match request {
            Ok(mut response) => {
                let body = response.text_async().await.unwrap();
                let doc = Document::parse(&body).unwrap();
                let elem = doc
                    .descendants()
                    .find(
                        |n|
                            n.tag_name().name() == "currencyCode" &&
                            n.text().unwrap_or("") == currency
                    );
                match elem {
                    Some(elem) => {
                        let rate = elem
                            .parent()
                            .unwrap()
                            .descendants()
                            .find(|n| n.tag_name().name() == "rateNew")
                            .unwrap()
                            .text()
                            .unwrap();
                        rates.push((dt.format("%m/%Y").to_string(), String::from(rate)));
                    }
                    None => {
                        println!("No exchange rate found for {}-{}", year, month);
                        return Ok(());
                    }
                }
            }
            Err(e) => {
                println!("Error fetching exchange rates: {}", e);
                return Ok(());
            }
        }
    }

    let current_dir = std::env::current_dir().unwrap();
    let mut csv_writer = csv::Writer
        ::from_path(current_dir.join(format!("exchange-rates-{}-{}", currency, year)))
        .unwrap();

    csv_writer.write_record(&["Date", "Rate", "Currency"]).unwrap();

    for (date, rate) in rates {
        csv_writer.write_record(&[date, rate, currency.clone()]).unwrap();
    }

    match csv_writer.flush() {
        Ok(_) => println!("Exchange rates written to file"),
        Err(e) => println!("Error writing exchange rates to file: {}", e),
    }

    Ok(())
}
