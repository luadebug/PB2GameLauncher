use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use cfg_if::cfg_if;
use iced::widget::{Button, Column, Container, Row, scrollable, Scrollable, Text, TextInput, Tooltip};
use iced::{Alignment, Element, Length, Task, Theme};
use iced::advanced::graphics::text::cosmic_text::Command;
use iced::futures::channel::mpsc;
use iced::futures::{channel, SinkExt};
use regex::Regex;
use crate::{Direction, Message, SCROLLABLE_ID, styles};


use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug)]
struct LoginResponse {
    success: bool,
    message: Option<String>,
}

enum LoginMethod {
    GameWebsite,
    StandaloneLauncher,
    MD5Password,
}

impl LoginMethod {
    fn message_suffix(&self) -> &'static str {
        match self {
            LoginMethod::GameWebsite => "\r\n(Signed in with password for game website)",
            LoginMethod::StandaloneLauncher => "\r\n(Signed in with password for standalone launcher)",
            LoginMethod::MD5Password => "\r\n(Signed with md5 password for game website)",
        }
    }
}


pub struct LauncherMainWindow {
    scrollable_direction: Direction,
    scrollbar_width: u16,
    scrollbar_margin: u16,
    scroller_width: u16,
    current_scroll_offset: scrollable::RelativeOffset,
    alignment: scrollable::Alignment,
    news_pages_count: u8,
    current_page: u8,
    loading_page: bool,
    news: Option<Vec<(String, String)>>,
    show_login_form: bool,
    username: String,
    password: String,
    password_visible: bool,
    has_signed_in: bool,
    signed_in_as: String,

}

impl LauncherMainWindow {
    fn new() -> Self {
        LauncherMainWindow {
            scrollable_direction: Direction::Vertical,
            scrollbar_width: 10,
            scrollbar_margin: 0,
            scroller_width: 10,
            current_scroll_offset: scrollable::RelativeOffset::START,
            alignment: scrollable::Alignment::Start,
            news_pages_count: crate::get_news_pages_count().unwrap_or_default(),
            current_page: 0,
            loading_page: false,
            news: Option::from(crate::get_news_and_dates_by_page_number(0).unwrap_or_default()),
            show_login_form: false,
            username: String::new(),
            password: String::new(),
            password_visible: false,
            has_signed_in: false,
            signed_in_as: String::new(),

        }
    }

