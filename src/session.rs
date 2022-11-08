use crate::day::Day;
use crate::helper::EnsureSuccess;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use async_scoped::TokioScope;
use chrono::prelude::*;
use color_eyre::eyre::{ensure, Context, ContextCompat};
use color_eyre::Result;
use once_cell::sync::Lazy;
use reqwest::cookie::{CookieStore, Jar};
use reqwest::{Client, IntoUrl, Url};
use scraper::{Html, Selector};

// Selectors
static DAYS_SELECTOR: Lazy<Selector> = Lazy::new(|| {
    let days_selector = "body > div.wrapper.fullheight-side > div.main-panel.full-height > \
        div.content > div > div.col-md-12 > div > div > div.d-flex";
    Selector::parse(days_selector).unwrap()
});
static DAY_DIV_SELECTOR: Lazy<Selector> =
    Lazy::new(|| Selector::parse("div.flex-1.ml-3.pt-1").unwrap());
static ID_DATE_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("h6 > b").unwrap());
static PROJECT_INFO_SELECTOR: Lazy<Selector> =
    Lazy::new(|| Selector::parse(":scope > span").unwrap());

pub struct Session {
    client: Client,
    cookie_jar: Arc<Jar>,
    emulate_browser: bool,
}

impl Session {
    pub fn new(emulate_browser: bool) -> Result<Self> {
        let cookie_jar = Arc::new(Jar::default());

        let client = Client::builder()
            .cookie_provider(cookie_jar.clone())
            .http1_only()
            .http1_title_case_headers()
            .base_url("https://www.endeavor.net.br/".into())
            .build()?;

        Ok(Self {
            client,
            cookie_jar,
            emulate_browser,
        })
    }

    pub async fn dummy_get<U: IntoUrl + Send>(&self, url: U) -> Result<()> {
        if self.emulate_browser {
            self.client.get(url).send().await?.ensure_success()?;
        }
        Ok(())
    }

    pub async fn login(&mut self, username: String, password: String) -> Result<()> {
        self.dummy_get("/horas").await?;

        self.client
            .post("/mobile_v2/login.asp?Action=Login")
            .form(&[("UserLogon", username), ("UserPwd", password)])
            .send()
            .await?
            .ensure_success_or("Login post failed")?;

        let cookies = self
            .cookie_jar
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
            "ENDEAVORu is not a cookie"
        );

        ensure!(
            cookies.contains_key("ENDEAVORp"),
            "ENDEAVORp is not a cookie"
        );
        Ok(())
    }

    pub async fn get_days(&mut self) -> Result<Vec<Day>> {
        let res = self
            .client
            .get("/mobile_v2/time_line.asp")
            .send()
            .await?
            .ensure_success_or("Failed to get timeline")?;

        let text = res.text().await?;
        let document = Html::parse_document(&text);
        let mut days = vec![];

        let today = Local::now().date_naive();
        for day in document.select(&DAYS_SELECTOR) {
            let elements = day
                .select(&DAY_DIV_SELECTOR)
                .next()
                .wrap_err("Day selection was empty")?;

            let id_date: String = elements
                .select(&ID_DATE_SELECTOR)
                .next()
                .wrap_err("Cannot find 'id - date' field")?
                .text()
                .collect();

            let project_info: Vec<String> = elements
                .select(&PROJECT_INFO_SELECTOR)
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

        ensure!(!days.is_empty(), "No days found");
        Ok(days)
    }

    pub async fn submit_multiple_simultaneously(
        &self,
        ids: Vec<String>,
        form: &[(&str, &str); 6],
    ) -> Result<()> {
        let ids: HashSet<String> = ids.into_iter().collect();

        let (_, outputs) = TokioScope::scope_and_block(|s| {
            for id in ids {
                s.spawn(async {
                    self.submit(id.clone(), form).await?;
                    Ok::<String, color_eyre::eyre::Report>(id)
                });
            }
        });

        // Error handling is lacking here since there seems to be no way of getting `outputs` in the
        // same order as the futures were spawned
        outputs
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .wrap_err("Failed to join one more more submit futures")?
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .wrap_err("Submission failed")?;

        Ok(())
    }

    // TODO make a better API. Using the 'raw' form as a parameter is ugly
    pub async fn submit(&self, id: String, form: &[(&str, &str); 6]) -> Result<()> {
        self.dummy_get(format!("/mobile_v2/tarefa.asp?app_id={id}"))
            .await?;
        self.dummy_get(format!("/mobile_v2/apontamento.asp?hist=&app_id={id}"))
            .await?;

        self.client
            .post(format!(
                "/mobile_v2/apontamento.asp?Action=Post&app_id={id}"
            ))
            .form(form)
            .send()
            .await?
            .ensure_success_or("Submit post failed")?;

        self.client
            .get(format!("/mobile_v2/finalizar.asp?app_id={id}"))
            .send()
            .await?
            .ensure_success()?;

        Ok(())
    }
}
