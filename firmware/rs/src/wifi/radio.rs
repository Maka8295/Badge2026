use std::net::Ipv4Addr;

use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::modem::WifiModem;
use esp_idf_svc::sys::EspError;
use esp_idf_svc::wifi::{AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi};

pub struct Network {
    pub ssid: String,
    pub rssi_dbm: i8,
    pub open: bool,
}

pub struct Radio {
    wifi: BlockingWifi<EspWifi<'static>>,
}

impl Radio {
    pub fn start() -> Result<Self, EspError> {
        // SAFETY: the Wi-Fi app holds at most one Radio at a time and nothing else uses the radio modem.
        let modem = unsafe { WifiModem::steal() };
        let sysloop = EspSystemEventLoop::take()?;
        let mut wifi = BlockingWifi::wrap(EspWifi::new(modem, sysloop.clone(), None)?, sysloop)?;
        wifi.set_configuration(&Configuration::Client(ClientConfiguration::default()))?;
        wifi.start()?;
        Ok(Radio { wifi })
    }

    pub fn scan(&mut self) -> Result<Vec<Network>, EspError> {
        let mut networks: Vec<Network> = self.wifi.scan()?.into_iter().filter(|ap| !ap.ssid.is_empty()).map(network).collect();
        networks.sort_by_key(|n| std::cmp::Reverse(n.rssi_dbm));
        networks.dedup_by(|a, b| a.ssid == b.ssid);
        Ok(networks)
    }

    pub fn connect(&mut self, ssid: &str, password: &str) -> Result<(), EspError> {
        self.wifi.set_configuration(&Configuration::Client(client_config(ssid, password)))?;
        self.wifi.connect()?;
        self.wifi.wait_netif_up()
    }

    pub fn disconnect(&mut self) {
        if let Err(e) = self.wifi.disconnect() {
            log::warn!("wifi disconnect: {e}");
        }
    }

    pub fn rssi_dbm(&self) -> Option<i8> {
        self.wifi.wifi().driver().get_ap_info().ok().map(|info| info.signal_strength)
    }

    pub fn ip(&self) -> Option<Ipv4Addr> {
        self.wifi.wifi().sta_netif().get_ip_info().ok().map(|info| info.ip)
    }
}

fn network(ap: esp_idf_svc::wifi::AccessPointInfo) -> Network {
    let open = matches!(ap.auth_method, None | Some(AuthMethod::None));
    Network { ssid: ap.ssid.to_string(), rssi_dbm: ap.signal_strength, open }
}

fn client_config(ssid: &str, password: &str) -> ClientConfiguration {
    let auth_method = if password.is_empty() { AuthMethod::None } else { AuthMethod::WPA2Personal };
    ClientConfiguration {
        ssid: ssid.try_into().unwrap_or_default(),
        password: password.try_into().unwrap_or_default(),
        auth_method,
        ..Default::default()
    }
}
