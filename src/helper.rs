use reqwest::Response;
use std::fmt::Display;

use color_eyre::eyre::{ensure, Result};

pub trait EnsureSuccess {
    fn ensure_success(self) -> Result<Self>
    where
        Self: Sized;
    fn ensure_success_or<D>(self, msg: D) -> Result<Self>
    where
        D: Display + Send + Sync + 'static,
        Self: Sized;
}

impl EnsureSuccess for Response {
    fn ensure_success(self) -> Result<Self> {
        let status = self.status();
        ensure!(
            status.is_success(),
            "{} returned HTTP status code {}",
            self.url().as_str(),
            status.as_str()
        );
        Ok(self)
    }

    fn ensure_success_or<D>(self, msg: D) -> Result<Self>
    where
        D: Display + Send + Sync + 'static,
    {
        let status = self.status();
        ensure!(
            status.is_success(),
            "{}\n{} returned HTTP status code {}",
            msg,
            self.url().as_str(),
            status.as_str()
        );
        Ok(self)
    }
}

pub mod naive_date_serializer {
    use chrono::NaiveDate;
    use serde::Serializer;

    pub fn serialize<S>(date: &NaiveDate, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format("%Y/%m/%d"));
        serializer.serialize_str(&s)
    }
}
