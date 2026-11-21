use crate::ui::ColorImage;

pub struct Sponsor {
    pub caption: &'static str,
    pub name: Option<&'static str>,
    pub logo: Option<&'static ColorImage>,
}

static META: ColorImage = ColorImage { width: 280, height: 56, pixels: include_bytes!("../../images/meta.raw") };
static KISSINGER: ColorImage = ColorImage { width: 280, height: 69, pixels: include_bytes!("../../images/kissinger.raw") };
static YESWEHACK: ColorImage = ColorImage { width: 280, height: 60, pixels: include_bytes!("../../images/yeswehack.raw") };
static AMAZON: ColorImage = ColorImage { width: 280, height: 94, pixels: include_bytes!("../../images/amazon.raw") };
static ELEMENT55: ColorImage = ColorImage { width: 91, height: 96, pixels: include_bytes!("../../images/element55.raw") };
static RED_BOUNDARY: ColorImage = ColorImage { width: 280, height: 68, pixels: include_bytes!("../../images/redboundary.raw") };
static MEKASMITH: ColorImage = ColorImage { width: 275, height: 120, pixels: include_bytes!("../../images/mekasmith.raw") };
static BINARY_NINJA: ColorImage = ColorImage { width: 198, height: 120, pixels: include_bytes!("../../images/binaryninja.raw") };
static INTIGRITI: ColorImage = ColorImage { width: 202, height: 120, pixels: include_bytes!("../../images/intigriti.raw") };
static CIPHERFALL: ColorImage = ColorImage { width: 134, height: 96, pixels: include_bytes!("../../images/cipherfall.raw") };
static VULN_VIBES: ColorImage = ColorImage { width: 90, height: 96, pixels: include_bytes!("../../images/vulnvibes.raw") };
static SHERPASEC: ColorImage = ColorImage { width: 280, height: 63, pixels: include_bytes!("../../images/sherpasec.raw") };
static INFOSECMAP: ColorImage = ColorImage { width: 120, height: 120, pixels: include_bytes!("../../images/infosecmap.raw") };
static NICK: ColorImage = ColorImage { width: 96, height: 96, pixels: include_bytes!("../../images/nick.raw") };
static EFF: ColorImage = ColorImage { width: 280, height: 54, pixels: include_bytes!("../../images/eff.raw") };
static YAMALAN: ColorImage = ColorImage { width: 96, height: 96, pixels: include_bytes!("../../images/yamalan.raw") };
static TRUFFLE: ColorImage = ColorImage { width: 280, height: 66, pixels: include_bytes!("../../images/truffle.raw") };

const fn logo(caption: &'static str, logo: &'static ColorImage) -> Sponsor {
    Sponsor { caption, name: None, logo: Some(logo) }
}

const fn named(caption: &'static str, name: &'static str, logo: &'static ColorImage) -> Sponsor {
    Sponsor { caption, name: Some(name), logo: Some(logo) }
}

// For sponsors without a logo on tengucon.org: the page shows the Tengu logo above the name.
const fn name_only(caption: &'static str, name: &'static str) -> Sponsor {
    Sponsor { caption, name: Some(name), logo: None }
}

pub static SPONSORS: [Sponsor; 18] = [
    logo("Gold sponsor", &META),
    logo("Bronze sponsor", &KISSINGER),
    logo("Bronze sponsor", &RED_BOUNDARY),
    named("Community sponsor", "Nick", &NICK),
    name_only("Community sponsor", "May Minis"),
    logo("Community sponsor", &YESWEHACK),
    logo("Community sponsor", &AMAZON),
    named("Community sponsor", "Element55", &ELEMENT55),
    logo("Community sponsor", &MEKASMITH),
    logo("Community sponsor", &BINARY_NINJA),
    logo("Community sponsor", &INTIGRITI),
    named("Community sponsor", "Cipherfall", &CIPHERFALL),
    named("Community sponsor", "Vulnerability Vibes", &VULN_VIBES),
    logo("Community sponsor", &SHERPASEC),
    logo("Community partner", &INFOSECMAP),
    logo("Community partner", &TRUFFLE),
    logo("Community partner", &EFF),
    named("Community partner", "YamaLAN", &YAMALAN),
];
