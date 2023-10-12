#![feature(lazy_cell)]
#![allow(clippy::await_holding_lock)]

#[macro_use]
extern crate askama;
#[macro_use]
extern crate log;

use askama::Template;
use embedded_svc::{
  http::server as e_server,
  utils::asyncify::Asyncify,
};
use esp_idf_hal::{
  ledc::{
    self,
    LedcDriver,
  },
  prelude::*,
};
use esp_idf_svc::{
  eventloop,
  http::server,
  nvs,
  timer,
  wifi,
};
use esp_idf_sys as _;
use std::sync::{
  Arc,
  Mutex,
}; /* If using the `binstart` feature of `esp-idf-sys`, always keep this module imported */
use tokio::sync::mpsc::{
  unbounded_channel,
  Sender,
};

const PRESS_DURATION_KEY: &str = "press_duration";
const DUTY_CYCLE_KEY: &str = "duty_cycle_key";

static NVS: std::sync::LazyLock<Mutex<nvs::EspDefaultNvs>> =
  std::sync::LazyLock::new(|| {
    let nvs = nvs::EspDefaultNvs::new(
      nvs::EspDefaultNvsPartition::take().unwrap(),
      "app",
      true,
    )
    .unwrap();

    Mutex::new(nvs)
  });

#[tokio::main]
async fn main() {
  // It is necessary to call this function once. Otherwise some patches to the runtime
  // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
  esp_idf_sys::link_patches();
  // Bind the log crate to the ESP Logging facilities
  esp_idf_svc::log::EspLogger::initialize_default();

  info!("app started successfully");

  let peripherals = Peripherals::take().unwrap();

  let mut ledc_driver = ledc::LedcDriver::<'static>::new(
    peripherals.ledc.channel0,
    ledc::LedcTimerDriver::new(
      peripherals.ledc.timer0,
      &ledc::config::TimerConfig::default().frequency(50.Hz()),
    )
    .unwrap(),
    peripherals.pins.gpio13,
  )
  .unwrap();

  info!("initialized `ledc`");

  let event_loop = eventloop::EspSystemEventLoop::take().unwrap();
  let mut timer_service = timer::EspTaskTimerService::new().unwrap();

  let esp_wifi =
    wifi::EspWifi::new(peripherals.modem, event_loop.clone(), None).unwrap();

  let mut esp_wifi =
    wifi::AsyncWifi::wrap(esp_wifi, event_loop.clone(), timer_service.clone())
      .unwrap();

  esp_wifi
    .set_configuration(&embedded_svc::wifi::Configuration::Client(
      embedded_svc::wifi::ClientConfiguration {
        ssid: "Dino Corp".into(),
        bssid: None,
        auth_method: embedded_svc::wifi::AuthMethod::WPA2Personal,
        password: "DinoGuapo".into(),
        channel: None,
      },
    ))
    .unwrap();

  esp_wifi.start().await.unwrap();

  esp_wifi.connect().await.unwrap();

  esp_wifi.wait_netif_up().await.unwrap();

  let hostname = esp_wifi.wifi().sta_netif().get_hostname().unwrap();
  let ip_address = esp_wifi.wifi().sta_netif().get_ip_info().unwrap().ip;

  info!("WiFi connected and Netif is up at {ip_address} hostname: {hostname}");

  let (tx, mut rx) = unbounded_channel::<()>();

  let mut esp_server =
    server::EspHttpServer::new(&server::Configuration::default()).unwrap();

  esp_server
    .fn_handler("/", e_server::Method::Get, home_handler)
    .unwrap()
    .fn_handler("/press", e_server::Method::Get, move |_| {
      tx.send(()).unwrap();

      Ok(())
    })
    .unwrap()
    .fn_handler(
      "/set-press-duration",
      e_server::Method::Get,
      set_press_duration_handler,
    )
    .unwrap()
    .fn_handler(
      "/set-duty-cycle",
      e_server::Method::Get,
      set_duty_cycle_handler,
    )
    .unwrap();

  let mut timer = timer_service.as_async().timer().unwrap();

  while rx.recv().await.is_some() {
    let nvs_lock = NVS.lock().unwrap();

    let press_duration = match nvs_lock.get_u8(PRESS_DURATION_KEY) {
      Ok(value) => match value {
        Some(value) => value,
        None => continue,
      },
      Err(_) => continue,
    };

    let duty_cycle = match nvs_lock.get_u8(DUTY_CYCLE_KEY) {
      Ok(value) => match value {
        Some(v) => v,
        None => continue,
      },
      Err(_) => continue,
    };

    drop(nvs_lock);

    ledc_driver.set_duty(duty_cycle as _).unwrap();

    ledc_driver.enable().unwrap();

    timer
      .after(std::time::Duration::from_millis(press_duration as _))
      .unwrap()
      .await;

    ledc_driver.disable().unwrap();
  }
}

fn home_handler(
  req: e_server::Request<&mut server::EspHttpConnection>,
) -> e_server::HandlerResult {
  info!("handling `/`");

  #[derive(Template)]
  #[template(path = "index.html")]
  struct HomePage {
    press_duration: u8,
    duty_cycle: u8,
  }

  let nvs_lock = NVS.lock().unwrap();

  let press_duration = nvs_lock.get_u8(PRESS_DURATION_KEY)?.unwrap_or_default();

  let duty_cycle = nvs_lock.get_u8(DUTY_CYCLE_KEY)?.unwrap_or_default();

  drop(nvs_lock);

  let page = HomePage {
    press_duration,
    duty_cycle,
  };

  req
    .into_ok_response()?
    .write(page.render().unwrap().as_bytes())?;

  Ok(())
}

fn set_press_duration_handler(
  mut req: e_server::Request<&mut server::EspHttpConnection>,
) -> e_server::HandlerResult {
  info!("handling `/set-press-duration`");

  let value = match req
    .uri()
    .split_once('=')
    .map(|(_, value)| value)
    .unwrap_or_default()
    .parse::<u8>()
  {
    Ok(value) => value,
    Err(_) => return Ok(()),
  };

  NVS
    .lock()
    .unwrap()
    .set_u8(PRESS_DURATION_KEY, value)
    .unwrap();

  Ok(())
}

fn set_duty_cycle_handler(
  req: e_server::Request<&mut server::EspHttpConnection>,
) -> e_server::HandlerResult {
  info!("handling `/set-duty-cycle`");

  let value = match req
    .uri()
    .split_once('=')
    .map(|(_, value)| value)
    .unwrap_or_default()
    .parse::<u8>()
  {
    Ok(value) => value,
    Err(_) => {
      return Ok(());
    }
  };

  NVS.lock().unwrap().set_u8(DUTY_CYCLE_KEY, value).unwrap();

  Ok(())
}
