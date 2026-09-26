//! Built-in word lists. Content warning: these lists hold offensive terms on
//! purpose so Relay can block them. They are matched through the same
//! normalisation as custom filter words (case, accents, leetspeak, homoglyphs,
//! split letters) and are never written to logs or shown unless requested.
//!
//! Words shorter than 6 letters only match exactly, so ambiguous everyday
//! words (for example French "retard", "raton" or "pipe") are left out.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::privacy::ForbiddenConcept;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WordPack {
    Hate,
    Scams,
    Sexual,
    Harassment,
    Doxxing,
}

impl WordPack {
    pub const ALL: [Self; 5] = [
        Self::Hate,
        Self::Scams,
        Self::Sexual,
        Self::Harassment,
        Self::Doxxing,
    ];

    pub fn words(self) -> &'static [&'static str] {
        match self {
            Self::Hate => HATE,
            Self::Scams => SCAMS,
            Self::Sexual => SEXUAL,
            Self::Harassment => HARASSMENT,
            Self::Doxxing => DOXXING,
        }
    }

    /// Concepts built once per pack; each word becomes a canonical value.
    pub fn concepts(self) -> &'static [ForbiddenConcept] {
        static CACHE: OnceLock<Vec<Vec<ForbiddenConcept>>> = OnceLock::new();
        let cache = CACHE.get_or_init(|| {
            Self::ALL
                .iter()
                .map(|pack| {
                    pack.words()
                        .iter()
                        .map(|word| ForbiddenConcept {
                            canonical: (*word).into(),
                            aliases: Vec::new(),
                            regexes: Vec::new(),
                        })
                        .collect()
                })
                .collect()
        });
        let index = Self::ALL
            .iter()
            .position(|pack| *pack == self)
            .expect("every pack is listed in ALL");
        &cache[index]
    }
}

const HATE: &[&str] = &[
    // English
    "nigger",
    "niggers",
    "nigga",
    "niggas",
    "faggot",
    "faggots",
    "retarded",
    "tranny",
    "trannies",
    "kike",
    "kikes",
    "chink",
    "spic",
    "wetback",
    "gook",
    "raghead",
    "towelhead",
    "sandnigger",
    "paki",
    "beaner",
    "heil hitler",
    "sieg heil",
    "white power",
    "gas the jews",
    // Français
    "nègre",
    "négro",
    "negresse",
    "bougnoule",
    "bougnoul",
    "youpin",
    "youtre",
    "bicot",
    "chinetoque",
    "bamboula",
    "pédé",
    "pédale",
    "gouine",
    "travelo",
    "sale arabe",
    "sale noir",
    "sale juif",
    "sale nègre",
    "mort aux juifs",
    "mort aux arabes",
];

const SCAMS: &[&str] = &[
    "free nitro",
    "nitro gratuit",
    "discord nitro free",
    "nitro giveaway",
    "steam gift",
    "cadeau steam",
    "free robux",
    "robux gratuit",
    "crypto airdrop",
    "crypto giveaway",
    "double your crypto",
    "claim your prize",
    "claim your reward",
    "réclame ton cadeau",
    "réclamez votre cadeau",
    "gift for you everyone",
    "free skins giveaway",
    "skins gratuits",
];

const SEXUAL: &[&str] = &[
    "porn",
    "porno",
    "pornhub",
    "xvideos",
    "xhamster",
    "onlyfans",
    "nudes",
    "send nudes",
    "sextape",
    "sexcam",
    "hentai",
    "blowjob",
    "cumshot",
    "gangbang",
    "milf",
    "xxx",
];

const HARASSMENT: &[&str] = &[
    // English
    "kys",
    "kill yourself",
    "go die",
    "motherfucker",
    "cunt",
    "bitch",
    "whore",
    "slut",
    // Français
    "fdp",
    "ntm",
    "fils de pute",
    "nique ta mère",
    "va mourir",
    "suicide toi",
    "tue toi",
    "connard",
    "connasse",
    "salope",
    "pute",
    "enculé",
    "bâtard",
];

const DOXXING: &[&str] = &[
    "dox",
    "doxx",
    "doxxed",
    "doxxing",
    "i know where you live",
    "here is his address",
    "here is her address",
    "voici son adresse",
    "voici ton adresse",
    "voici son numéro",
    "je sais où tu habites",
    "je connais ton adresse",
];