    fn parse_and_create_elements<'a>(&'a self, html_text: &'a str) -> Column<'a, Message> {
        let mut elements = Column::new().spacing(5);
        let re = regex::Regex::new("<a href=\"(.*?)\">(.*?)</a>").unwrap();
        let mut last_end = 0;

        for cap in re.captures_iter(html_text) {
            let (start, end) = (cap.get(0).unwrap().start(), cap.get(0).unwrap().end());


            let url = if cap[1].starts_with("http") {
                cap[1].to_string()
            } else if cap[1].starts_with('/') {
                format!("https://plazmaburst2.com{}", cap[1].to_string())
            } else {
                format!("https://plazmaburst2.com/{}", cap[1].to_string())
            };


            let text = cap[2].to_string();

            // Add text before the link
            if start > last_end {
                elements = elements.push(Text::new(&html_text[last_end..start]).font(iced::Font::with_name("Segoe UI Emoji")));
            }

            // Create a clickable element for the link
            let button = Button::new(Text::new(text).font(iced::Font::with_name("Segoe UI Emoji")))
                .style(styles::transparent_button_hyperlink_style(&self.theme()))
                .on_press(Message::LinkClicked(url.clone()));

            let tooltip_button = Tooltip::new(button,
                                              Text::new(url.clone()),
                                              iced::widget::tooltip::Position::FollowCursor);

            elements = elements.push(tooltip_button);

            last_end = end;
        }

        // Add any remaining text after the last link
        if last_end < html_text.len() {
            elements = elements.push(Text::new(&html_text[last_end..]).font(iced::Font::with_name("Segoe UI Emoji")));
        }

        elements
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::LinkClicked(url) => {
                std::thread::spawn(move || {
                    if let Err(e) = open::that(&url) {
                        eprintln!("Failed to open link: {}", e);
                    }
                });
                Task::none()
            }
            Message::TogglePasswordVisibility => {
                self.password_visible = !self.password_visible;
                Task::none()
            }
            Message::UsernameChanged(username) => {
                self.username = username;
                Task::none()
            }
            Message::PasswordChanged(password) => {
                self.password = password;
                Task::none()
            }


            Message::SubmitLogin => {
                self.show_login_form = false;
                let message = format!("Username: {}, Password: {}", self.username, self.password);
                let username = self.username.clone();
                let password = self.password.clone();
                let (sender, mut receiver) = std::sync::mpsc::channel();


                tokio::spawn(async move {
                    let (fetch_result, has_signed_in) = handle_login(&username, &password).await;

                    let message_level = if has_signed_in {
                        rfd::MessageLevel::Info
                    } else {
                        rfd::MessageLevel::Error
                    };

                    println!("{}", fetch_result);
                    rfd::MessageDialog::new()
                        .set_title(&message)
                        .set_description(&fetch_result)
                        .set_level(message_level)
                        .show();
                    sender.send(has_signed_in).unwrap();
                });
                let has_signed_in = receiver.recv().unwrap();
                self.has_signed_in = has_signed_in;
                self.signed_in_as = self.username.clone();

                if (self.has_signed_in)
                {
                    let exe_path = match std::env::current_exe() {
                        Ok(path) => path,
                        Err(err) => {
                            eprintln!("Failed to get current executable path: {}", err);
                            return Task::none();
                        }
                    };

                    let auth_file_path = exe_path
                        .parent()
                        .expect("Unable to get parent of Launcher EXE path.")
                        .join("Plazma Burst 2.auth")
                        .canonicalize()
                        .unwrap_or_else(|_| {
                            eprintln!("Failed to get canonical path to auth file.");
                            PathBuf::from("Plazma Burst 2.auth") // Fallback
                        });

                    let auth_file_path = auth_file_path
                        .to_str()
                        .map(|s| s.trim_start_matches(r"\\?\"))
                        .unwrap_or_else(|| {
                            eprintln!("Failed to convert auth file path to string.");
                            "Plazma Burst 2.auth"
                        })
                        .to_string();


                    if let Err(e) = write_auth_file(&auth_file_path, &self.username, &self.password) {
                        eprintln!("Failed to write auth file: {}", e);
                    }
                }


                Task::none()
            }

            Message::PlayGamePressed => {
                tokio::spawn(async move {
                    start_game_process().await;
                });
                Task::none()
            }

            Message::DownloadGamePressed => {
                tokio::spawn(async move {
                    handle_download_game().await;
                });
                Task::none()
            }
            Message::LoginPressed => {
                self.show_login_form = true;
                Task::none()
            }
            Message::LoginCancel => {
                self.show_login_form = false;
                Task::none()
            }
            Message::LoginCompleted(has_signed_in, username) => {
                self.has_signed_in = has_signed_in;
                if has_signed_in {

                    self.signed_in_as = username;


                } else {
                    self.signed_in_as.clear();
                }
                Task::none()
            }
            Message::PageChanged(page_number) if page_number != self.current_page => {
                self.loading_page = true;
                self.current_page = page_number;
                self.news = crate::get_news_and_dates_by_page_number(page_number).ok();
                self.loading_page = false;
                Task::none()

            },
            Message::SwitchDirection(direction) => {
                self.current_scroll_offset = scrollable::RelativeOffset::START;
                self.scrollable_direction = direction;

                scrollable::snap_to(
                    SCROLLABLE_ID.clone(),
                    self.current_scroll_offset,
                )
            }
            Message::AlignmentChanged(alignment) => {
                self.current_scroll_offset = scrollable::RelativeOffset::START;
                self.alignment = alignment;

                scrollable::snap_to(
                    SCROLLABLE_ID.clone(),
                    self.current_scroll_offset,
                )
            }
            Message::ScrollbarWidthChanged(width) => {
                self.scrollbar_width = width;

                Task::none()
            }
            Message::ScrollbarMarginChanged(margin) => {
                self.scrollbar_margin = margin;

                Task::none()
            }
            Message::ScrollerWidthChanged(width) => {
                self.scroller_width = width;

                Task::none()
            }
            Message::ScrollToBeginning => {
                self.current_scroll_offset = scrollable::RelativeOffset::START;

                scrollable::snap_to(
                    SCROLLABLE_ID.clone(),
                    self.current_scroll_offset,
                )
            }
            Message::ScrollToEnd => {
                self.current_scroll_offset = scrollable::RelativeOffset::END;

                scrollable::snap_to(
                    SCROLLABLE_ID.clone(),
                    self.current_scroll_offset,
                )
            }
            Message::Scrolled(viewport) => {
                self.current_scroll_offset = viewport.relative_offset();

                Task::none()
            }
            _ => { Task::none() }
        }
    }

    pub fn view(&self) -> Element<Message> {


        if self.show_login_form {
            let login_form = self.create_login_form();
            Container::new(login_form)
                .padding(20)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into()
        }
        else
        {
        let mut content = Column::new().spacing(20);



            if let Some(news) = &self.news {
                for (date, text) in news {
                    let text_date = Text::new(date)
                        .font(iced::Font::with_name("Verdana"))
                        .size(25)
                        .color([0.58, 0.75, 0.95])
                        .width(Length::Fill)
                        .height(Length::Shrink);

                    // Call parse_and_create_elements for each news item text
                    let parsed_news_content = self.parse_and_create_elements(text);

                    let column = Column::new().push(text_date).push(parsed_news_content);
                    content = content.push(column);
                }
            }

            let buttons_row = Row::new()
                .spacing(10) // Adjust spacing between buttons as needed
                .padding(10); // Adjust padding inside the scrollable area as needed


            let buttons_row = (1..=self.news_pages_count).fold(buttons_row, |row, i| {
                let mut button = Button::new(Text::new(i.to_string()));
                if self.current_page == i - 1 {
                    button = button.style(styles::news_pages_selected_button_style(&self.theme()));
                } else {
                    button = button.style(styles::news_pages_switch_button_style(&self.theme()));
                }
                if !self.loading_page {
                    button = button.on_press(Message::PageChanged(i - 1));
                }
                row.push(button)
            });

            let scrollable_buttons = Scrollable::with_direction(
                buttons_row,
                scrollable::Direction::Horizontal(
                    scrollable::Properties::new()
                        .scroller_width(10)
                )).width(Length::Fill).height(70);


        let scrollable_content = Scrollable::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .id(SCROLLABLE_ID.clone())
            .on_scroll(Message::Scrolled);

        let final_content = Column::new()
            .align_items(Alignment::Center)
            .spacing(10)
            .push(scrollable_content)
            .push(scrollable_buttons);

            println!("{}", self.has_signed_in);
            let signed_in_text = if self.has_signed_in {
                Text::new(format!("Signed in as: {}", self.signed_in_as)).font(iced::Font::with_name("Segoe UI Emoji"))
            } else {
                Text::new("Not signed in").font(iced::Font::with_name("Segoe UI Emoji"))
            };

        let login_button = Button::new(Text::new("Login")).on_press(Message::LoginPressed);

        let download_game_button = Button::new(Text::new("Download Game"))
                .on_press(Message::DownloadGamePressed);

        let play_game_button = Button::new(Text::new("Play Game"))
                .on_press(Message::PlayGamePressed);

        let button_row = Row::new()
                .spacing(10)
                .push(login_button)
                .push(download_game_button)
                .push(play_game_button);
        let displayed = final_content
            .push(signed_in_text)
            .push(button_row);

            Container::new(displayed).padding(20).into()
        }
    }

    fn create_login_form(&self) -> Element<Message> {
        let username_input = TextInput::new("Username", &*self.username)
            .on_input(Message::UsernameChanged)
            .padding(10);
        let password_input:TextInput<Message> = TextInput::new("Password", &*self.password)
            .secure(!self.password_visible) // Toggle based on the password_visible state
            .on_input(Message::PasswordChanged)
            .padding(10)
            .into(); // Convert to Element<Message> for wrapping in Container

        let visibility_toggle_button:Button<Message> = Button::new(Text::new(if self.password_visible { "👀" } else { "🙈" })
            .font(iced::Font::with_name("Segoe UI Emoji")))
            .on_press(Message::TogglePasswordVisibility)
            .into(); // Convert to Element<Message> for wrapping in Container

        // Assuming you want to set a specific height, e.g., 50 pixels
        let password_input_container = Container::new(password_input)
            .center_y(50); // Center the TextInput vertically within the Container

        let visibility_toggle_button_container = Container::new(visibility_toggle_button.height(40))
            .center_y(50); // Center the Button vertically within the Container

        let password_row = Row::new()
            .push(password_input_container)
            .push(visibility_toggle_button_container);

        let submit_button = Button::new(Text::new("Submit"))
            .on_press(Message::SubmitLogin);
        let cancel_button = Button::new(Text::new("Cancel"))
            .on_press(Message::LoginCancel);
        Column::new()
            .spacing(10)
            .align_items(Alignment::Center)
            .push(username_input)
            .push(password_row)
            .push(submit_button)
            .push(cancel_button)
            .into()
    }

    pub fn theme(&self) -> Theme {
        Theme::Dark
    }
}

fn write_auth_file(path: &str, username: &str, password: &str) -> std::io::Result<()> {
    println!("{} {} {}",path, username, password);
    let mut file = File::create(path)?;
    writeln!(file, "{}", username)?;
    writeln!(file, "{}", password)?;
    Ok(())
}


async fn handle_login(username: &String, password: &String) -> (String, bool) {
    let mut fetch_result = crate::login_website_http_post(&username, &password)
        .await
        .expect("Failed to fetch sign in POST request http post");

    let md5_regex = regex::Regex::new(r"^[a-f0-9]{32}$").unwrap();

    if md5_regex.is_match(&password) && !fetch_result.contains("(") {
        fetch_result = format!("{} {}", fetch_result, "\r\n(Signed with md5 password for game website)");
    } else if fetch_result.starts_with("Welcome back") {
        fetch_result = format!("{} {}", fetch_result, "\r\n(Signed with password for game website)");
    }

    if !fetch_result.starts_with("Welcome back") {
        let fetch_result2 = crate::login_website_http_post_rq_load(&username, &password)
            .await
            .expect("Failed to fetch sign in POST request post rq load");

        if fetch_result2.starts_with("x") {
            fetch_result = format!("Welcome back, {} ! \r\n(Signed in with password for standalone launcher)", username);
        }
    }

    let has_signed_in = fetch_result.starts_with("Welcome back");

    (fetch_result, has_signed_in)
}

async fn download_and_save_file(url: &str, file_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let response = reqwest::get(url).await?;
    let content = response.bytes().await?;
    let mut file = File::create(file_path)?;
    file.write_all(&content)?;
    Ok(())
}

// Define an enum to represent supported platforms and their architectures
#[derive(Debug)]
enum Platform {
    Windows(Architecture),
    MacOS,
    Linux(Architecture),
}

#[derive(Debug)]
enum Architecture {
    X86_64,
    I686,
}

// Define a struct to hold download information
#[derive(Debug)]
struct DownloadInfo {
    url: &'static str,
    file_name: &'static str,
}

impl Platform {
    fn get_download_info(&self) -> Option<&'static DownloadInfo> {
        match self {
            Platform::Windows(arch) => match arch {
                Architecture::X86_64 => Some(&DownloadInfo {
                    url: URL_X86_64_WINDOWS,
                    file_name: "flashplayer.exe",
                }),
                Architecture::I686 => Some(&DownloadInfo {
                    url: URL_I686_WINDOWS,
                    file_name: "flashplayer.exe",
                }),
                _ => None,
            },
            Platform::MacOS => Some(&DownloadInfo {
                url: URL_MACOS,
                file_name: "flashplayer.dmg",
            }),
            Platform::Linux(arch) => match arch {
                Architecture::X86_64 => Some(&DownloadInfo {
                    url: URL_X86_64_LINUX,
                    file_name: "flashplayer",
                }),
                Architecture::I686 => Some(&DownloadInfo {
                    url: URL_I686_LINUX,
                    file_name: "flashplayer",
                }),
                _ => None,
            },
        }
    }
}

