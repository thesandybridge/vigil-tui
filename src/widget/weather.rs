use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use serde::Deserialize;
use tokio::task::JoinHandle;

use crate::theme::Theme;

#[derive(Deserialize)]
struct ApiResponse {
    current: CurrentWeather,
}

#[derive(Deserialize)]
struct CurrentWeather {
    temperature_2m: f64,
    weather_code: u8,
    wind_speed_10m: f64,
    relative_humidity_2m: u8,
}

#[derive(Clone)]
struct WeatherData {
    temperature: f64,
    weather_code: u8,
    wind_speed: f64,
    humidity: u8,
    unit_celsius: bool,
    wind_unit_kmh: bool,
}

#[derive(Clone)]
enum FetchResult {
    Loading,
    Data(WeatherData),
    Error(String),
}

impl Default for FetchResult {
    fn default() -> Self {
        Self::Loading
    }
}

#[derive(Default, Clone)]
struct WeatherState {
    result: FetchResult,
}

pub struct WeatherWidget {
    state: Arc<Mutex<WeatherState>>,
    latitude: f64,
    longitude: f64,
    unit_celsius: bool,
    interval: Duration,
}

impl WeatherWidget {
    pub fn new(config: Option<&toml::Value>) -> Result<Self> {
        let latitude = config
            .and_then(|c| c.get("latitude"))
            .and_then(|v| v.as_float())
            .unwrap_or(0.0);
        let longitude = config
            .and_then(|c| c.get("longitude"))
            .and_then(|v| v.as_float())
            .unwrap_or(0.0);
        let unit_celsius = config
            .and_then(|c| c.get("units"))
            .and_then(|v| v.as_str())
            .map(|s| s != "fahrenheit")
            .unwrap_or(true);

        Ok(Self {
            state: Arc::new(Mutex::new(WeatherState::default())),
            latitude,
            longitude,
            unit_celsius,
            interval: Duration::from_secs(300),
        })
    }

    pub fn start_updates(&self) -> JoinHandle<()> {
        let state = Arc::clone(&self.state);
        let interval = self.interval;
        let lat = self.latitude;
        let lon = self.longitude;
        let unit_celsius = self.unit_celsius;

        tokio::spawn(async move {
            let client = reqwest::Client::new();
            loop {
                let temp_unit = if unit_celsius { "celsius" } else { "fahrenheit" };
                let wind_unit = if unit_celsius { "kmh" } else { "mph" };
                let url = format!(
                    "https://api.open-meteo.com/v1/forecast\
                     ?latitude={lat}&longitude={lon}\
                     &current=temperature_2m,relative_humidity_2m,weather_code,wind_speed_10m\
                     &temperature_unit={temp_unit}&wind_speed_unit={wind_unit}"
                );

                let result = async {
                    let resp = client.get(&url).send().await?.error_for_status()?;
                    let data: ApiResponse = resp.json().await?;
                    Ok::<_, anyhow::Error>(data)
                }
                .await;

                {
                    let mut s = state.lock().unwrap();
                    match result {
                        Ok(api) => {
                            s.result = FetchResult::Data(WeatherData {
                                temperature: api.current.temperature_2m,
                                weather_code: api.current.weather_code,
                                wind_speed: api.current.wind_speed_10m,
                                humidity: api.current.relative_humidity_2m,
                                unit_celsius,
                                wind_unit_kmh: unit_celsius,
                            });
                        }
                        Err(e) => {
                            s.result = FetchResult::Error(e.to_string());
                        }
                    }
                }
                tokio::time::sleep(interval).await;
            }
        })
    }
}

