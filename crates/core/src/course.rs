use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Course {
    pub code: String,
    pub title: String,
    pub credits: i64,
    pub cycle: Cycle,
    #[serde(default)]
    pub prerequisites: Option<Prerequisites>,
    #[serde(default)]
    pub equivalents: Vec<String>,
    pub seasons: BTreeMap<Season, SeasonOffering>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Prerequisites {
    pub raw: String,
    pub tree: PrereqTree,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum PrereqTree {
    Course(String),
    All { all: Vec<PrereqTree> },
    Any { any: Vec<PrereqTree> },
    ProgramCredits { program_credits: ProgramCredits },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ProgramCredits {
    pub program: String,
    pub credits: i64,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SeasonOffering {
    pub components: Vec<Component>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Component {
    #[serde(rename = "type")]
    pub kind: ComponentKind,
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Section {
    pub nrc: String,
    pub section: Option<String>,
    pub mode: Mode,
    pub slots: Vec<Slot>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Slot {
    pub day: Day,
    pub start: Time,
    pub end: Time,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Season {
    Fall,
    Winter,
    Summer,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum ComponentKind {
    Lecture,
    Laboratory,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    InPerson,
    Remote,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Day {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(try_from = "String", into = "String")]
pub struct Time {
    pub hour: u8,
    pub minute: u8,
}

impl TryFrom<String> for Time {
    type Error = String;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        let (h, m) = s
            .split_once(':')
            .ok_or_else(|| format!("invalid time (expected HH:MM) : {s}"))?;
        let hour =
            h.parse::<u8>().map_err(|_| format!("invalid hour : {s}"))?;
        let minute = m
            .parse::<u8>()
            .map_err(|_| format!("invalid minute : {s}"))?;
        if hour > 23 || minute > 59 {
            return Err(format!("time out of range : {s}"));
        }
        Ok(Time { hour, minute })
    }
}

impl From<Time> for String {
    fn from(t: Time) -> String {
        format!("{:02}:{:02}", t.hour, t.minute)
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
#[serde(try_from = "u8", into = "u8")]
pub enum Cycle {
    First,
    Second,
}

impl TryFrom<u8> for Cycle {
    type Error = String;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            1 => Ok(Cycle::First),
            2 => Ok(Cycle::Second),
            other => Err(format!("invalid level : {other}")),
        }
    }
}

impl From<Cycle> for u8 {
    fn from(c: Cycle) -> u8 {
        match c {
            Cycle::First => 1,
            Cycle::Second => 2,
        }
    }
}
