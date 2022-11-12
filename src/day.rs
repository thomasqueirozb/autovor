use std::fmt;

use chrono::prelude::*;
use color_eyre::eyre::{ensure, Context, ContextCompat, Result};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Day {
    pub id: String,
    #[serde(with = "crate::helper::naive_date_serializer")]
    pub date: NaiveDate,
    pub project_number: String,
    pub customer: String,
}

impl Day {
    pub fn new(id_date: &str, mut project_info: Vec<String>) -> Result<Self> {
        let (id, date) = id_date
            .split_once(" - ")
            .wrap_err_with(|| format!("Text ' - ' not found in: {id_date}"))?;
        let date = NaiveDate::parse_from_str(date, "%d-%b-%Y")
            .wrap_err_with(|| format!("Could not parse date: {date}"))?;

        ensure!(
            project_info.len() == 2,
            "Found {} items when parsing the text for project info, expected 2\nproject_info: {project_info:?}",
            project_info.len()
        );
        let project_number = project_info.pop().unwrap();
        let customer = project_info.pop().unwrap();

        Ok(Self {
            id: id.to_string(),
            date,
            project_number,
            customer,
        })
    }
}

impl fmt::Display for Day {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let date = self.date.format("%d/%b/%Y");
        write!(
            f,
            "{} ({}) -- #{} | {}",
            date, self.customer, self.id, self.project_number,
        )
    }
}
