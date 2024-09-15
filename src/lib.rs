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
    let mut value_string = value_js_result.value.unwrap().to_string();

    // Remove leading and trailing quotes
    if value_string.starts_with('"') && value_string.ends_with('"') {
        value_string.remove(0);
        value_string.pop();
    }

    println!("value_string: {}", value_string);
    Ok(value_string)
}