// Define constants for download URLs for readability
const URL_X86_64_WINDOWS: &str = "https://github.com/luadebug/PB2GameLauncher/raw/main/flashplayer-x86_64-pc-windows-msvc.exe";
const URL_I686_WINDOWS: &str = "https://github.com/luadebug/PB2GameLauncher/raw/main/flashplayer-i686-pc-windows-msvc.exe";
const URL_MACOS: &str = "https://github.com/luadebug/PB2GameLauncher/raw/main/flashplayer_32_sa.dmg";
const URL_X86_64_LINUX: &str = "https://github.com/luadebug/PB2GameLauncher/raw/main/flashplayer-x86_64-unknown-linux-gnu";
const URL_I686_LINUX: &str = "https://github.com/luadebug/PB2GameLauncher/raw/main/flashplayer-i686-unknown-linux-gnu";

const PB2_TIME: &str = "https://www.plazmaburst2.com/launcher/time.php";

const PB2_SWF: &str = "https://www.plazmaburst2.com/pb2/pb2_re34.swf";

// Function to get the platform based on compile-time configuration
fn get_platform() -> Platform {
    cfg_if! {
        if #[cfg(target_os = "windows")] {
            let arch = if cfg!(target_arch = "x86_64") {
                Architecture::X86_64
            } else if cfg!(target_arch = "i686") {
                Architecture::I686
            } else {
                panic!("Unsupported architecture"); // Or handle gracefully
            };
            Platform::Windows(arch)
        } else if #[cfg(target_os = "macos")] {
            Platform::MacOS
        } else if #[cfg(target_os = "linux")] {
            let arch = if cfg!(target_arch = "x86_64") {
                Architecture::X86_64
            } else if cfg!(target_arch = "i686") {
                Architecture::I686
            } else {
                panic!("Unsupported architecture"); // Or handle gracefully
            };
            Platform::Linux(arch)
        } else {
            panic!("Unsupported operating system"); // Or handle gracefully
        }
    }
}

