#![warn(clippy::pedantic, clippy::nursery, clippy::style, clippy::cargo)]
#![allow(clippy::multiple_crate_versions)]

mod day;
use day::Day;

mod session;
use session::Session;

mod helper;

use inquire::InquireError;
use inquire::{formatter::MultiOptionFormatter, MultiSelect};

use std::path::PathBuf;

use clap::Parser;
use color_eyre::eyre::{ensure, Context, Result};

/// CLI for Endeavor
#[allow(clippy::struct_excessive_bools)]
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Credentials file path
    ///
    /// This file must consist of ONLY 2 lines.
    /// The first line should be the username and the second line the password.
    /// Whitespace in the start or end of lines is ignored.
    #[clap(short, long, value_parser, default_value = "creds.txt")]
    creds_path: PathBuf,

    /// Default behavior is to not show days in the future.
    #[clap(short, long)]
    all_days: bool,

    /// If specified, unecessary requests will be made so that it better emulates
    /// what a browser actually would do.
    #[clap(long)]
    emulate_browser: bool,

    /// [NON-INTERACTIVE] Get days output as JSON
    #[clap(long, conflicts_with = "get_days_csv")]
    get_days_json: bool,

    /// [NON-INTERACTIVE] Get days output as CSV
    #[clap(long, conflicts_with = "get_days_json")]
    get_days_csv: bool,

    /// [NON-INTERACTIVE] Submit IDs. Comma separated list of IDs
    #[clap(long, value_delimiter = ',', conflicts_with_all = ["get_days_json", "get_days_csv"])]
    submit_ids: Option<Vec<String>>,
    /*

    /// Don't ask for user input (NOT RECOMENDED)
    #[clap(long)]
    automatic: bool,
    */
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let args = Args::parse();

    let mut creds = std::fs::read_to_string(&args.creds_path)?
        .lines()
        .map(String::from)
        .map(|s| s.trim().to_string())
        .collect::<Vec<String>>();
    ensure!(
        creds.len() == 2,
        "{} must have exactly 2 lines",
        args.creds_path.to_string_lossy()
    );

    let (password, username) = (creds.pop().unwrap(), creds.pop().unwrap());

    let mut session = Session::new(args.all_days, args.emulate_browser)?;

    session
        .login(username.clone(), password.clone())
        .await
        .wrap_err("Failed to login")?;

    // Check non-interactive flags
    if let Some(submit_ids) = args.submit_ids {
        for id in submit_ids {
            if id.is_empty() {
                continue;
            }

            println!("Submitting id: {id}");
            // TODO don't hard code this
            let form = &[
                ("horas_normais_horas", "8"),
                ("horas_normais_minutos", "00"),
                ("horas_extras_horas", "00"),
                ("horas_extras_minutos", "00"),
                ("horas_dobro_horas", "00"),
                ("horas_dobro_minutos", "00"),
            ];

            session
                .submit(id.clone(), form)
                .await
                .wrap_err_with(|| format!("Failed to submit hours for id: {id}"))?;
        }
        return Ok(());
    }

    let days = session.get_days().await?;

    if args.get_days_json {
        println!("{}", serde_json::to_string(&days)?);
        return Ok(());
    } else if args.get_days_csv {
        let mut writer = csv::Writer::from_writer(vec![]);
        for day in days {
            writer.serialize(day)?;
        }

        println!("{}", String::from_utf8(writer.into_inner()?)?);
        return Ok(());
    }
    ensure!(!days.is_empty(), "No days found");

    let formatter: MultiOptionFormatter<Day> =
        &|day_list| format!("Selected days: {}", day_list.len());

    let ans = MultiSelect::new("Select days:", days)
        .with_formatter(formatter)
        .prompt();

    let days = match ans {
        Ok(days) => days,
        Err(inquire_error) => {
            return match inquire_error {
                InquireError::OperationCanceled | InquireError::OperationInterrupted => {
                    println!("Selection canceled");
                    Ok(())
                }
                _ => Err(inquire_error).wrap_err("Selection failed"),
            }
        }
    };

    for day in days {
        println!("Submitting: {day}");
        let id = &day.id;

        // TODO don't hard code this
        let form = &[
            ("horas_normais_horas", "8"),
            ("horas_normais_minutos", "00"),
            ("horas_extras_horas", "00"),
            ("horas_extras_minutos", "00"),
            ("horas_dobro_horas", "00"),
            ("horas_dobro_minutos", "00"),
        ];

        session
            .submit(id.clone(), form)
            .await
            .wrap_err_with(|| format!("Failed to submit hours for {day}"))?;
    }

    Ok(())
}
