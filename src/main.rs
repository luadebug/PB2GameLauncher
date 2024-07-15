use std::error::Error;
use std::io::Read;
use iced::widget::scrollable;

use reqwest::blocking::get;
use scraper::{ElementRef, Html, Node, Selector};
use std::str::FromStr;

use reqwest::header::{ACCEPT, ACCEPT_ENCODING, ACCEPT_LANGUAGE, CONTENT_TYPE, HOST, ORIGIN, REFERER, USER_AGENT};
use flate2::read::GzDecoder;
use md5::compute;

pub async fn login_website_http_post(login: &String, password: &String) -> Result<String, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:127.0) Gecko/20100101 Firefox/127.0".parse()?);
    headers.insert(ACCEPT, "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8".parse()?);
    headers.insert(ACCEPT_LANGUAGE, "en-US,en;q=0.5".parse()?);
    headers.insert(ACCEPT_ENCODING, "gzip".parse()?);
    headers.insert(CONTENT_TYPE, "application/x-www-form-urlencoded".parse()?);
    headers.insert(ORIGIN, "https://www.plazmaburst2.com".parse()?);
    headers.insert(REFERER, "https://www.plazmaburst2.com/".parse()?);
    // Check if the password is already in MD5 format
    let md5_regex = regex::Regex::new(r"^[a-f0-9]{32}$").unwrap();
    let password_to_use = if md5_regex.is_match(&password) {
        password.clone()
    } else {
        // Convert password to MD5 if it's not in MD5 format
        format!("{:x}", compute(password.as_bytes()))
    };
    let response = client.post("https://www.plazmaburst2.com/")
        .headers(headers)
        .body(format!("login={}&password={}&Submit=Log-in", login, password_to_use))
        .send()
        .await?;
    println!("login={}&password={}&Submit=Log-in", login, password_to_use);
    if response.status() != reqwest::StatusCode::OK {
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Request failed")));
    }

    let content_encoding = response.headers().get(reqwest::header::CONTENT_ENCODING);
    let body = if let Some(encoding) = content_encoding {
        if encoding == "gzip" {
            // If GZIP-encoded, decode the content
            let bytes = response.bytes().await?;
            let mut gz = GzDecoder::new(&bytes[..]);
            let mut decoded_body = String::new();
            gz.read_to_string(&mut decoded_body)?;
            decoded_body
        } else {
            response.text().await?
        }
    } else {
        response.text().await?
    };

    let document = Html::parse_document(&body);
    let selector = Selector::parse("td#wb_box").unwrap();
    let mut message_found = false;

    if let Some(element) = document.select(&selector).next() {
        let welcome_message = element.text().collect::<Vec<_>>().join(" ").trim().to_string();
        if let Some(end) = welcome_message.find('!') {
            let extracted_message = &welcome_message[..=end]; // Includes the exclamation mark
            println!("Extracted Message: {}", extracted_message);
            message_found = true;
            return Ok(extracted_message.to_string());
        }
    }
    //let l = login.clone();
    //let p = password.clone();
    // If the welcome message is not found, search for an alert() call

    if !message_found {
        let alert_regex = regex::Regex::new("alert\\(['\"](.*?)['\"]\\)").unwrap();
        if let Some(caps) = alert_regex.captures(&body) {
            if let Some(message) = caps.get(1) {
                let regex = regex::Regex::new(r"\\n\\n|\\n").unwrap();
                let msg = regex.replace_all(message.as_str(), "\r\n");
                let regex2 = regex::Regex::new(r"\\").unwrap();
                let cleaned_msg = regex2.replace_all(&msg, "").to_string();
                return Ok(cleaned_msg);
            }
        }
    }
    return Ok("No connection to game server.".to_string());

    // If neither the welcome message nor an alert() message is found, return a default error message
    //return login_website_http_post_rq_load(login.to_string(), password.to_string());
}





pub async fn login_website_http_post_rq_load(login: &String, password: &String) -> Result<String, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(USER_AGENT, "Shockwave Flash".parse()?);
    headers.insert(ACCEPT, "text/xml, application/xml, application/xhtml+xml, text/html;q=0.9, text/plain;q=0.8, text/css, image/png, image/jpeg, image/gif;q=0.8, application/x-shockwave-flash, video/mp4;q=0.9, flv-application/octet-stream;q=0.8, video/x-flv;q=0.7, audio/mp4, application/futuresplash, */*;q=0.5".parse()?);
    headers.insert(CONTENT_TYPE, "application/x-www-form-urlencoded".parse()?);
    //headers.insert(b"x-flash-version", "11,7,700,224".parse()?);
    headers.insert(HOST, "www.plazmaburst2.com".parse()?);

    let response = client.post("https://www.plazmaburst2.com/pb2/server.php")
        .headers(headers)
        .body(format!("rq=load&l={}&p={}", login, password))
        .send()
        .await?;
    println!("login={}&password={}&Submit=Log-in", login, password);
    if response.status() == 200
    {
        return Ok(response.text().await?);
    }
    else
    {
        return Ok("Failed to login.".to_string());
    }
}