async fn start_game_process() -> Task<Message> {
    // Get the current executable's directory
    let exe_path = match std::env::current_exe() {
        Ok(path) => path,
        Err(err) => {
            eprintln!("Failed to get current executable path: {}", err);
            return Task::none();
        }
    };

    let swf_file_path = exe_path
        .parent()
        .expect("Unable to get parent of Launcher EXE path.")
        .join("pb2_re34_alt.swf")
        .canonicalize()
        .unwrap_or_else(|_| {
            eprintln!("Failed to get canonical path to SWF file.");
            PathBuf::from("pb2_re34_alt.swf") // Fallback
        });

    let swf_file_path = swf_file_path
        .to_str()
        .map(|s| s.trim_start_matches(r"\\?\"))
        .unwrap_or_else(|| {
            eprintln!("Failed to convert SWF file path to string.");
            "pb2_re34_alt.swf"
        })
        .to_string();

    let auth_file = "Plazma Burst 2.auth";

    let Platform = Platform::get_download_info(&get_platform());

    let flash_player_path = Platform.unwrap().file_name;

    let myparams = if fs::metadata(auth_file).is_ok() {
        let auth_content = fs::read_to_string(auth_file).unwrap_or_default();
        let parts: Vec<&str> = auth_content.splitn(2, '\n').collect();

        if parts.len() == 2 {
            format!("?l={}&p={}&from_standalone=1", parts[0], parts[1])
        } else {
            "?l=.guest&p=.guest&from_standalone=1".to_string()
        }
    } else {
        "?l=.guest&p=.guest&from_standalone=1".to_string()
    };

    println!("display()={}", swf_file_path);

    let command = format!("{}{}", swf_file_path, myparams);
    std::process::Command::new(flash_player_path)
        .args([command])
        .spawn()
        .expect("Failed to start game process");

    Task::none()
}

async fn handle_download_game() -> Task<Message> {

    let Platform = Platform::get_download_info(&get_platform());





    // Check if the download URL is empty
    if Platform.unwrap().url.is_empty() {
        eprintln!("Flashplayer download URL not available for your platform.");
        return Task::none();
    }

    // Get the current executable's directory
    let exe_path = match std::env::current_exe() {
        Ok(path) => path,
        Err(err) => {
            eprintln!("Failed to get current executable path: {}", err);
            return Task::none();
        }
    };

    // Construct the file path based on the executable directory and filename
    let file_path = exe_path.parent().expect("Unable to get parent of Launcher EXE path.").join(Platform.unwrap().file_name);
    let time_file_path = exe_path.parent().expect("Unable to get parent of Launcher EXE path.").join("last_update.v");
    let swf_file_path = exe_path.parent().expect("Unable to get parent of Launcher EXE path.").join("pb2_re34_alt.swf");
    // Check if the file already exists before downloading
    if !std::fs::metadata(&file_path).is_ok() {
        // Download and save the file if it doesn't exist
        tokio::spawn(async move {
            match download_and_save_file(Platform.unwrap().url, &file_path).await {
                Ok(_) => println!("Flashplayer downloaded successfully."),
                Err(err) => eprintln!("Failed to download Flashplayer: {}", err),
            }
        });
    } else {
        println!("Flashplayer already exists in the same directory as the launcher.");
    }

    // Check if the file already exists before downloading
    if !std::fs::metadata(&time_file_path).is_ok() {
        tokio::spawn(async move {
            match download_and_save_file(PB2_TIME, &time_file_path).await {
                Ok(_) => println!("PB2 time downloaded successfully."),
                Err(err) => eprintln!("Failed to download PB2 time: {}", err),
            }
            match download_and_save_file(PB2_SWF, &swf_file_path).await {
                Ok(_) => println!("PB2 swf downloaded successfully."),
                Err(err) => eprintln!("Failed to download PB2 swf: {}", err),
            }
        });

    } else {
        // Check if last_update.v exists
        if std::fs::metadata(&time_file_path).is_ok() {
            tokio::spawn(async move {
                // Read the content of last_update.v
                let local_time = fs::read_to_string(&time_file_path).unwrap_or_default();

                // Fetch the content from the remote URL
                let remote_time = reqwest::get(PB2_TIME).await.expect("Failed to get PB2 time").text().await.unwrap_or_default();

                // Compare and decide whether to download
                if local_time != remote_time {
                    println!("PB2 update available. Downloading...");
                    match download_and_save_file(PB2_TIME, &time_file_path).await {
                        Ok(_) => println!("PB2 time downloaded successfully."),
                        Err(err) => eprintln!("Failed to download PB2 time: {}", err),
                    }
                    match download_and_save_file(PB2_SWF, &swf_file_path).await {
                        Ok(_) => println!("PB2 swf downloaded successfully."),
                        Err(err) => eprintln!("Failed to download PB2 swf: {}", err),
                    }
                    println!("PB2 updated successfully.");
                } else {
                    println!("PB2 is up to date.");
                }
            });
        }
    }


    Task::none()
}
impl Default for LauncherMainWindow {
    fn default() -> Self {
        Self::new()
    }
}