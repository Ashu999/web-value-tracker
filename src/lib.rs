mod app;
pub use app::ThisApp;

use headless_chrome::{Browser, LaunchOptions};
use notify_rust::{Notification, Timeout};
use poll_promise::Promise;
use std::{collections::VecDeque, error::Error};

async fn get_current_value(url: &str, css_selector: &str) -> Result<String, Box<dyn Error>> {
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
    println!("value_string: {}", value_string);

    // Remove leading and trailing quotes
    if value_string.starts_with('"') && value_string.ends_with('"') {
        value_string.remove(0);
        value_string.pop();
    }

    Ok(value_string)
}

fn get_web_value_promise(
    id: String,
    link: String,
    css_selector: String,
) -> Promise<(String, String)> {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    Promise::spawn_blocking(move || {
        let result = runtime.block_on(async {
            crate::get_current_value(&link, &css_selector)
                .await
                .unwrap_or_default()
        });
        (id, result)
    })
}

fn show_notifcation(name: &String, old_value: &String, new_value: &String) {
    Notification::new()
        .summary(format!("Web value tracker").as_str())
        .body(
            format!(
                "Value of: {:?} changed from: {:?} to: {:?}\nAt time: {:?}",
                name,
                old_value,
                new_value,
                crate::get_current_date_time(),
            )
            .as_str(),
        )
        .timeout(Timeout::Never) // this however is
        .show()
        .unwrap();
}

fn get_current_date_time() -> String {
    chrono::Local::now().format("%b %d %H:%M:%S %Y").to_string()
}

fn fetch_latest_values_promises(
    table_data: &Vec<crate::app::ValueData>,
) -> VecDeque<Promise<(String, String)>> {
    println!("fetching latest values");
    let mut promises = VecDeque::new();

    for row in table_data {
        let id = row.id.clone();
        let link = row.link.clone();
        let css_selector = row.css_selector.clone();

        let promise = get_web_value_promise(id, link, css_selector);
        promises.push_back(promise);
    }

    promises
}

fn fetch_latest_values_and_notify_blocking(
    table_data: &mut Vec<crate::app::ValueData>,
) -> VecDeque<(String, String)> {
    println!("fetching latest values, notify");
    let mut new_values = VecDeque::new();

    for row in &*table_data {
        let id = row.id.clone();
        let name = row.name.clone();
        let link = row.link.clone();
        let css_selector = row.css_selector.clone();
        let old_value = row.latest_value.clone();

        let new_value = get_web_value_blocking(link, css_selector);

        if !new_value.is_empty() && new_value != old_value {
            show_notifcation(&name, &old_value, &new_value);
        }
        new_values.push_back((id, new_value));
    }
    new_values
}

fn get_web_value_blocking(link: String, css_selector: String) -> String {
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async {
            crate::get_current_value(&link, &css_selector)
                .await
                .unwrap_or_default()
        })
    })
}