fn get_news_and_dates_by_page_number(pagenumber: u8) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
    let body = get(format!("https://www.plazmaburst2.com/?a=&s=0&pg={}", pagenumber))?.text()?;
    let fragment = Html::parse_document(&body);
    let date_selector = Selector::parse("strong.news_date").unwrap();

    let mut results = Vec::new();

    for element in fragment.select(&date_selector) {
        let date = element.inner_html();
        let mut news_text = String::new();
        let mut next_sibling = element.next_sibling();
        while let Some(sibling) = next_sibling {
            match sibling.value() {
                Node::Element(el) => {
                    let el_ref = ElementRef::wrap(sibling).unwrap();
                    // Check if the element is a <div> with align="center", then break
                    if el_ref.value().name() == "div" && el_ref.value().attr("align") == Some("center") {
                        break;
                    }
                    if el_ref.value().name() == "div" && el_ref.value().attr("class") == Some("news_div") {
                        break;
                    }
                    if el_ref.value().name() == "br" {
                        news_text.push('\n');
                    } else if el_ref.value().name() == "b" {
                        news_text.push_str(&el_ref.inner_html());
                    } else if el_ref.value().name() == "a" {
                        // Extract both the URL and the link text
                        if let Some(href) = el_ref.value().attr("href") {
                            let link_text = el_ref.inner_html();
                            // Format and append the link and text to news_text
                            news_text.push_str(&format!("<a href=\"{}\">{}</a>", href, link_text));
                        }
                    } else {
                        // Include other elements' inner HTML
                        news_text.push_str(&el_ref.inner_html());
                    }
                },
                Node::Text(text_node) => {
                    news_text.push_str(&text_node.text);
                },
                _ => {}
            }
            next_sibling = sibling.next_sibling();
        }
        news_text = news_text.replace("\t", "");
        news_text = news_text.replace("\n\n", "\n");
        news_text = news_text.replace("</a>.\n", "</a>");
        news_text = news_text.replace("</a>!", "</a>");
        news_text = news_text.replace("</a>.", "</a>");
        results.push((date, news_text));
    }
    println!("News Text: {:?}", results); // Print news_text before pushing
    Ok(results)
}



fn get_news_pages_count() -> Result<u8, Box<dyn std::error::Error>> {
    let body = get("https://www.plazmaburst2.com/")?.text()?;

    let fragment = Html::parse_document(&body);
    let selector = Selector::parse("div > a").unwrap();

    let mut max_page = 0;

    for element in fragment.select(&selector) {
        if let Some(page_number_str) = element.value().attr("href") {
            if let Some(page_number) = page_number_str.split('=').last() {
                if let Ok(page_number) = u8::from_str(page_number) {
                    if page_number > max_page {
                        max_page = page_number;
                    }
                }
            }
        }
    }

    println!("Last page number: {}", max_page + 1);

    Ok(max_page + 1)
}


use once_cell::sync::Lazy;


use iced::advanced::{Renderer, Widget};
use iced::futures::SinkExt;
mod styles;
mod LauncherMainWindow;

static SCROLLABLE_ID: Lazy<scrollable::Id> = Lazy::new(scrollable::Id::unique);



pub fn main() -> iced::Result {
    iced::application(
        "Plazma Burst 2 Launcher",
        LauncherMainWindow::LauncherMainWindow::update,
        LauncherMainWindow::LauncherMainWindow::view,
    )
        .theme(LauncherMainWindow::LauncherMainWindow::theme)
        .run()
}


#[derive(Debug, Clone, Eq, PartialEq, Copy)]
enum Direction {
    Vertical,
    Horizontal,
    Multi,
}

#[derive(Debug, Clone)]
pub enum Message {
    SwitchDirection(Direction),
    AlignmentChanged(scrollable::Alignment),
    ScrollbarWidthChanged(u16),
    ScrollbarMarginChanged(u16),
    ScrollerWidthChanged(u16),
    ScrollToBeginning,
    ScrollToEnd,
    Scrolled(scrollable::Viewport),
    PageLoaded,
    PageChanged(u8), // swap page handler
    PageLoadFailed,
    UsernameChanged(String),
    PasswordChanged(String),
    LoginPressed,
    LoginCancel,
    LoginCompleted(bool, String),
    SubmitLogin,
    TogglePasswordVisibility,
    LinkClicked(String),
    DownloadGamePressed,
    PlayGamePressed
}






