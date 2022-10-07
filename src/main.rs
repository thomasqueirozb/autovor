mod day;
use day::Day;

mod helper;
use helper::EnsureSuccess;

use inquire::{formatter::MultiOptionFormatter, MultiSelect};
use reqwest::cookie::{CookieStore, Jar};
use reqwest::{Client, Url};

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use chrono::prelude::*;
use clap::Parser;
use color_eyre::eyre::{ensure, Context, ContextCompat, Result};
use scraper::{Html, Selector};

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

    /// Don't ask for user input (NOT RECOMENDED)
    #[clap(long)]
    automatic: bool,
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

    let cookie_jar = Arc::new(Jar::default());

    let client = Client::builder()
        .cookie_provider(cookie_jar.clone())
        .http1_only()
        .http1_title_case_headers()
        .base_url("https://www.endeavor.net.br/".into())
        .build()?;

    macro_rules! dummy_get {
        ($url:expr) => {
            if args.emulate_browser {
                client.get($url).send().await?.ensure_success()?;
            }
        };
    }

    dummy_get!("/horas");

    client
        .post("/mobile_v2/login.asp?Action=Login")
        .form(&[("UserLogon", username), ("UserPwd", password)])
        .send()
        .await?
        .ensure_success_or("Failed to login")?;

    // dbg!(&cookie_jar);
    let cookies = cookie_jar
        .cookies(&Url::parse("https://www.endeavor.net.br").unwrap())
        .wrap_err("Could not find cookies for www.endeavor.net.br after login")?;

    // NOTE: the cookie jar trait doesn't define an easy way to get a HashMap of cookies or even to
    //       get the value of a specific cookie. Therefore the following is needed

    let cookies = cookies
        .to_str()
        .wrap_err("Cannot convert cookies to string")?;

    let cookies: HashMap<&str, &str> = cookies
        .split("; ") // Cookies cannot contain ';' or ' ' in them, so splitting is fine
        .map(|pair| pair.split_once('=').unwrap()) // Implementation always adds a '='
        .collect();

    ensure!(
        cookies.contains_key("ENDEAVORu"),
        "Authentication failed: ENDEAVORu is not a cookie"
    );

    ensure!(
        cookies.contains_key("ENDEAVORp"),
        "Authentication failed: ENDEAVORp is not a cookie"
    );

    // dbg!(cookies);
    let res = client
        .get("/mobile_v2/time_line.asp")
        .send()
        .await?
        .ensure_success_or("Failed to get timeline")?;

    let text = res.text().await?;
    let document = Html::parse_document(&text);

    let days_selector = "body > div.wrapper.fullheight-side > div.main-panel.full-height > \
        div.content > div > div.col-md-12 > div > div > div.d-flex";
    let days_selector = Selector::parse(days_selector).unwrap();

    let day_div_selector = Selector::parse("div.flex-1.ml-3.pt-1").unwrap();

    let id_date_selector = Selector::parse("h6 > b").unwrap();
    let project_info_selector = Selector::parse(":scope > span").unwrap();

    let days = {
        let mut days = vec![];

        let today = Local::now().date_naive();
        for day in document.select(&days_selector) {
            let elements = day
                .select(&day_div_selector)
                .next()
                .wrap_err("Day selection was empty")?;

            let id_date: String = elements
                .select(&id_date_selector)
                .next()
                .wrap_err("Cannot find 'id - date' field")?
                .text()
                .collect();

            let project_info: Vec<String> = elements
                .select(&project_info_selector)
                .next()
                .wrap_err("Cannot find project field")?
                .text()
                .collect::<Vec<&str>>()
                .into_iter()
                .map(String::from)
                .collect();

            let day = Day::new(&id_date, project_info)?;

            // TODO add option to disable this check
            if day.date <= today {
                days.push(day);
            }
        }
        days.sort_by(|a, b| a.date.cmp(&b.date));
        days
    };

    ensure!(!days.is_empty(), "No days found");

    let formatter: MultiOptionFormatter<Day> = &|a| format!("Selected days: {}", a.len());

    let ans = MultiSelect::new("Select days:", days)
        .with_formatter(formatter)
        .prompt();

    let days = ans.wrap_err("Error selecting days")?;

    for day in days {
        println!("Submitting: {}", day);
        let id = day.id;

        dummy_get!(format!("/mobile_v2/tarefa.asp?app_id={id}"));
        dummy_get!(format!("/mobile_v2/apontamento.asp?hist=&app_id={id}"));

        client
            .post(format!(
                "/mobile_v2/apontamento.asp?Action=Post&app_id={id}"
            ))
            .form(&[
                ("horas_normais_horas", "8"),
                ("horas_normais_minutos", "00"),
                ("horas_extras_horas", "00"),
                ("horas_extras_minutos", "00"),
                ("horas_dobro_horas", "00"),
                ("horas_dobro_minutos", "00"),
            ])
            .send()
            .await?
            .ensure_success_or("Failed to submit hours")?;

        client
            .get(format!("/mobile_v2/finalizar.asp?app_id={id}"))
            .send()
            .await?
            .ensure_success()?;
    }

    Ok(())
}
