#![warn(clippy::all, rust_2018_idioms)]
mod app;
pub use app::ThisApp;

use headless_chrome::{Browser, LaunchOptions};
use std::error::Error;

pub async fn get_current_value(url: &str, css_selector: &str) -> Result<String, Box<dyn Error>> {
    let browser = Browser::new(LaunchOptions {
        headless: true,
        ..Default::default()
    })?;

    let tab = browser.new_tab()?;
    tab.disable_debugger()?;

    // Navigate to the URL
    tab.navigate_to(url)?;

    let _e = tab.wait_for_element(css_selector)?;

    // Execute JavaScript to get the value
    // let value_js_result = tab.evaluate(
    //     &format!(
    //         r#"
    //     document.querySelector({:?})?.innerText.trim()
    //     "#,
    //         css_selector
    //     ),
    //     true,
    // )?;
    let value_js_result = tab.evaluate(
        &format!(
            r#"
        document.querySelector({:?})?.textContent
        "#,
            css_selector
        ),
        true,
    )?;

    // Extract the value string from the JavaScript result
    let value_string = value_js_result.value.unwrap().to_string();

    println!("value_string: {}", value_string);
    Ok(value_string)
}
