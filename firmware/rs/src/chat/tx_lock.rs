use esp_idf_svc::nvs::{EspNvs, NvsDefault};
use esp_idf_svc::sys::mbedtls_sha256;

const KEY_TX_UNLOCKED: &str = "tx_ok";
const KEY_PASSWORD_SHA256: &str = "tx_pw";
const PASSWORD_MIN: usize = 4;
const TOO_SHORT: &str = "password too short (min 4)";
const SAVE_FAILED: &str = "could not save password";
const SHA256_LEN: usize = 32;
const NOT_SHA224: i32 = 0;
const DEFAULT_PASSWORD_SHA256: [u8; SHA256_LEN] = [
    0x27, 0x25, 0xda, 0x0f, 0x54, 0xe9, 0xc8, 0xb4, 0x6d, 0x62, 0xfb, 0x84, 0xce, 0xe5, 0x29, 0xc0, 0xbd, 0x62, 0xea,
    0xa3, 0xc8, 0xf3, 0x77, 0x6d, 0x8d, 0x18, 0x23, 0xf0, 0x43, 0x98, 0xa1, 0xd2,
];

pub const DISCLAIMER: [&str; 8] = [
    "LoRa transmit is locked.",
    "Radio rules differ from place to place.",
    "You are responsible for the frequency,",
    "power and content you send.",
    "You transmit at your own risk.",
    "",
    "Type the password and press OK to",
    "accept this and unlock transmit.",
];

pub fn is_unlocked(nvs: &EspNvs<NvsDefault>) -> bool {
    nvs.get_u8(KEY_TX_UNLOCKED).ok().flatten() == Some(1)
}

pub fn try_unlock(nvs: &EspNvs<NvsDefault>, password: &str) -> bool {
    if sha256(password.as_bytes()) != password_sha256(nvs) {
        return false;
    }
    if let Err(e) = nvs.set_u8(KEY_TX_UNLOCKED, 1) {
        log::error!("saving tx unlock: {e}");
    }
    true
}

pub fn set_password(nvs: &EspNvs<NvsDefault>, password: &str) -> Result<(), &'static str> {
    if password.len() < PASSWORD_MIN {
        return Err(TOO_SHORT);
    }
    nvs.set_blob(KEY_PASSWORD_SHA256, &sha256(password.as_bytes())).map_err(|e| {
        log::error!("saving tx password: {e}");
        SAVE_FAILED
    })
}

fn password_sha256(nvs: &EspNvs<NvsDefault>) -> [u8; SHA256_LEN] {
    let mut stored = [0u8; SHA256_LEN];
    let found = matches!(nvs.get_blob(KEY_PASSWORD_SHA256, &mut stored), Ok(Some(blob)) if blob.len() == SHA256_LEN);
    if found {
        stored
    } else {
        DEFAULT_PASSWORD_SHA256
    }
}

fn sha256(data: &[u8]) -> [u8; SHA256_LEN] {
    let mut digest = [0u8; SHA256_LEN];
    // SAFETY: data and digest are valid for the lengths passed; mbedtls writes exactly SHA256_LEN bytes.
    unsafe { mbedtls_sha256(data.as_ptr(), data.len(), digest.as_mut_ptr(), NOT_SHA224) };
    digest
}
