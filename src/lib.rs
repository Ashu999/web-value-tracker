#![warn(clippy::all, rust_2018_idioms)]
mod app;
pub use app::ThisApp;

use headless_chrome::{Browser, LaunchOptions};
use regex::Regex;
use std::error::Error;

pub async fn get_current_value(
    url: &str,
    css_selector: &str,
) -> Result<Option<String>, Box<dyn Error>> {
    let browser = Browser::new(LaunchOptions {
        headless: true,
        ..Default::default()
    })?;

    let tab = browser.new_tab()?;
    tab.disable_debugger()?;

    // Navigate to the URL
    tab.navigate_to(url)?;

    let _e = tab.wait_for_element(css_selector)?;
    // println!("e: {:?}\n", e.attributes);

    // Execute JavaScript to get the price
    let price_js_result = tab.evaluate(
        &format!(
            r#"
        document.querySelector({:?})?.textContent
        "#,
            css_selector
        ),
        true,
    )?;

    // Extract the price from the JavaScript result
    if let Some(price_str) = price_js_result.value.unwrap().as_str() {
        // Remove commas from the price string
        let re = Regex::new(r",").unwrap();
        let price_without_commas = re.replace_all(price_str, "").to_string();
        Ok(Some(price_without_commas))
    } else {
        Ok(None)
    }
}
