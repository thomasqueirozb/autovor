use crate::day::Day;
use crate::helper::EnsureSuccess;

use color_eyre::eyre::{ensure, eyre, ContextCompat};
use color_eyre::Result;

use once_cell::sync::Lazy;
use std::sync::Arc;

use chrono::prelude::*;
use reqwest::{Client, IntoUrl};
use reqwest_cookie_store::{CookieStore, CookieStoreMutex};
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
    cookie_store: Arc<CookieStoreMutex>,
    emulate_browser: bool,
    all_days: bool,
}

impl Session {
    pub fn new(all_days: bool, emulate_browser: bool) -> Result<Self> {
        let cookie_store = CookieStoreMutex::new(CookieStore::default());
        let cookie_store = Arc::new(cookie_store);

        let client = Client::builder()
            .cookie_provider(cookie_store.clone())
            .http1_only()
            .http1_title_case_headers()
            .base_url("https://www.endeavor.net.br/".into())
            .build()?;

        Ok(Self {
            client,
            cookie_store,
            emulate_browser,
            all_days,
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

        let cookie_store = self
            .cookie_store
            .lock()
            .or(Err(eyre!("Failed to lock cookie store")))?;

        for cookie in ["ENDEAVORu", "ENDEAVORp"] {
            let found = cookie_store.contains("www.endeavor.net.br", "/", cookie);
            ensure!(found, format!("Cookie {cookie} not found"))
        }

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

            if self.all_days || day.date <= today {
                days.push(day);
            }
        }
        days.sort_by(|a, b| a.date.cmp(&b.date));

        Ok(days)
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
