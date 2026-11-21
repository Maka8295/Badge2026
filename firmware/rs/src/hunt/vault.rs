use esp_idf_svc::sys::{mbedtls_md_hmac, mbedtls_md_info_from_type, mbedtls_md_type_t_MBEDTLS_MD_SHA256, mbedtls_sha256};

const HUNT_BIN: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/hunt.bin"));
const MAGIC: &[u8] = b"HUNT";
const KEY_LEN: usize = 32;
const TAG_LEN: usize = 16;
const NOT_SHA224: i32 = 0;
const FIRST_BLOCK_INDEX: [u8; 4] = 1u32.to_be_bytes();
const TAG_LABEL: &[u8] = b"tag";
const ENCRYPT_LABEL: &[u8] = b"enc";
const FINISH_LABEL: &[u8] = b"finish";
const PUBLIC_LABEL: &[u8] = b"public";
const FINISH_CODE_BYTES: usize = 4;

type Key = [u8; KEY_LEN];

struct Reader<'a> {
    data: &'a [u8],
}

impl<'a> Reader<'a> {
    fn take(&mut self, len: usize) -> &'a [u8] {
        let (head, rest) = self.data.split_at(len);
        self.data = rest;
        head
    }

    fn u16(&mut self) -> u16 {
        u16::from_le_bytes(self.take(2).try_into().unwrap())
    }

    fn u32(&mut self) -> u32 {
        u32::from_le_bytes(self.take(4).try_into().unwrap())
    }
}

struct Locked<'a> {
    tag: &'a [u8],
    body: &'a [u8],
}

pub struct Vault {
    salt: &'static [u8],
    iterations: u32,
    public: &'static [u8],
    stages: Vec<Locked<'static>>,
}

impl Vault {
    pub fn load() -> Self {
        let mut r = Reader { data: HUNT_BIN };
        assert_eq!(r.take(MAGIC.len()), MAGIC, "hunt.bin has a bad header");
        let iterations = r.u32();
        let salt_len = r.u16() as usize;
        let salt = r.take(salt_len);
        let public_len = r.u32() as usize;
        let stage_count = r.u16();
        let public = r.take(public_len);
        let stages = (0..stage_count).map(|_| read_locked(&mut r)).collect();
        Vault { salt, iterations, public, stages }
    }

    pub fn public(&self) -> Vec<u8> {
        decrypt(&sha256(&[self.salt, PUBLIC_LABEL]), self.public)
    }

    pub fn open(&self, answer: &str) -> Option<Vec<u8>> {
        let key = pbkdf2(normalize(answer).as_bytes(), self.salt, self.iterations);
        let wanted = sha256(&[&key, TAG_LABEL]);
        let locked = self.stages.iter().find(|s| s.tag == &wanted[..TAG_LEN])?;
        Some(decrypt(&key, locked.body))
    }
}

pub fn finish_code(answer: &str, board_name: &str) -> String {
    let digest = sha256(&[FINISH_LABEL, normalize(answer).as_bytes(), normalize(board_name).as_bytes()]);
    let hex: String = digest[..FINISH_CODE_BYTES].iter().map(|b| format!("{b:02X}")).collect();
    format!("{}-{}", &hex[..FINISH_CODE_BYTES], &hex[FINISH_CODE_BYTES..])
}

fn read_locked<'a>(r: &mut Reader<'a>) -> Locked<'a> {
    let tag = r.take(TAG_LEN);
    let len = r.u32() as usize;
    Locked { tag, body: r.take(len) }
}

pub fn normalize(answer: &str) -> String {
    answer.trim().to_lowercase()
}

fn decrypt(key: &Key, body: &[u8]) -> Vec<u8> {
    body.chunks(KEY_LEN)
        .enumerate()
        .flat_map(|(i, chunk)| {
            let block = sha256(&[key, ENCRYPT_LABEL, &(i as u32).to_be_bytes()]);
            chunk.iter().zip(block).map(|(c, k)| c ^ k).collect::<Vec<_>>()
        })
        .collect()
}

fn pbkdf2(password: &[u8], salt: &[u8], iterations: u32) -> Key {
    let mut u = hmac(password, &[salt, &FIRST_BLOCK_INDEX].concat());
    let mut out = u;
    for _ in 1..iterations {
        u = hmac(password, &u);
        out.iter_mut().zip(u).for_each(|(o, x)| *o ^= x);
    }
    out
}

fn hmac(key: &[u8], data: &[u8]) -> Key {
    let mut out = [0u8; KEY_LEN];
    // SAFETY: all pointers are valid for the lengths passed; SHA-256 HMAC writes exactly KEY_LEN bytes.
    unsafe {
        let md = mbedtls_md_info_from_type(mbedtls_md_type_t_MBEDTLS_MD_SHA256);
        mbedtls_md_hmac(md, key.as_ptr(), key.len(), data.as_ptr(), data.len(), out.as_mut_ptr());
    }
    out
}

fn sha256(parts: &[&[u8]]) -> Key {
    let data = parts.concat();
    let mut out = [0u8; KEY_LEN];
    // SAFETY: data and out are valid for the lengths passed; SHA-256 writes exactly KEY_LEN bytes.
    unsafe { mbedtls_sha256(data.as_ptr(), data.len(), out.as_mut_ptr(), NOT_SHA224) };
    out
}