fn weather_icon(code: u8) -> &'static str {
    match code {
        0 => "\u{2600}",         // Clear: sun
        1 => "\u{2600}",         // Mainly clear
        2 => "\u{26C5}",         // Partly cloudy
        3 => "\u{2601}",         // Overcast
        45 | 48 => "\u{1F32B}",  // Fog
        51 | 53 | 55 => "\u{1F327}", // Drizzle
        61 | 63 | 65 => "\u{1F327}", // Rain
        66 | 67 => "\u{1F327}",  // Freezing rain
        71 | 73 | 75 => "\u{2744}", // Snow
        77 => "\u{2744}",        // Snow grains
        80 | 81 | 82 => "\u{1F327}", // Rain showers
        85 | 86 => "\u{2744}",   // Snow showers
        95 => "\u{26C8}",        // Thunderstorm
        96 | 99 => "\u{26C8}",   // Thunderstorm + hail
        _ => "\u{2601}",         // Default to cloud
    }
}

fn weather_description(code: u8) -> &'static str {
    match code {
        0 => "Clear sky",
        1 => "Mainly clear",
        2 => "Partly cloudy",
        3 => "Overcast",
        45 => "Fog",
        48 => "Depositing rime fog",
        51 => "Light drizzle",
        53 => "Moderate drizzle",
        55 => "Dense drizzle",
        61 => "Slight rain",
        63 => "Moderate rain",
        65 => "Heavy rain",
        66 => "Light freezing rain",
        67 => "Heavy freezing rain",
        71 => "Slight snow",
        73 => "Moderate snow",
        75 => "Heavy snow",
        77 => "Snow grains",
        80 => "Slight rain showers",
        81 => "Moderate rain showers",
        82 => "Violent rain showers",
        85 => "Slight snow showers",
        86 => "Heavy snow showers",
        95 => "Thunderstorm",
        96 => "Thunderstorm, slight hail",
        99 => "Thunderstorm, heavy hail",
        _ => "Unknown",
    }
}

impl super::Widget for WeatherWidget {
    fn draw(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let snapshot = { self.state.lock().unwrap().clone() };

        let block = Block::default()
            .title(" Weather ")
            .borders(Borders::ALL)
            .border_type(theme.border_type())
            .border_style(Style::default().fg(theme.dim));

        match &snapshot.result {
            FetchResult::Loading => {
                let paragraph = Paragraph::new("Loading...")
                    .block(block)
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(theme.dim));
                frame.render_widget(paragraph, area);
            }
            FetchResult::Error(msg) => {
                let lines = vec![
                    Line::from(Span::styled("Error fetching weather", Style::default().fg(Color::Red))),
                    Line::from(""),
                    Line::from(Span::styled(msg.as_str(), Style::default().fg(theme.dim))),
                ];
                let paragraph = Paragraph::new(lines).block(block);
                frame.render_widget(paragraph, area);
            }
            FetchResult::Data(data) => {
                let icon = weather_icon(data.weather_code);
                let desc = weather_description(data.weather_code);
                let temp_unit = if data.unit_celsius { "\u{00B0}C" } else { "\u{00B0}F" };
                let wind_unit = if data.wind_unit_kmh { "km/h" } else { "mph" };

                let lines = vec![
                    Line::from(Span::styled(
                        format!("{icon} {:.1}{temp_unit}", data.temperature),
                        Style::default().fg(theme.accent),
                    )),
                    Line::from(Span::styled(desc, Style::default().fg(theme.fg))),
                    Line::from(""),
                    Line::from(Span::styled(
                        format!("\u{1F4A8} {:.1} {wind_unit}", data.wind_speed),
                        Style::default().fg(theme.fg),
                    )),
                    Line::from(Span::styled(
                        format!("\u{1F4A7} {}%", data.humidity),
                        Style::default().fg(theme.fg),
                    )),
                ];
                let paragraph = Paragraph::new(lines).block(block);
                frame.render_widget(paragraph, area);
            }
        }
    }

    fn error(&self) -> Option<String> {
        let state = self.state.lock().unwrap();
        if let FetchResult::Error(ref msg) = state.result {
            Some(msg.clone())
        } else {
            None
        }
    }
}
