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

    /// If specified, unecessary requests will be made so that it better emulates
    /// what a browser actually would do.
    #[clap(long)]
    emulate_browser: bool,
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

    let mut session = Session::new(args.emulate_browser)?;

    session
        .login(username.clone(), password.clone())
        .await
        .wrap_err("Failed to login")?;

    let days = session.get_days().await?;

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
        println!("Submitting: {}", day);
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
            .wrap_err_with(|| format!("Failed to submit hours for {}", day))?;
    }

    Ok(())
}
