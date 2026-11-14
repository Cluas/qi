//! 「知气雅集」情感化节律引擎：
//! 1. 24 节气 × 月相 × 窗外天色心境
//! 2. 岁时风物双轨季票（清欢轨 + 雅集轨）与晨露账本（含静养日免费 +3 滴晨露）
//! 3. 食光拍立得贴纸与月历餐垫（免费每月 5 张，雅集无限 + 嘌呤/钠轻提示）
//! 4. 伴侣周日夜话慢递 & 自家灵宠第一人称周信 & 节气藏书票
//! 5. 隐私优先双人同窗（1 字节心情枚举 + 窗台岁时金桂盆栽）
//! 6. 抬腕嘴替桌面微件（诗意隐喻 vs 数值视图隐私开关）
//! 7. 我的四时身体风物长卷 & 宋瓷勋章（含「证伪小达人」与「窗台添柴人」）

use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::analytics::AnalyticsSnapshot;
use crate::entitlements::{
    DEFAULT_LOCALE, EntitlementVerificationResult, OfflineEntitlementState, custom_pet_quota,
    has_feature, is_en_locale,
};
use crate::experiments::ExperimentEvaluation;
use crate::safety_ai::contains_cjk;
use crate::storage::AppSettings;

pub const FREE_STICKER_MONTHLY_QUOTA: usize = 5;
pub const FREE_PRESET_MAX: usize = 2;
pub const MAX_EQUIPPED_KEEPSAKES: usize = 3;
pub const RECOVERY_PRIORITY_DEW_BONUS: i64 = 3;
pub const MAX_PET_PHOTO_DATA_URL_BYTES: usize = 1_048_576;

pub const SOLAR_TERMS_24: [&str; 24] = [
    "小寒", "大寒", "立春", "雨水", "惊蛰", "春分", "清明", "谷雨", "立夏", "小满", "芒种", "夏至",
    "小暑", "大暑", "立秋", "处暑", "白露", "秋分", "寒露", "霜降", "立冬", "小雪", "大雪", "冬至",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolarTermState {
    pub date: String,
    pub term_name: String,
    pub term_index: usize,
    pub season: String,
    pub season_cn: String,
    pub lunar_phase: String,
    pub lunar_day_approx: u32,
    pub weather_mood: String,
    pub active_world: String,
    pub whisper: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonalPassNode {
    pub id: String,
    pub title: String,
    pub track: String,
    pub reward_kind: String,
    pub reward_id: String,
    pub cost_dew: i64,
    pub claimed: bool,
    pub unlocked_by_tier: bool,
    pub evergreen_term: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutfitSpec {
    pub id: String,
    pub name: String,
    pub desc: String,
    pub track: String,
    pub unlocked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealInkSpec {
    pub id: String,
    pub name: String,
    pub hex_hint: String,
    pub unlocked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeepsakeCatalogItem {
    pub id: String,
    pub name: String,
    pub hint: String,
    pub track: String,
    pub unlocked: bool,
    pub equipped: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeepsakeCatalog {
    pub max_equipped_slots: usize,
    pub allowed_slots_for_tier: usize,
    pub equipped_ids: Vec<String>,
    pub items: Vec<KeepsakeCatalogItem>,
    pub outfits: Vec<OutfitSpec>,
    pub seal_inks: Vec<SealInkSpec>,
    pub active_seal_ink: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DewLedgerEntry {
    pub id: i64,
    pub created_at: String,
    pub delta: i64,
    pub source: String,
    pub note: String,
    pub reward_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanctuaryPreset {
    pub id: i64,
    pub name: String,
    pub world_id: String,
    pub weather_mood: String,
    pub seal_ink: String,
    pub keepsakes_json: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoodSticker {
    pub id: i64,
    pub date: String,
    pub title: String,
    pub filter: String,
    pub note: String,
    pub nutrition_hint: String,
    pub image_ref: String,
    pub is_favorite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StickerMonthlyQuota {
    pub year_month: String,
    pub used_count: usize,
    pub limit: Option<usize>,
    pub remaining: Option<usize>,
    pub unlimited: bool,
    pub can_create: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanionWeeklyLetter {
    pub week_number: u32,
    pub title: String,
    pub companion_id: String,
    pub companion_name: String,
    pub solar_term: String,
    pub ex_libris_title: String,
    pub ex_libris_seal: String,
    pub body: String,
    pub unlocked: bool,
    pub lock_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoSanctuaryActionSpec {
    pub id: String,
    pub label: String,
    pub dew_reward: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoSanctuaryState {
    pub pair_code: String,
    pub partner_name: String,
    pub self_mood_byte: u8,
    pub self_mood_label: String,
    pub partner_mood_byte: u8,
    pub partner_mood_label: String,
    pub bonsai_name: String,
    pub bonsai_dew: i64,
    pub bonsai_stage: String,
    pub next_stage_dew: Option<i64>,
    pub available_actions: Vec<CoSanctuaryActionSpec>,
    pub privacy_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetStudioPayload {
    pub privacy_mode: String,
    pub solar_term: String,
    pub lunar_phase: String,
    pub headline: String,
    pub subline: String,
    pub badge_text: String,
    pub companion_line: String,
    pub qi_score_visible: bool,
    pub qi_score: Option<u32>,
    pub metrics_line: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FourSeasonsScrollNode {
    pub season: String,
    pub season_cn: String,
    pub solar_term: String,
    pub title: String,
    pub summary: String,
    pub highlight_value: String,
    pub sealed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CeramicBadge {
    pub id: String,
    pub title: String,
    pub glaze_style: String,
    pub earned: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomPetCompanion {
    pub id: String,
    pub name: String,
    pub species: String,
    pub persona: String,
    pub photo_data_url: String,
    pub die_cut_frame: String,
    pub starlight_memorial: bool,
    pub paw_stamp_title: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomPetQuota {
    pub used_count: usize,
    pub allowed_count: usize,
    pub remaining_add_slots: usize,
    pub can_add_more: bool,
    pub is_pro_ex_libris_unlocked: bool,
    pub can_generate_weekly_letter: bool,
    pub retained_above_free_quota: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PetWeeklyLetter {
    pub id: i64,
    pub pet_id: String,
    pub pet_name: String,
    pub species: String,
    pub week_number: u32,
    pub solar_term: String,
    pub title: String,
    pub body: String,
    pub ex_libris_seal: String,
    pub starlight_memorial: bool,
    pub locale: String,
    pub created_at: String,
}

pub fn normalize_pet_species(raw: &str) -> &'static str {
    let lower = raw.trim().to_ascii_lowercase();
    if lower.is_empty() || lower == "cat" || raw.contains('猫') {
        "cat"
    } else if lower == "dog" || raw.contains('犬') || raw.contains('狗') {
        "dog"
    } else if lower == "rabbit" || lower == "bunny" || raw.contains('兔') {
        "rabbit"
    } else if lower == "fox" || raw.contains('狐') {
        "fox"
    } else {
        "other"
    }
}

pub fn normalize_die_cut_frame(raw: &str) -> &'static str {
    match raw.trim().to_ascii_lowercase().as_str() {
        "lacquer_arch" => "lacquer_arch",
        "paper_cut" => "paper_cut",
        "starlight_halo" => "starlight_halo",
        "warm_stamp" => "warm_stamp",
        _ => "celadon_oval",
    }
}

pub fn validate_pet_photo_data_url(raw: &str, locale: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }
    let en = is_en_locale(locale);
    if trimmed.len() > MAX_PET_PHOTO_DATA_URL_BYTES {
        return Err(if en {
            format!(
                "Pet photo exceeds the {} KB safety limit. Please crop or resize before saving.",
                MAX_PET_PHOTO_DATA_URL_BYTES / 1024
            )
        } else {
            format!(
                "灵宠照片体积超过 {} KB 安全上限，请先裁切压缩后再上传。",
                MAX_PET_PHOTO_DATA_URL_BYTES / 1024
            )
        });
    }

    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("data:") {
        let allowed_prefixes = [
            "data:image/png;base64,",
            "data:image/jpeg;base64,",
            "data:image/jpg;base64,",
            "data:image/webp;base64,",
            "data:image/gif;base64,",
        ];
        let matched_prefix = allowed_prefixes
            .iter()
            .find(|prefix| lower.starts_with(*prefix));
        let Some(prefix) = matched_prefix else {
            return Err(if en {
                "Only raster image data URLs (PNG, JPEG, WebP, or GIF) are allowed for pet portraits.".to_owned()
            } else {
                "仅支持 PNG / JPEG / WebP / GIF 格式的本地图片 Data URL（出于安全限制不支持 SVG 或脚本类型）。".to_owned()
            });
        };
        let b64_payload = trimmed[prefix.len()..].trim();
        if b64_payload.is_empty()
            || !b64_payload
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'=' | b'\n' | b'\r'))
        {
            return Err(if en {
                "Invalid base64 image payload for custom pet photo.".to_owned()
            } else {
                "灵宠图片的 Base64 编码格式无效，请重新选择图片。".to_owned()
            });
        }
        return Ok(trimmed.to_owned());
    }

    if lower.starts_with("local://")
        || lower.starts_with("asset://")
        || lower.starts_with("file://")
        || trimmed.starts_with("/assets/")
    {
        if trimmed.contains("..") || trimmed.contains('<') || trimmed.contains('>') {
            return Err(if en {
                "Invalid local asset path for pet photo.".to_owned()
            } else {
                "灵宠本地图片路径包含不安全字符。".to_owned()
            });
        }
        return Ok(trimmed.to_owned());
    }

    Err(if en {
        "Pet photo must be a local image data URL (PNG/JPEG/WebP/GIF) or local:// / asset:// reference.".to_owned()
    } else {
        "灵宠头像仅支持本地图片 Data URL（PNG/JPEG/WebP/GIF）或 local:// / asset:// 本地资源路径。".to_owned()
    })
}

pub fn compute_custom_pet_quota(
    used_count: usize,
    ent: &EntitlementVerificationResult,
) -> CustomPetQuota {
    let allowed = custom_pet_quota(ent);
    let remaining_add_slots = allowed.saturating_sub(used_count);
    CustomPetQuota {
        used_count,
        allowed_count: allowed,
        remaining_add_slots,
        can_add_more: used_count < allowed,
        is_pro_ex_libris_unlocked: has_feature(ent, "custom_pet_ex_libris"),
        can_generate_weekly_letter: has_feature(ent, "pet_weekly_letters"),
        retained_above_free_quota: used_count > 1 && allowed == 1,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolarAtelierPayload {
    pub solar_term: SolarTermState,
    pub dew_balance: i64,
    pub dew_ledger: Vec<DewLedgerEntry>,
    pub seasonal_pass: Vec<SeasonalPassNode>,
    pub keepsake_catalog: KeepsakeCatalog,
    pub presets: Vec<SanctuaryPreset>,
    pub preset_limit: Option<usize>,
    pub stickers: Vec<FoodSticker>,
    pub sticker_quota: StickerMonthlyQuota,
    pub weekly_letters: Vec<CompanionWeeklyLetter>,
    pub co_sanctuary: CoSanctuaryState,
    pub widget_preview: WidgetStudioPayload,
    pub four_seasons_scroll: Vec<FourSeasonsScrollNode>,
    pub ceramic_badges: Vec<CeramicBadge>,
    pub custom_pets: Vec<CustomPetCompanion>,
    pub active_custom_pet: Option<CustomPetCompanion>,
    pub custom_pet_quota: CustomPetQuota,
    pub pet_weekly_letters: Vec<PetWeeklyLetter>,
}

/// 24 节气公历起始近似月日（按全年顺序：小寒 1/5 .. 冬至 12/21）
const TERM_ANCHORS: [(u32, u32, &str, &str, &str, &str); 24] = [
    (1, 5, "小寒", "winter", "冬", "寒风初凛，宜早卧晚起、护住肩颈与足底暖意。"),
    (1, 20, "大寒", "winter", "冬", "岁末极寒，把热茶沏在手边，静候春信归来。"),
    (2, 4, "立春", "spring", "春", "东风解冻，晨起伸展两分钟，让肩背随春意舒展。"),
    (2, 19, "雨水", "spring", "春", "细雨润窗棂，今日宜温食淡饮、少生冷油腻。"),
    (3, 6, "惊蛰", "spring", "春", "春雷初响，午后若困倦不妨闭目养神一刻钟。"),
    (3, 21, "春分", "spring", "春", "昼夜均分，今日作息宜守中道，不急不躁。"),
    (4, 5, "清明", "spring", "春", "气清景明，趁日光未烈去户外走一走吧。"),
    (4, 20, "谷雨", "spring", "春", "雨生百谷，湿气渐起，晚餐清简最养脾胃。"),
    (5, 6, "立夏", "summer", "夏", "绿树阴浓，心神易浮，睡前留半小时远离屏幕。"),
    (5, 21, "小满", "summer", "夏", "物致于此小得盈满，愿你今天的元气也恰好充盈。"),
    (6, 6, "芒种", "summer", "夏", "仲夏忙而不乱，午间小憩 20 分钟最能回血。"),
    (6, 21, "夏至", "summer", "夏", "白昼最长之日，别把工作拖到深夜，留给夜色一点余白。"),
    (7, 7, "小暑", "summer", "夏", "暑气渐蒸，补水别等口渴，温白开比冰饮更护肠胃。"),
    (7, 23, "大暑", "summer", "夏", "蝉鸣最盛时，运动宜避开正午烈日，以微汗为度。"),
    (8, 7, "立秋", "autumn", "秋", "一叶知秋意，入夜渐凉，记得给窗台留一盏暖茶。"),
    (8, 23, "处暑", "autumn", "秋", "暑气至此而止，早睡二十分钟能明显稳住次日 HRV。"),
    (9, 8, "白露", "autumn", "秋", "露凝而白，夜凉添衣，睡前温水泡脚格外安神。"),
    (9, 23, "秋分", "autumn", "秋", "金风玉露平分秋色，是开启 14 天微习惯实验的好时节。"),
    (10, 8, "寒露", "autumn", "秋", "寒露润秋燥，梨汤与温粥胜过深夜辛辣烧烤。"),
    (10, 23, "霜降", "autumn", "秋", "霜叶红时，敛藏阳气，高强度训练后更要重视拉伸。"),
    (11, 7, "立冬", "winter", "冬", "水始冰、地始冻，书房点一盏小灯，安心把节奏放缓。"),
    (11, 22, "小雪", "winter", "冬", "微雪将至，晚饭早些吃，好眠就是最好的冬藏。"),
    (12, 7, "大雪", "winter", "冬", "围炉煮茶最相宜，今天不必硬撑着追步数。"),
    (12, 22, "冬至", "winter", "冬", "一阳初生，今夜最长，愿你安稳睡个好觉。"),
];

pub fn normalize_weather_mood(raw: &str) -> &'static str {
    match raw.trim() {
        "misty_rain" => "misty_rain",
        "first_snow" => "first_snow",
        "clear_moon" => "clear_moon",
        _ => "sunny_crisp",
    }
}

const TERM_ANCHORS_EN: [(&str, &str, &str); 24] = [
    ("Minor Cold", "Winter", "Crisp winter winds invite earlier nights and keeping your neck and feet warm."),
    ("Major Cold", "Winter", "Steep a warm mug nearby as winter peaks and wait gently for spring's arrival."),
    ("Start of Spring", "Spring", "An early breeze begins to thaw the chill; stretch for two minutes after waking."),
    ("Rain Water", "Spring", "Soft rain settles on the windowsill—favor warm, light meals over chilled or heavy dishes."),
    ("Awakening of Insects", "Spring", "Early thunder stirs; a quiet fifteen-minute afternoon rest refreshes your focus."),
    ("Spring Equinox", "Spring", "Day and night stand in balance. Keep your pace steady and unhurried today."),
    ("Pure Brightness", "Spring", "Air clears under soft sunlight; step outside for an easy walk before noon glare."),
    ("Grain Rain", "Spring", "Spring rain nourishes young buds while humidity rises—keep dinner simple and warm."),
    ("Start of Summer", "Summer", "Deepening greens bring warmer air; step away from bright screens thirty minutes before sleep."),
    ("Grain Buds", "Summer", "Everything swells toward modest fullness; may your energy feel just right today."),
    ("Grain in Ear", "Summer", "Midday busyness settles easily with a calm twenty-minute midday rest."),
    ("Summer Solstice", "Summer", "On the year's longest day, leave evening unhurried instead of pushing late into the night."),
    ("Minor Heat", "Summer", "Summer warmth builds—sip warm or room-temperature water steadily before feeling thirsty."),
    ("Major Heat", "Summer", "At the height of summer, keep movement gentle and avoid harsh midday sun."),
    ("Start of Autumn", "Autumn", "A cool evening breeze whispers autumn's turn; keep a warm tea cup by your desk."),
    ("End of Heat", "Autumn", "Lingering heat softens; turning in twenty minutes earlier noticeably steadies overnight HRV."),
    ("White Dew", "Autumn", "Cool dew gathers after dusk—layer up and soak your feet in warm water before bed."),
    ("Autumn Equinox", "Autumn", "Balanced light and crisp air make an ideal window to start a 14-day rhythm experiment."),
    ("Cold Dew", "Autumn", "Warm pear broth or simple congee soothes autumn dryness better than late-night spice."),
    ("Frost's Descent", "Autumn", "Save your warmth as frost arrives, and linger a little longer on gentle post-session stretches."),
    ("Start of Winter", "Winter", "Light a warm desk lamp as stillness settles in, and let your daily tempo soften."),
    ("Minor Snow", "Winter", "With early flurries near, an earlier dinner gives you deeper restorative rest."),
    ("Major Snow", "Winter", "Cozy tea by the hearth fits today far better than forcing an extra step goal."),
    ("Winter Solstice", "Winter", "On the year's longest night, drift off gently and let deeper stillness renew your balance."),
];

fn lunar_phase_for_date_localized(date: NaiveDate, locale: &str) -> (String, u32) {
    // 以 2000-01-06 朔日为基准，按平均朔望月 29.53058867 天估算月相
    let ref_new_moon = NaiveDate::from_ymd_opt(2000, 1, 6).expect("valid ref date");
    let diff_days = (date - ref_new_moon).num_days() as f64;
    let synodic = 29.53058867;
    let age = ((diff_days % synodic) + synodic) % synodic;
    let lunar_day = (age.floor() as u32).clamp(0, 29) + 1;
    let en = is_en_locale(locale);
    let phase = if age < 1.85 || age >= 27.68 {
        if en { "New Moon" } else { "朔月" }
    } else if age < 5.54 {
        if en { "Waxing Crescent" } else { "蛾眉月" }
    } else if age < 9.23 {
        if en { "First Quarter" } else { "上弦月" }
    } else if age < 12.92 {
        if en { "Waxing Gibbous" } else { "盈凸月" }
    } else if age < 16.61 {
        if en { "Full Moon" } else { "望月" }
    } else if age < 20.30 {
        if en { "Waning Gibbous" } else { "亏凸月" }
    } else if age < 23.99 {
        if en { "Last Quarter" } else { "下弦月" }
    } else if en {
        "Waning Crescent"
    } else {
        "残月"
    };
    (phase.to_owned(), lunar_day)
}

/// ① 计算 24 节气、月相、天色心境与对应动态书房世界。
pub fn compute_solar_term_state(date_str: &str, weather_mood: &str) -> SolarTermState {
    compute_solar_term_state_localized(date_str, weather_mood, DEFAULT_LOCALE)
}

pub fn compute_solar_term_state_localized(
    date_str: &str,
    weather_mood: &str,
    locale: &str,
) -> SolarTermState {
    let en = is_en_locale(locale);
    let date = NaiveDate::parse_from_str(date_str.trim(), "%Y-%m-%d")
        .unwrap_or_else(|_| NaiveDate::from_ymd_opt(2026, 11, 13).expect("valid fallback date"));
    let md = (date.month0() + 1) * 100 + date.day();

    // 找到 <= md 的最后一个节气；若小于 105 (小寒) 则落在上一年的 冬至 (index 23)
    let mut matched_idx = 23usize;
    for (idx, &(m, d, _, _, _, _)) in TERM_ANCHORS.iter().enumerate() {
        if md >= m * 100 + d {
            matched_idx = idx;
        }
    }

    let (_, _, term_name_zh, season, season_cn_zh, base_whisper_zh) = TERM_ANCHORS[matched_idx];
    let (term_name_en, season_en, base_whisper_en) = TERM_ANCHORS_EN[matched_idx];
    let (lunar_phase, lunar_day_approx) = lunar_phase_for_date_localized(date, locale);
    let mood = normalize_weather_mood(weather_mood);

    let active_world = match mood {
        "misty_rain" => "solar_spring_rain",
        "first_snow" => "solar_winter_hearth",
        _ => match season {
            "spring" => "solar_spring_garden",
            "summer" => "solar_summer_lotus",
            "autumn" => "solar_autumn_osmanthus",
            _ => "solar_winter_study",
        },
    };

    let (term_name, season_label, whisper) = if en {
        let mood_hint_en = match mood {
            "misty_rain" => "Soft mist drifts across the pane—brew a warm cup and listen to the rain.",
            "first_snow" => "First snow settles on the eaves while the hearth glows warmly; let your shoulders drop.",
            "clear_moon" => "Clear moonlight cools the courtyard; let your thoughts settle and rest early tonight.",
            _ => "Clear daylight fills the window—move smoothly with your breath today.",
        };
        (
            term_name_en.to_owned(),
            season_en.to_owned(),
            format!("{term_name_en} · {lunar_phase}: {base_whisper_en} {mood_hint_en}"),
        )
    } else {
        let mood_hint = match mood {
            "misty_rain" => "窗外烟雨微凉，适合泡一盏热茶静听雨声。",
            "first_snow" => "初雪落檐，炉火正暖，先把紧绷的肩膀放下来。",
            "clear_moon" => "月色澄澈如水，今夜宜少思虑、早歇息。",
            _ => "天光晴朗通透，顺着呼吸慢慢来就很好。",
        };
        (
            term_name_zh.to_owned(),
            season_cn_zh.to_owned(),
            format!("{term_name_zh} · {lunar_phase}：{base_whisper_zh}{mood_hint}"),
        )
    };

    SolarTermState {
        date: date.format("%Y-%m-%d").to_string(),
        term_name,
        term_index: matched_idx,
        season: season.to_owned(),
        season_cn: season_label,
        lunar_phase,
        lunar_day_approx,
        weather_mood: mood.to_owned(),
        active_world: active_world.to_owned(),
        whisper,
    }
}

pub fn seasonal_pass_definitions() -> Vec<(&'static str, &'static str, &'static str, &'static str, &'static str, i64, &'static str)> {
    // (id, title, track, reward_kind, reward_id, cost_dew, evergreen_term)
    vec![
        ("pass_free_1", "青瓷茶盏", "free", "keepsake", "tea_cup", 2, "立冬"),
        ("pass_free_2", "暖绒常服", "free", "outfit", "warm_robe", 3, "立冬"),
        ("pass_free_3", "朱砂印泥", "free", "seal_ink", "cinnabar", 0, "小雪"),
        ("pass_free_4", "第 1 封伴侣周信", "free", "letter", "week_1", 0, "立冬"),
        ("pass_atelier_1", "烟雨青蓑衣", "atelier", "outfit", "rain_cape", 3, "雨水"),
        ("pass_atelier_2", "初雪白狐裘", "atelier", "outfit", "snow_cloak", 4, "小雪"),
        ("pass_atelier_3", "荷风轻纱巾", "atelier", "outfit", "lotus_scarf", 4, "小满"),
        ("pass_atelier_4", "星夜观月兜帽", "atelier", "outfit", "starlight_hood", 5, "冬至"),
        ("pass_atelier_5", "赤金泥金印", "atelier", "seal_ink", "gold", 3, "秋分"),
        ("pass_atelier_6", "青金石秘色印", "atelier", "seal_ink", "lapis", 4, "白露"),
        ("pass_atelier_7", "窗台金桂小瓶", "atelier", "keepsake", "osmanthus", 2, "寒露"),
        ("pass_atelier_8", "暖纸安眠夜灯", "atelier", "keepsake", "night_lantern", 3, "冬至"),
    ]
}

fn translate_pass_item_en(id: &str, title_zh: &str, term_zh: &str) -> (&'static str, &'static str) {
    let title = match id {
        "pass_free_1" => "Celadon teacup",
        "pass_free_2" => "Warm fleece robe",
        "pass_free_3" => "Cinnabar seal ink",
        "pass_free_4" => "Week 1 companion letter",
        "pass_atelier_1" => "Misty rain cape",
        "pass_atelier_2" => "First snow cloak",
        "pass_atelier_3" => "Lotus breeze scarf",
        "pass_atelier_4" => "Starlight moon hood",
        "pass_atelier_5" => "Song gold seal ink",
        "pass_atelier_6" => "Lapis lazuli seal ink",
        "pass_atelier_7" => "Window osmanthus vase",
        "pass_atelier_8" => "Warm paper night lantern",
        _ => match title_zh {
            _ => "Seasonal keepsake",
        },
    };
    let term = match term_zh {
        "立冬" => "Start of Winter",
        "小雪" => "Minor Snow",
        "雨水" => "Rain Water",
        "小满" => "Grain Buds",
        "冬至" => "Winter Solstice",
        "秋分" => "Autumn Equinox",
        "白露" => "White Dew",
        "寒露" => "Cold Dew",
        _ => "Start of Winter",
    };
    (title, term)
}

pub fn build_seasonal_pass(
    ent: &EntitlementVerificationResult,
    claimed_rewards: &[String],
) -> Vec<SeasonalPassNode> {
    build_seasonal_pass_localized(ent, claimed_rewards, DEFAULT_LOCALE)
}

pub fn build_seasonal_pass_localized(
    ent: &EntitlementVerificationResult,
    claimed_rewards: &[String],
    locale: &str,
) -> Vec<SeasonalPassNode> {
    let en = is_en_locale(locale);
    let atelier_ok = has_feature(ent, "atelier_seasonal_track");
    seasonal_pass_definitions()
        .into_iter()
        .map(|(id, title, track, reward_kind, reward_id, cost_dew, term)| {
            let (disp_title, disp_term) = if en {
                let (t, s) = translate_pass_item_en(id, title, term);
                (t.to_owned(), s.to_owned())
            } else {
                (title.to_owned(), term.to_owned())
            };
            SeasonalPassNode {
                id: id.to_owned(),
                title: disp_title,
                track: track.to_owned(),
                reward_kind: reward_kind.to_owned(),
                reward_id: reward_id.to_owned(),
                cost_dew,
                claimed: claimed_rewards.iter().any(|r| r == id || r == reward_id),
                unlocked_by_tier: track == "free" || atelier_ok,
                evergreen_term: disp_term,
            }
        })
        .collect()
}

pub fn build_keepsake_catalog(
    settings: &AppSettings,
    ent: &EntitlementVerificationResult,
    claimed_rewards: &[String],
) -> KeepsakeCatalog {
    build_keepsake_catalog_localized(settings, ent, claimed_rewards, DEFAULT_LOCALE)
}

pub fn build_keepsake_catalog_localized(
    settings: &AppSettings,
    ent: &EntitlementVerificationResult,
    claimed_rewards: &[String],
    locale: &str,
) -> KeepsakeCatalog {
    let en = is_en_locale(locale);
    let atelier_ok = has_feature(ent, "atelier_seasonal_track");
    let allowed_slots = if has_feature(ent, "keepsake_slot_3") {
        MAX_EQUIPPED_KEEPSAKES
    } else {
        2
    };
    let equipped_ids: Vec<String> = serde_json::from_str::<Vec<String>>(&settings.equipped_keepsakes_json)
        .unwrap_or_default()
        .into_iter()
        .take(MAX_EQUIPPED_KEEPSAKES)
        .collect();

    let raw_items = if en {
        [
            ("tea_cup", "Celadon teacup", "Warm the first sip by the window", "free"),
            ("osmanthus", "Osmanthus vase", "Quiet autumn fragrance on the sill", "atelier"),
            ("cushion", "Woven floor cushion", "Cozy spot for afternoon naps", "free"),
            ("night_lantern", "Warm paper lantern", "Soft amber light for rainy nights", "atelier"),
            ("celadon_cup", "Ru-ware tea bowl", "Steady calm in a clear porcelain cup", "free"),
            ("pine_twig", "Courtyard pine sprig", "Evergreen reminder to breathe deeply", "atelier"),
            ("cinnabar_seal", "Shoushan stone seal", "Leave a small vermilion mark each day", "free"),
            ("stone_lantern", "Garden stone lantern", "Quiet glow while listening to evening rain", "atelier"),
        ]
    } else {
        [
            ("tea_cup", "天青茶盏", "点亮第一缕暖茶香", "free"),
            ("osmanthus", "金桂小瓶", "窗台秋意暗香来", "atelier"),
            ("cushion", "编织软垫", "小满午睡更松软", "free"),
            ("night_lantern", "暖纸夜灯", "雨夜亮一盏安眠灯", "atelier"),
            ("celadon_cup", "汝窑香茗", "一盏清茶定心神", "free"),
            ("pine_twig", "迎客苍松", "四时青翠守呼吸", "atelier"),
            ("cinnabar_seal", "寿山闲章", "每日钤印留清记", "free"),
            ("stone_lantern", "庭院石灯", "听雨观心夜微明", "atelier"),
        ]
    };
    let items = raw_items
        .into_iter()
        .map(|(id, name, hint, track)| KeepsakeCatalogItem {
            id: id.to_owned(),
            name: name.to_owned(),
            hint: hint.to_owned(),
            track: track.to_owned(),
            unlocked: track == "free" || atelier_ok || claimed_rewards.iter().any(|r| r == id),
            equipped: equipped_ids.iter().any(|e| e == id),
        })
        .collect();

    let outfits = if en {
        vec![
            OutfitSpec {
                id: "warm_robe".into(),
                name: "Warm linen robe".into(),
                desc: "Soft natural cotton weave for cool mornings".into(),
                track: "free".into(),
                unlocked: true,
            },
            OutfitSpec {
                id: "rain_cape".into(),
                name: "Misty rain cape".into(),
                desc: "Atelier cloak made for quiet rainy afternoons".into(),
                track: "atelier".into(),
                unlocked: atelier_ok || claimed_rewards.iter().any(|r| r == "rain_cape"),
            },
            OutfitSpec {
                id: "snow_cloak".into(),
                name: "First snow cloak".into(),
                desc: "Plush winter cape to wind down early".into(),
                track: "atelier".into(),
                unlocked: atelier_ok || claimed_rewards.iter().any(|r| r == "snow_cloak"),
            },
            OutfitSpec {
                id: "lotus_scarf".into(),
                name: "Lotus breeze scarf".into(),
                desc: "Breezy summer silk wrap for warmer evenings".into(),
                track: "atelier".into(),
                unlocked: atelier_ok || claimed_rewards.iter().any(|r| r == "lotus_scarf"),
            },
            OutfitSpec {
                id: "starlight_hood".into(),
                name: "Starlight moon hood".into(),
                desc: "Solstice embroidery sewn for long winter nights".into(),
                track: "atelier".into(),
                unlocked: atelier_ok || claimed_rewards.iter().any(|r| r == "starlight_hood"),
            },
        ]
    } else {
        vec![
            OutfitSpec {
                id: "warm_robe".into(),
                name: "暖绒常服".into(),
                desc: "素色棉麻织锦，入冬护暖必备".into(),
                track: "free".into(),
                unlocked: true,
            },
            OutfitSpec {
                id: "rain_cape".into(),
                name: "烟雨青蓑衣".into(),
                desc: "听雨书房限定披风，不惧春雨微寒".into(),
                track: "atelier".into(),
                unlocked: atelier_ok || claimed_rewards.iter().any(|r| r == "rain_cape"),
            },
            OutfitSpec {
                id: "snow_cloak".into(),
                name: "初雪白狐裘".into(),
                desc: "大雪冬藏软绒斗篷，陪你早早入眠".into(),
                track: "atelier".into(),
                unlocked: atelier_ok || claimed_rewards.iter().any(|r| r == "snow_cloak"),
            },
            OutfitSpec {
                id: "lotus_scarf".into(),
                name: "荷风轻纱巾".into(),
                desc: "小满清夏轻纱，通风透气又轻盈".into(),
                track: "atelier".into(),
                unlocked: atelier_ok || claimed_rewards.iter().any(|r| r == "lotus_scarf"),
            },
            OutfitSpec {
                id: "starlight_hood".into(),
                name: "星夜观月兜帽".into(),
                desc: "冬至长夜星辰纹饰，护住甜梦".into(),
                track: "atelier".into(),
                unlocked: atelier_ok || claimed_rewards.iter().any(|r| r == "starlight_hood"),
            },
        ]
    };

    let seal_unlock = has_feature(ent, "poster_custom_seals");
    let seal_inks = vec![
        SealInkSpec {
            id: "cinnabar".into(),
            name: if en { "Classic cinnabar".into() } else { "传统朱砂红".into() },
            hex_hint: "#C94A38".into(),
            unlocked: true,
        },
        SealInkSpec {
            id: "gold".into(),
            name: if en { "Song gold paste".into() } else { "宋金泥金印".into() },
            hex_hint: "#C59B38".into(),
            unlocked: seal_unlock || claimed_rewards.iter().any(|r| r == "gold"),
        },
        SealInkSpec {
            id: "lapis".into(),
            name: if en { "Lapis lazuli".into() } else { "青金石秘色".into() },
            hex_hint: "#2D5B88".into(),
            unlocked: seal_unlock || claimed_rewards.iter().any(|r| r == "lapis"),
        },
    ];

    KeepsakeCatalog {
        max_equipped_slots: MAX_EQUIPPED_KEEPSAKES,
        allowed_slots_for_tier: allowed_slots,
        equipped_ids,
        items,
        outfits,
        seal_inks,
        active_seal_ink: if settings.active_seal_ink.trim().is_empty() {
            "cinnabar".to_owned()
        } else {
            settings.active_seal_ink.clone()
        },
    }
}

/// ③ 食光贴纸轻营养提示（嘌呤 / 钠 / 糖分与咖啡因温和提示）。
pub fn infer_sticker_nutrition_hint(title: &str) -> String {
    infer_sticker_nutrition_hint_localized(title, DEFAULT_LOCALE)
}

pub fn infer_sticker_nutrition_hint_localized(title: &str, locale: &str) -> String {
    let en = is_en_locale(locale);
    let lower = title.to_ascii_lowercase();
    let is_purine = ["海鲜", "牡蛎", "生蚝", "花甲", "基围虾", "螃蟹", "浓汤", "火锅", "啤酒", "内脏", "烧烤", "猪肝", "肉汤"]
        .iter()
        .any(|k| title.contains(k))
        || ["seafood", "oyster", "shrimp", "crab", "hotpot", "beer", "broth", "bbq", "liver"]
            .iter()
            .any(|k| lower.contains(k));
    let is_sodium = ["咸", "腊", "卤", "咸菜", "泡菜", "拉面", "酱", "麻辣烫", "外卖", "炸鸡", "重口"]
        .iter()
        .any(|k| title.contains(k))
        || ["ramen", "kimchi", "takeout", "fried chicken", "salty", "pickle", "soy"]
            .iter()
            .any(|k| lower.contains(k));
    let is_sweet_caffeine = ["奶茶", "拿铁", "咖啡", "甜品", "蛋糕", "冰淇淋", "可乐"]
        .iter()
        .any(|k| title.contains(k))
        || ["latte", "coffee", "espresso", "bubble tea", "boba", "cake", "ice cream", "cola", "matcha"]
            .iter()
            .any(|k| lower.contains(k));
    let is_light = ["清蒸", "白灼", "沙拉", "青菜", "燕麦", "清茶", "豆浆", "粥", "水煮蛋", "三文鱼", "藜麦"]
        .iter()
        .any(|k| title.contains(k))
        || ["steamed", "salad", "greens", "oatmeal", "oats", "tea", "congee", "salmon", "quinoa", "egg"]
            .iter()
            .any(|k| lower.contains(k));

    if en {
        if is_purine {
            "Purine cue: richer in purines—pair with plenty of warm water and dark leafy greens.".to_owned()
        } else if is_sodium {
            "Sodium cue: savory seasoning detected—follow with warm water and potassium-rich greens.".to_owned()
        } else if is_sweet_caffeine {
            "Sugar & caffeine cue: savor in moderation, and taper caffeine after 15:00 for deeper sleep.".to_owned()
        } else if is_light {
            "Light & balanced: low purine, gentle on digestion, and supportive of calm daily rhythms.".to_owned()
        } else {
            "Daily meal note: eat unhurriedly to comfortable fullness and enjoy the warmth of the meal.".to_owned()
        }
    } else if is_purine {
        "嘌呤提示：偏中高嘌呤风味，建议搭配足量温水与深色叶菜帮助代谢。".to_owned()
    } else if is_sodium {
        "钠含量提示：调味相对偏浓郁，餐后注意补充温水与富含钾的新鲜蔬果。".to_owned()
    } else if is_sweet_caffeine {
        "糖分与咖啡因提示：浅尝即止更自在，15:00 后减少咖啡因更护深睡。".to_owned()
    } else if is_light {
        "清欢轻食：低脂低嘌呤且温润好吸收，非常贴合日常养气节律。".to_owned()
    } else {
        "日常食光记录：细嚼慢咽、八分饱足，好好享受这一餐的温度。".to_owned()
    }
}

pub fn compute_sticker_quota(
    year_month: &str,
    used_in_month: usize,
    ent: &EntitlementVerificationResult,
) -> StickerMonthlyQuota {
    let unlimited = has_feature(ent, "unlimited_stickers");
    if unlimited {
        StickerMonthlyQuota {
            year_month: year_month.to_owned(),
            used_count: used_in_month,
            limit: None,
            remaining: None,
            unlimited: true,
            can_create: true,
        }
    } else {
        let rem = FREE_STICKER_MONTHLY_QUOTA.saturating_sub(used_in_month);
        StickerMonthlyQuota {
            year_month: year_month.to_owned(),
            used_count: used_in_month,
            limit: Some(FREE_STICKER_MONTHLY_QUOTA),
            remaining: Some(rem),
            unlimited: false,
            can_create: used_in_month < FREE_STICKER_MONTHLY_QUOTA,
        }
    }
}

pub fn companion_profile(companion_id: &str) -> (&'static str, &'static str) {
    companion_profile_localized(companion_id, DEFAULT_LOCALE)
}

pub fn companion_profile_localized(companion_id: &str, locale: &str) -> (&'static str, &'static str) {
    if is_en_locale(locale) {
        match companion_id {
            "shanfang" | "zen_garden" => ("shanfang", "Study Pine & Stone Keeper"),
            "meiqiou" | "playful_bento" => ("meiqiou", "Inky Cat Charcoal"),
            _ => ("xiaoman", "Calico Cat Xiao Man"),
        }
    } else {
        match companion_id {
            "shanfang" | "zen_garden" => ("shanfang", "山房松石守望者"),
            "meiqiou" | "playful_bento" => ("meiqiou", "软胶猫「煤球」"),
            _ => ("xiaoman", "三花猫「小满」"),
        }
    }
}

/// ④ 生成 4 封个性化伴侣周信与节气藏书票。
/// 规则：第 1 封永远免费解锁；第 2-4 封在 Atelier 有效时或 `ExpiredReadOnlyVault`（曾解锁过）下可读。
pub fn build_companion_weekly_letters(
    companion_id: &str,
    snapshot: &AnalyticsSnapshot,
    solar: &SolarTermState,
    intakes_count: usize,
    stickers_count: usize,
    ent: &EntitlementVerificationResult,
) -> Vec<CompanionWeeklyLetter> {
    build_companion_weekly_letters_localized(
        companion_id,
        snapshot,
        solar,
        intakes_count,
        stickers_count,
        ent,
        DEFAULT_LOCALE,
    )
}

pub fn build_companion_weekly_letters_localized(
    companion_id: &str,
    snapshot: &AnalyticsSnapshot,
    solar: &SolarTermState,
    intakes_count: usize,
    stickers_count: usize,
    ent: &EntitlementVerificationResult,
    locale: &str,
) -> Vec<CompanionWeeklyLetter> {
    let en = is_en_locale(locale);
    let (cid, cname) = companion_profile_localized(companion_id, locale);
    let archive_unlocked = has_feature(ent, "weekly_letters_archive")
        || (ent.state == OfflineEntitlementState::ExpiredReadOnlyVault && ent.valid_signature);

    let safe_badge = if en && contains_cjk(&snapshot.state_badge) {
        match snapshot.state.as_str() {
            "prime_ready" => "Well rested".to_owned(),
            "recovery_priority" => "Rest gently".to_owned(),
            _ => "Balanced".to_owned(),
        }
    } else {
        snapshot.state_badge.clone()
    };

    if en {
        let sleep_str = snapshot
            .sleep_last_night_hours
            .map(|h| format!("{h:.1} hours"))
            .unwrap_or_else(|| "a gentle pace".to_owned());
        let metric_str = snapshot
            .metrics
            .get("hrv_rmssd_ms")
            .or_else(|| snapshot.metrics.get("hrv_sdnn_ms"))
            .and_then(|m| m.value)
            .map(|v| format!("overnight HRV of {v:.0} ms"))
            .or_else(|| {
                snapshot
                    .metrics
                    .get("steps")
                    .and_then(|m| m.value)
                    .map(|s| format!("{:.0} daily steps", s))
            })
            .unwrap_or_else(|| "steady rhythm logs".to_owned());

        let tone_prefix = match cid {
            "shanfang" => "Fresh tea is steaming on the pine table as I unroll a sheet for you: ",
            "meiqiou" => "Mew! I curled up right beside your notebook with warm paws to write: ",
            _ => "After stretching out in afternoon sunlight on the sill, I tucked this letter in the post: ",
        };

        let first_signal = snapshot
            .signals
            .first()
            .filter(|s| !contains_cjk(s))
            .cloned()
            .unwrap_or_else(|| "steady rhythm baseline".to_owned());

        let specs = [
            (
                1_u32,
                format!("Letter 1 · First arrival under {}", solar.term_name),
                solar.term_name.clone(),
                format!("{} Note · Ex Libris No.01", solar.season_cn),
                "Quiet Beginning".to_owned(),
                format!(
                    "{tone_prefix}We shared {sleep_str} of night stillness and {metric_str} this week, settling at a vitality score of {} ({safe_badge}). Never chase perfect streaks—{intakes_count} mindful meals and one quiet pause already make a fine week.",
                    snapshot.qi_score
                ),
                true,
                None,
            ),
            (
                2_u32,
                format!("Letter 2 · Meal polaroids & {}", solar.lunar_phase),
                solar.term_name.clone(),
                format!("{} Table · Ex Libris No.02", solar.season_cn),
                "Warm Table".to_owned(),
                format!(
                    "{tone_prefix}You have saved {stickers_count} food stickers and {intakes_count} gentle meal notes. Pairing {metric_str} with {sleep_str} of rest shows how kindly your body answers calm, unhurried evenings.",
                ),
                archive_unlocked,
                if archive_unlocked {
                    None
                } else {
                    Some("Unlock Qi Atelier for weekly Sunday letters and seasonal Ex Libris stamps from week 2 onward.".to_owned())
                },
            ),
            (
                3_u32,
                "Letter 3 · Quiet patterns behind the numbers".to_owned(),
                "Frost's Descent".to_owned(),
                "Autumn Dew Ledger · Ex Libris No.03".to_owned(),
                "Balanced Poise".to_owned(),
                format!(
                    "{tone_prefix}Your clearest rhythm note these past three weeks has been \"{first_signal}\". On tired nights you never lose progress here—rest days earn extra dew so numbers support calm rather than pressure.",
                ),
                archive_unlocked,
                if archive_unlocked {
                    None
                } else {
                    Some("Unlock Qi Atelier to read archived weekly letters and Ex Libris seals.".to_owned())
                },
            ),
            (
                4_u32,
                "Letter 4 · Moonlight at the window across seasons".to_owned(),
                "Winter Solstice".to_owned(),
                "Four Seasons Seal · Ex Libris No.04".to_owned(),
                "Kindred Spirit".to_owned(),
                format!(
                    "{tone_prefix}Looking back over {intakes_count} notes and today's vitality of {} ({safe_badge}), what shines brightest is never pushing harder on hard days—it is knowing when to pause gently. Tea is always warm by your window.",
                    snapshot.qi_score
                ),
                archive_unlocked,
                if archive_unlocked {
                    None
                } else {
                    Some("Unlock Qi Atelier for the full 52-week letter archive (unlocked letters stay readable forever).".to_owned())
                },
            ),
        ];

        return specs
            .into_iter()
            .map(
                |(week_number, title, term, ex_title, ex_seal, body, unlocked, lock_reason)| {
                    CompanionWeeklyLetter {
                        week_number,
                        title,
                        companion_id: cid.to_owned(),
                        companion_name: cname.to_owned(),
                        solar_term: term,
                        ex_libris_title: ex_title,
                        ex_libris_seal: ex_seal,
                        body: if unlocked {
                            body
                        } else {
                            "This handwritten companion letter rests sealed in the postbox. Unlock Qi Atelier to read it and preserve it in your read-only vault forever.".to_owned()
                        },
                        unlocked,
                        lock_reason,
                    }
                },
            )
            .collect();
    }

    let sleep_str = snapshot
        .sleep_last_night_hours
        .map(|h| format!("{h:.1} 小时"))
        .unwrap_or_else(|| "按体感休息".to_owned());
    let metric_str = snapshot
        .metrics
        .get("hrv_rmssd_ms")
        .or_else(|| snapshot.metrics.get("hrv_sdnn_ms"))
        .and_then(|m| m.value)
        .map(|v| format!("夜间 HRV {v:.0} ms"))
        .or_else(|| {
            snapshot
                .metrics
                .get("steps")
                .and_then(|m| m.value)
                .map(|s| format!("日行 {:.0} 步", s))
        })
        .unwrap_or_else(|| "顺应节律记录".to_owned());

    let tone_prefix = match cid {
        "shanfang" => "石案清茶刚微温，展纸给你写几句：",
        "meiqiou" => "喵呜！我刚踩着软垫在手账本边趴好：",
        _ => "窗台晒过午后太阳，我抖抖胡须给你寄来这封慢递：",
    };

    let specs = [
        (
            1_u32,
            format!("第一封 · {}初见记", solar.term_name),
            solar.term_name.clone(),
            format!("{}初雪笺 · 藏书票 No.01", solar.season_cn),
            "素心初见".to_owned(),
            format!(
                "{tone_prefix}这周陪你一起看过了 {sleep_str} 的夜色与 {metric_str}，今天你的元气分数停在 {} 分（{}）。不必追求满分打卡，能安稳吃好 {intakes_count} 顿饭、留下一两张生活微光，就已经非常棒啦。",
                snapshot.qi_score, safe_badge
            ),
            true,
            None,
        ),
        (
            2_u32,
            format!("第二封 · 饮食贴纸与 {} 夜话", solar.lunar_phase),
            solar.term_name.clone(),
            format!("{}食光集 · 藏书票 No.02", solar.season_cn),
            "食光留香".to_owned(),
            format!(
                "{tone_prefix}不知不觉你已经留存了 {stickers_count} 张食光贴纸，加上 {intakes_count} 条日常饮食印记。{metric_str} 与昨晚 {sleep_str} 告诉我，你的身体很喜欢温和规律的晚间收尾。继续这样不紧不慢地照顾自己吧。",
            ),
            archive_unlocked,
            if archive_unlocked {
                None
            } else {
                Some("知气雅集解锁第 2 周起全部周日夜话慢递与节气藏书票".to_owned())
            },
        ),
        (
            3_u32,
            "第三封 · 从统计到生活归因的小发现".to_owned(),
            "霜降".to_owned(),
            "秋露归因录 · 藏书票 No.03".to_owned(),
            "明理守中".to_owned(),
            format!(
                "{tone_prefix}这三周里，你的核心信号是「{}」。哪怕偶尔有一两天睡得短了，系统也不会扣掉你的心意，反而希望你在疲惫日安心多攒几颗晨露。记住，数据是来帮我们放松而不是制造焦虑的。",
                snapshot.signals.first().cloned().unwrap_or_else(|| "节律稳步积累".to_owned())
            ),
            archive_unlocked,
            if archive_unlocked {
                None
            } else {
                Some("知气雅集解锁往期周信归档与藏书票收藏".to_owned())
            },
        ),
        (
            4_u32,
            "第四封 · 满月窗前与四时之约".to_owned(),
            "冬至".to_owned(),
            "四时同行印 · 藏书票 No.04".to_owned(),
            "四时知己".to_owned(),
            format!(
                "{tone_prefix}走过完整一个月后，回头看这 {intakes_count} 次记录与 {} 分的今日状态，最珍贵的从来不是某天冲得多高，而是你在「{}」时懂得顺势停歇。无论四季怎么流转，窗台永远为你留着一杯热茶。",
                snapshot.qi_score, safe_badge
            ),
            archive_unlocked,
            if archive_unlocked {
                None
            } else {
                Some("知气雅集解锁完整 52 周慢递档案（到期后既往已解锁信件永久可读）".to_owned())
            },
        ),
    ];

    specs
        .into_iter()
        .map(
            |(week_number, title, term, ex_title, ex_seal, body, unlocked, lock_reason)| {
                CompanionWeeklyLetter {
                    week_number,
                    title,
                    companion_id: cid.to_owned(),
                    companion_name: cname.to_owned(),
                    solar_term: term,
                    ex_libris_title: ex_title,
                    ex_libris_seal: ex_seal,
                    body: if unlocked {
                        body
                    } else {
                        "此封伴侣手写慢递封存于岁时邮筒中，开启知气雅集即可拆阅并永久保留在只读保险箱内。".to_owned()
                    },
                    unlocked,
                    lock_reason,
                }
            },
        )
        .collect()
}

pub fn generate_pet_weekly_letter_draft(
    pet: &CustomPetCompanion,
    snapshot: &AnalyticsSnapshot,
    solar: &SolarTermState,
    week_number: u32,
    locale: &str,
    created_at: &str,
) -> PetWeeklyLetter {
    let en = is_en_locale(locale);
    let species = normalize_pet_species(&pet.species);
    let trimmed_persona = pet.persona.trim();
    let persona_note = if trimmed_persona.is_empty()
        || (en && crate::safety_ai::contains_cjk(trimmed_persona))
    {
        if en {
            "quiet windowsill company"
        } else {
            "安静守在窗边陪你歇息"
        }
    } else {
        trimmed_persona
    };
    let custom_seal = if !pet.paw_stamp_title.trim().is_empty() {
        pet.paw_stamp_title.trim().to_owned()
    } else if en {
        format!("{}'s Paw Seal", pet.name)
    } else {
        format!("{} · 梅花爪印", pet.name)
    };

    let sleep_phrase_en = snapshot
        .sleep_last_night_hours
        .map(|h| format!("{h:.1} hours of sleep"))
        .unwrap_or_else(|| "a calm night".to_owned());
    let sleep_phrase_zh = snapshot
        .sleep_last_night_hours
        .map(|h| format!("昨夜 {h:.1} 小时的安稳休息"))
        .unwrap_or_else(|| "昨夜安安静静的歇息".to_owned());

    let (title, body) = if en {
        let species_voice = match species {
            "cat" => "I curled my tail neatly on the warm windowsill cushion",
            "dog" => "I rested my chin near your slippers by the sunlit doorway",
            "rabbit" => "I twitched my ears beside your teacup as gentle light drifted in",
            "fox" => "I tucked my fluffy tail beneath the paper lantern by your desk",
            _ => "I settled softly in my favorite corner on your desk",
        };
        if pet.starlight_memorial {
            (
                format!("Week {week_number} · Starlight note from {}", pet.name),
                format!(
                    "From above the evening clouds beyond {}, it is me, {}. Remembering my way of {}, I still stay close whenever your desk lamp turns warm. I watched over {} and saw your vitality settle at {} today—please drink warm water, go easy on yourself, and leave room for slow rest. I am always right beside the moonbeams.",
                    solar.term_name, pet.name, persona_note, sleep_phrase_en, snapshot.qi_score
                ),
            )
        } else {
            (
                format!("Week {week_number} · Letter from {}", pet.name),
                format!(
                    "Hello from the window sill on {}! It is {}. True to my habit of {}, {species_voice}. Watching you carry {} into a vitality score of {} today made me stretch happily in the sun. Keep me close on busy afternoons and remember to pause when the evening lamp turns on.",
                    solar.term_name, pet.name, persona_note, sleep_phrase_en, snapshot.qi_score
                ),
            )
        }
    } else {
        let species_voice = match species {
            "cat" => "我把尾巴松松盘在窗台软垫上，呼噜声正轻",
            "dog" => "我把下巴搭在门廊日影旁的拖鞋边，轻轻摇着尾巴等你",
            "rabbit" => "我蹲在茶杯旁边竖起长耳朵，看风把纱帘吹起",
            "fox" => "我把蓬松大尾巴盖在书桌灯影里，替你暖着微凉的黄昏",
            _ => "我安静窝在书房窗口最顺手的那一角陪着你",
        };
        if pet.starlight_memorial {
            (
                format!("第 {week_number} 周 · {} 的星光窗边信", pet.name),
                format!(
                    "在{}的晚风和微光另一头，是我，{}呀。还记得我总爱「{}」的样子吗？每当窗台夜灯亮起来，我还是会乖乖落在光晕里守着你。看着你昨晚迎来了{}、今天元气停在 {} 分，别太赶路，天冷记得多喝一口温水、早些靠枕歇息，星光和我都一直在窗边。",
                    solar.term_name, pet.name, persona_note, sleep_phrase_zh, snapshot.qi_score
                ),
            )
        } else {
            (
                format!("第 {week_number} 周 · {} 寄来的窗边信", pet.name),
                format!(
                    "{}窗外天色正好，我是{}！照着我「{}」的脾气，{species_voice}。这几天陪你一起走过{}，看到你今天的元气停在 {} 分，我也跟着在阳光里舒展了一圈。忙起来记得回头摸摸我的耳朵，夜色沉下来就早点收起屏幕一起歇息呀。",
                    solar.term_name, pet.name, persona_note, sleep_phrase_zh, snapshot.qi_score
                ),
            )
        }
    };

    PetWeeklyLetter {
        id: 0,
        pet_id: pet.id.clone(),
        pet_name: pet.name.clone(),
        species: species.to_owned(),
        week_number,
        solar_term: solar.term_name.clone(),
        title,
        body,
        ex_libris_seal: custom_seal,
        starlight_memorial: pet.starlight_memorial,
        locale: crate::entitlements::normalize_locale(locale).to_owned(),
        created_at: created_at.to_owned(),
    }
}

pub fn mood_byte_from_state(state: &str) -> (u8, &'static str) {
    mood_byte_from_state_localized(state, DEFAULT_LOCALE)
}

pub fn mood_byte_from_state_localized(state: &str, locale: &str) -> (u8, &'static str) {
    if is_en_locale(locale) {
        match state {
            "prime_ready" => (0, "SUNNY_PRIME · Bright & rested"),
            "recovery_priority" => (2, "COZY_RECOVER · Hearth rest"),
            _ => (1, "BREEZY_STEADY · Calm & steady"),
        }
    } else {
        match state {
            "prime_ready" => (0, "SUNNY_PRIME · 晴朗充沛"),
            "recovery_priority" => (2, "COZY_RECOVER · 暖炉静养"),
            _ => (1, "BREEZY_STEADY · 微风守中"),
        }
    }
}

pub fn mood_label_from_byte(b: i64) -> &'static str {
    mood_label_from_byte_localized(b, DEFAULT_LOCALE)
}

pub fn mood_label_from_byte_localized(b: i64, locale: &str) -> &'static str {
    if is_en_locale(locale) {
        match b {
            0 => "SUNNY_PRIME · Bright & rested",
            2 => "COZY_RECOVER · Hearth rest",
            _ => "BREEZY_STEADY · Calm & steady",
        }
    } else {
        match b {
            0 => "SUNNY_PRIME · 晴朗充沛",
            2 => "COZY_RECOVER · 暖炉静养",
            _ => "BREEZY_STEADY · 微风守中",
        }
    }
}

pub fn bonsai_stage_for_dew(dew: i64) -> (&'static str, Option<i64>) {
    bonsai_stage_for_dew_localized(dew, DEFAULT_LOCALE)
}

pub fn bonsai_stage_for_dew_localized(dew: i64, locale: &str) -> (&'static str, Option<i64>) {
    if is_en_locale(locale) {
        if dew < 5 {
            ("Sprouting", Some(5))
        } else if dew < 15 {
            ("Leafing out", Some(15))
        } else if dew < 30 {
            ("Budding", Some(30))
        } else {
            ("Golden bloom", None)
        }
    } else if dew < 5 {
        ("萌芽", Some(5))
    } else if dew < 15 {
        ("抽枝", Some(15))
    } else if dew < 30 {
        ("含苞", Some(30))
    } else {
        ("满树金桂", None)
    }
}

/// ⑤ 隐私优先双人同窗本地状态（仅同步 1 字节心情枚举）。
pub fn build_co_sanctuary_state(settings: &AppSettings, snapshot: &AnalyticsSnapshot) -> CoSanctuaryState {
    build_co_sanctuary_state_localized(settings, snapshot, DEFAULT_LOCALE)
}

pub fn build_co_sanctuary_state_localized(
    settings: &AppSettings,
    snapshot: &AnalyticsSnapshot,
    locale: &str,
) -> CoSanctuaryState {
    let en = is_en_locale(locale);
    let (self_mood_byte, self_mood_label) = mood_byte_from_state_localized(&snapshot.state, locale);
    let partner_byte = settings.partner_mood.clamp(0, 2) as u8;
    let partner_label = mood_label_from_byte_localized(settings.partner_mood, locale);
    let (bonsai_stage, next_stage_dew) = bonsai_stage_for_dew_localized(settings.bonsai_dew, locale);

    let partner_name = if settings.partner_name.trim().is_empty() {
        if en { "Window Partner".to_owned() } else { "窗边同窗".to_owned() }
    } else {
        settings.partner_name.clone()
    };

    CoSanctuaryState {
        pair_code: if settings.pair_code.trim().is_empty() {
            "QI-2026".to_owned()
        } else {
            settings.pair_code.clone()
        },
        partner_name,
        self_mood_byte,
        self_mood_label: self_mood_label.to_owned(),
        partner_mood_byte: partner_byte,
        partner_mood_label: partner_label.to_owned(),
        bonsai_name: if en {
            "Window Osmanthus Bonsai".to_owned()
        } else {
            "窗台岁时金桂盆栽".to_owned()
        },
        bonsai_dew: settings.bonsai_dew,
        bonsai_stage: bonsai_stage.to_owned(),
        next_stage_dew,
        available_actions: if en {
            vec![
                CoSanctuaryActionSpec {
                    id: "warm_tea".into(),
                    label: "Send warm tea +1 dew".into(),
                    dew_reward: 1,
                },
                CoSanctuaryActionSpec {
                    id: "tuck_blanket".into(),
                    label: "Tuck a blanket +1 dew".into(),
                    dew_reward: 1,
                },
                CoSanctuaryActionSpec {
                    id: "paw_stamp".into(),
                    label: "Leave a paw tap +1 dew".into(),
                    dew_reward: 1,
                },
            ]
        } else {
            vec![
                CoSanctuaryActionSpec {
                    id: "warm_tea".into(),
                    label: "递热茶 +1 滴晨露".into(),
                    dew_reward: 1,
                },
                CoSanctuaryActionSpec {
                    id: "tuck_blanket".into(),
                    label: "盖薄被 +1 滴晨露".into(),
                    dew_reward: 1,
                },
                CoSanctuaryActionSpec {
                    id: "paw_stamp".into(),
                    label: "按爪印 +1 滴晨露".into(),
                    dew_reward: 1,
                },
            ]
        },
        privacy_note: if en {
            "Privacy contract: exchanges only a 1-byte mood status (0=Bright / 1=Steady / 2=Rest)—never heart rate, steps, or lab values.".to_owned()
        } else {
            "隐私契约：仅交换 1 字节心情枚举 (0=晴朗 / 1=微风 / 2=静养)，零传输心率、步数或化验数值。".to_owned()
        },
    }
}

/// ⑥ 抬腕嘴替桌面微件载荷（支持 `"metaphor"` 诗意隐喻 vs `"metrics"` 数值开关）。
pub fn build_widget_preview(
    privacy_mode: &str,
    snapshot: &AnalyticsSnapshot,
    solar: &SolarTermState,
) -> WidgetStudioPayload {
    build_widget_preview_localized(privacy_mode, snapshot, solar, DEFAULT_LOCALE)
}

pub fn build_widget_preview_localized(
    privacy_mode: &str,
    snapshot: &AnalyticsSnapshot,
    solar: &SolarTermState,
    locale: &str,
) -> WidgetStudioPayload {
    let en = is_en_locale(locale);
    let mode = if privacy_mode.trim() == "metrics" {
        "metrics"
    } else {
        "metaphor"
    };

    let metaphor_headline = if en {
        match snapshot.state.as_str() {
            "prime_ready" => "Light spring breeze on the rail—glide with ease",
            "recovery_priority" => "Simmering tea beneath the eaves—leave space for rest",
            _ => "Drifting clouds and steady steps—calmly balanced",
        }
    } else {
        match snapshot.state.as_str() {
            "prime_ready" => "春风拂栏，宜乘兴而行",
            "recovery_priority" => "檐下煮茶，今日适宜留白",
            _ => "云淡风轻，步履守中自稳",
        }
    };

    let safe_badge = if en && contains_cjk(&snapshot.state_badge) {
        match snapshot.state.as_str() {
            "prime_ready" => "Well rested".to_owned(),
            "recovery_priority" => "Rest gently".to_owned(),
            _ => "Balanced".to_owned(),
        }
    } else {
        snapshot.state_badge.clone()
    };
    let safe_action = if en && contains_cjk(&snapshot.micro_action) {
        match snapshot.state.as_str() {
            "prime_ready" => "Step outside for 20 minutes of daylight.".to_owned(),
            "recovery_priority" => "Ease intensity today and rest early.".to_owned(),
            _ => "Stay hydrated and keep evening rhythms calm.".to_owned(),
        }
    } else {
        snapshot.micro_action.clone()
    };

    let hrv_str = snapshot
        .metrics
        .get("hrv_rmssd_ms")
        .or_else(|| snapshot.metrics.get("hrv_sdnn_ms"))
        .and_then(|m| m.value)
        .map(|v| format!("HRV {v:.0}ms"))
        .unwrap_or_else(|| {
            if en { "No continuous HRV".to_owned() } else { "无连续HRV".to_owned() }
        });
    let rhr_str = snapshot
        .metrics
        .get("resting_heart_rate_bpm")
        .and_then(|m| m.value)
        .map(|v| format!("RHR {v:.0}bpm"))
        .unwrap_or_else(|| "RHR --".to_owned());
    let sleep_str = snapshot
        .sleep_last_night_hours
        .map(|h| {
            if en {
                format!("Sleep {h:.1}h")
            } else {
                format!("睡 {h:.1}h")
            }
        })
        .unwrap_or_else(|| {
            if en { "Steps-only mode".to_owned() } else { "纯步数模式".to_owned() }
        });
    let metrics_line = format!("{hrv_str} · {rhr_str} · {sleep_str}");

    if mode == "metrics" {
        WidgetStudioPayload {
            privacy_mode: "metrics".to_owned(),
            solar_term: solar.term_name.clone(),
            lunar_phase: solar.lunar_phase.clone(),
            headline: if en {
                format!("Vitality {} · {}", snapshot.qi_score, safe_badge)
            } else {
                format!("元气 {} · {}", snapshot.qi_score, safe_badge)
            },
            subline: safe_action,
            badge_text: safe_badge,
            companion_line: format!("{} · {}", solar.term_name, solar.lunar_phase),
            qi_score_visible: true,
            qi_score: Some(snapshot.qi_score),
            metrics_line: Some(metrics_line),
        }
    } else {
        WidgetStudioPayload {
            privacy_mode: "metaphor".to_owned(),
            solar_term: solar.term_name.clone(),
            lunar_phase: solar.lunar_phase.clone(),
            headline: metaphor_headline.to_owned(),
            subline: solar.whisper.clone(),
            badge_text: format!("{} · {}", solar.term_name, solar.lunar_phase),
            companion_line: if en {
                "Wrist metaphor mode · Raw physiological numbers hidden".to_owned()
            } else {
                "抬腕嘴替 · 已隐藏具体生理数值".to_owned()
            },
            qi_score_visible: false,
            qi_score: None,
            metrics_line: None,
        }
    }
}

/// ⑦ 四时身体风物长卷 + 宋瓷勋章（「证伪小达人」「岁时知己」「食光收藏家」「窗台添柴人」）。
pub fn build_four_seasons_scroll_and_badges(
    settings: &AppSettings,
    snapshot: &AnalyticsSnapshot,
    solar: &SolarTermState,
    experiments: &[ExperimentEvaluation],
    stickers_count: usize,
    intakes_count: usize,
    distinct_metric_days: usize,
) -> (Vec<FourSeasonsScrollNode>, Vec<CeramicBadge>) {
    build_four_seasons_scroll_and_badges_localized(
        settings,
        snapshot,
        solar,
        experiments,
        stickers_count,
        intakes_count,
        distinct_metric_days,
        DEFAULT_LOCALE,
    )
}

pub fn build_four_seasons_scroll_and_badges_localized(
    settings: &AppSettings,
    snapshot: &AnalyticsSnapshot,
    solar: &SolarTermState,
    experiments: &[ExperimentEvaluation],
    stickers_count: usize,
    intakes_count: usize,
    distinct_metric_days: usize,
    locale: &str,
) -> (Vec<FourSeasonsScrollNode>, Vec<CeramicBadge>) {
    let en = is_en_locale(locale);
    let safe_badge = if en && contains_cjk(&snapshot.state_badge) {
        match snapshot.state.as_str() {
            "prime_ready" => "Well rested".to_owned(),
            "recovery_priority" => "Rest gently".to_owned(),
            _ => "Balanced".to_owned(),
        }
    } else {
        snapshot.state_badge.clone()
    };
    let safe_hw_tier = if en && contains_cjk(&snapshot.hardware_tier_label) {
        match snapshot.hardware_tier.as_str() {
            "tier_a" => "Tier A · Full HRV + Sleep + Activity".to_owned(),
            "tier_b" => "Tier B · Sleep + Resting HR + Activity".to_owned(),
            _ => "Tier C · Activity + Subjective Rhythm".to_owned(),
        }
    } else {
        snapshot.hardware_tier_label.clone()
    };

    let scroll = if en {
        vec![
            FourSeasonsScrollNode {
                season: "spring".into(),
                season_cn: "Spring · Rising vigor".into(),
                solar_term: "Grain Rain".into(),
                title: "Morning light & sleep alignment".into(),
                summary: "Capture your first daylight walks and step tempo to steady sleep regularity.".into(),
                highlight_value: format!("Baseline · {safe_hw_tier}"),
                sealed: solar.season == "spring" || distinct_metric_days >= 7,
            },
            FourSeasonsScrollNode {
                season: "summer".into(),
                season_cn: "Summer · Calm heart".into(),
                solar_term: "Grain Buds".into(),
                title: "Midsummer rhythm & caffeine curfew".into(),
                summary: "Guard an afternoon 15:00 caffeine cutoff through warm cicada days.".into(),
                highlight_value: format!("{} experiments logged", experiments.len()),
                sealed: solar.season == "summer" || distinct_metric_days >= 14,
            },
            FourSeasonsScrollNode {
                season: "autumn".into(),
                season_cn: "Autumn · Nourishing harvest".into(),
                solar_term: "Frost's Descent".into(),
                title: "Meal polaroids & metabolic poise".into(),
                summary: "Preserve the warmth of everyday meals and tea in your food journal.".into(),
                highlight_value: format!("{stickers_count} stickers · {intakes_count} meal notes"),
                sealed: solar.season == "autumn" || stickers_count > 0 || distinct_metric_days >= 21,
            },
            FourSeasonsScrollNode {
                season: "winter".into(),
                season_cn: "Winter · Quiet storage".into(),
                solar_term: solar.term_name.clone(),
                title: "Deep winter rest & gentle restoration".into(),
                summary: "Switch to double-dew Recovery Mode on low days to give your nervous system space.".into(),
                highlight_value: format!("Vitality {} · {safe_badge}", snapshot.qi_score),
                sealed: solar.season == "winter" || distinct_metric_days >= 30,
            },
        ]
    } else {
        vec![
            FourSeasonsScrollNode {
                season: "spring".into(),
                season_cn: "春 · 疏泄生发".into(),
                solar_term: "谷雨".into(),
                title: "初春睡意校准".into(),
                summary: "记下第一缕晨光与步数节奏，唤醒睡眠稳态。".into(),
                highlight_value: format!("目标口径 · {}", snapshot.hardware_tier_label),
                sealed: solar.season == "spring" || distinct_metric_days >= 7,
            },
            FourSeasonsScrollNode {
                season: "summer".into(),
                season_cn: "夏 · 养心定神".into(),
                solar_term: "小满".into(),
                title: "盛夏节律与咖啡因界限".into(),
                summary: "在蝉鸣季守住午后 15:00 无咖啡因窗口。".into(),
                highlight_value: format!("累计试验 {} 项", experiments.len()),
                sealed: solar.season == "summer" || distinct_metric_days >= 14,
            },
            FourSeasonsScrollNode {
                season: "autumn".into(),
                season_cn: "秋 · 敛阴润燥".into(),
                solar_term: "霜降".into(),
                title: "金秋食光与代谢归仓".into(),
                summary: "用拍立得手账定格一餐一茶的温度。".into(),
                highlight_value: format!("食光贴纸 {stickers_count} 张 · 餐记 {intakes_count} 次"),
                sealed: solar.season == "autumn" || stickers_count > 0 || distinct_metric_days >= 21,
            },
            FourSeasonsScrollNode {
                season: "winter".into(),
                season_cn: "冬 · 藏精蓄元".into(),
                solar_term: solar.term_name.clone(),
                title: "岁末冬藏与安稳入眠".into(),
                summary: "疲惫日安心切换静养双倍露，留足修复窗口。".into(),
                highlight_value: format!("当前元气 {} · {}", snapshot.qi_score, snapshot.state_badge),
                sealed: solar.season == "winter" || distinct_metric_days >= 30,
            },
        ]
    };

    let has_disproved_experiment = experiments
        .iter()
        .any(|e| e.outcome_verdict == "neutral" || e.outcome_verdict == "disproved");

    let badges = if en {
        vec![
            CeramicBadge {
                id: "falsification_master".into(),
                title: "Myth Buster".into(),
                glaze_style: "Ru Celadon Glaze".into(),
                earned: has_disproved_experiment,
                reason: if has_disproved_experiment {
                    "Completed an N-of-1 experiment and disproved an ineffective habit—saving wasted effort with data.".into()
                } else {
                    "Complete any N-of-1 experiment whose outcome shows neutral difference to light this badge.".into()
                },
            },
            CeramicBadge {
                id: "solar_confidant".into(),
                title: "Solar Confidant".into(),
                glaze_style: "Ding Ivory Glaze".into(),
                earned: settings.companion_loyalty_days >= 30 || distinct_metric_days >= 14,
                reason: "Recorded steadily through half a solar cycle to learn your personal baseline.".into(),
            },
            CeramicBadge {
                id: "food_collector".into(),
                title: "Table Collector".into(),
                glaze_style: "Ge Gold-Crackled Glaze".into(),
                earned: stickers_count >= 1 || intakes_count >= 5,
                reason: "Captured everyday meals with food polaroids or natural language meal notes.".into(),
            },
            CeramicBadge {
                id: "hearth_guardian".into(),
                title: "Hearth Keeper".into(),
                glaze_style: "Jian Tenmoku Hare's Fur".into(),
                earned: settings.guardian_sponsor,
                reason: if settings.guardian_sponsor {
                    "Lit your Hearth Keeper ceramic badge with a transferable Hearth Gift Pass community seat.".into()
                } else {
                    "Purchase a Hearth Gift Pass · Gift 1 Community Seat to light this Song ceramic badge.".into()
                },
            },
        ]
    } else {
        vec![
            CeramicBadge {
                id: "falsification_master".into(),
                title: "证伪小达人".into(),
                glaze_style: "汝窑天青釉".into(),
                earned: has_disproved_experiment,
                reason: if has_disproved_experiment {
                    "已完成一项 N-of-1 实验并客观证伪无效假设，用数据省下无效努力！".into()
                } else {
                    "完成任意 N-of-1 实验且结果判定为无显著作用（neutral）即可点亮。".into()
                },
            },
            CeramicBadge {
                id: "solar_confidant".into(),
                title: "岁时知己".into(),
                glaze_style: "定窑象牙白".into(),
                earned: settings.companion_loyalty_days >= 30 || distinct_metric_days >= 14,
                reason: "持续记录陪伴走过半个节气周期，熟悉自己的生理基线波动。".into(),
            },
            CeramicBadge {
                id: "food_collector".into(),
                title: "食光收藏家".into(),
                glaze_style: "哥窑金丝铁线".into(),
                earned: stickers_count >= 1 || intakes_count >= 5,
                reason: "用饮食手账贴纸或自然语言记录留存日常三餐烟火气。".into(),
            },
            CeramicBadge {
                id: "hearth_guardian".into(),
                title: "窗台添柴人".into(),
                glaze_style: "建盏曜变黑釉".into(),
                earned: settings.guardian_sponsor,
                reason: if settings.guardian_sponsor {
                    "已点亮「窗台添柴人」宋瓷徽章，持有一份可转赠的社区漂流席位礼赠券。".into()
                } else {
                    "选购「窗台添柴礼赠券 · 赠予社区漂流席位」即可点亮此宋瓷徽章。".into()
                },
            },
        ]
    };

    (scroll, badges)
}

#[allow(clippy::too_many_arguments)]
pub fn build_solar_atelier_payload(
    settings: &AppSettings,
    snapshot: &AnalyticsSnapshot,
    ent: &EntitlementVerificationResult,
    dew_balance: i64,
    dew_ledger: Vec<DewLedgerEntry>,
    claimed_rewards: &[String],
    presets: Vec<SanctuaryPreset>,
    stickers: Vec<FoodSticker>,
    used_stickers_in_month: usize,
    experiments: &[ExperimentEvaluation],
    intakes_count: usize,
    distinct_metric_days: usize,
) -> SolarAtelierPayload {
    build_solar_atelier_payload_v5(
        settings,
        snapshot,
        ent,
        dew_balance,
        dew_ledger,
        claimed_rewards,
        presets,
        stickers,
        used_stickers_in_month,
        experiments,
        intakes_count,
        distinct_metric_days,
        Vec::new(),
        Vec::new(),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn build_solar_atelier_payload_v5(
    settings: &AppSettings,
    snapshot: &AnalyticsSnapshot,
    ent: &EntitlementVerificationResult,
    dew_balance: i64,
    dew_ledger: Vec<DewLedgerEntry>,
    claimed_rewards: &[String],
    presets: Vec<SanctuaryPreset>,
    stickers: Vec<FoodSticker>,
    used_stickers_in_month: usize,
    experiments: &[ExperimentEvaluation],
    intakes_count: usize,
    distinct_metric_days: usize,
    custom_pets: Vec<CustomPetCompanion>,
    pet_weekly_letters: Vec<PetWeeklyLetter>,
) -> SolarAtelierPayload {
    let locale = settings.locale.as_str();
    let solar_term = compute_solar_term_state_localized(&snapshot.target_date, &settings.weather_mood, locale);
    let seasonal_pass = build_seasonal_pass_localized(ent, claimed_rewards, locale);
    let keepsake_catalog = build_keepsake_catalog_localized(settings, ent, claimed_rewards, locale);
    let ym: String = snapshot.target_date.chars().take(7).collect();
    let sticker_quota = compute_sticker_quota(&ym, used_stickers_in_month, ent);
    let weekly_letters = build_companion_weekly_letters_localized(
        "xiaoman",
        snapshot,
        &solar_term,
        intakes_count,
        stickers.len(),
        ent,
        locale,
    );
    let co_sanctuary = build_co_sanctuary_state_localized(settings, snapshot, locale);
    let widget_preview =
        build_widget_preview_localized(&settings.widget_privacy_mode, snapshot, &solar_term, locale);
    let (four_seasons_scroll, ceramic_badges) = build_four_seasons_scroll_and_badges_localized(
        settings,
        snapshot,
        &solar_term,
        experiments,
        stickers.len(),
        intakes_count,
        distinct_metric_days,
        locale,
    );
    let preset_limit = if has_feature(ent, "preset_slots_unlimited") {
        None
    } else {
        Some(FREE_PRESET_MAX)
    };
    let active_custom_pet = custom_pets.iter().find(|p| p.is_active).cloned();
    let custom_pet_quota = compute_custom_pet_quota(custom_pets.len(), ent);

    SolarAtelierPayload {
        solar_term,
        dew_balance,
        dew_ledger,
        seasonal_pass,
        keepsake_catalog,
        presets,
        preset_limit,
        stickers,
        sticker_quota,
        weekly_letters,
        co_sanctuary,
        widget_preview,
        four_seasons_scroll,
        ceramic_badges,
        custom_pets,
        active_custom_pet,
        custom_pet_quota,
        pet_weekly_letters,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entitlements::{SubscriptionTier, mint_oset_for_tier, verify_oset_with_clock};

    #[test]
    fn covers_all_24_solar_terms_and_weather_world_mapping() {
        let mut seen = std::collections::BTreeSet::new();
        let test_dates = [
            "2026-01-06", "2026-01-21", "2026-02-05", "2026-02-20", "2026-03-07", "2026-03-22",
            "2026-04-06", "2026-04-21", "2026-05-07", "2026-05-22", "2026-06-07", "2026-06-22",
            "2026-07-08", "2026-07-24", "2026-08-08", "2026-08-24", "2026-09-09", "2026-09-24",
            "2026-10-09", "2026-10-24", "2026-11-13", "2026-11-23", "2026-12-08", "2026-12-23",
        ];
        for d in test_dates {
            let st = compute_solar_term_state(d, "sunny_crisp");
            seen.insert(st.term_name);
        }
        assert_eq!(seen.len(), 24);
        for t in SOLAR_TERMS_24 {
            assert!(seen.contains(t), "missing solar term {t}");
        }

        let rain = compute_solar_term_state("2026-11-13", "misty_rain");
        assert_eq!(rain.active_world, "solar_spring_rain");
        assert_eq!(rain.term_name, "立冬");
        assert_eq!(rain.season, "winter");

        let snow = compute_solar_term_state("2026-11-13", "first_snow");
        assert_eq!(snow.active_world, "solar_winter_hearth");

        let clear = compute_solar_term_state("2026-05-22", "clear_moon");
        assert_eq!(clear.term_name, "小满");
        assert_eq!(clear.season, "summer");
        assert_eq!(clear.active_world, "solar_summer_lotus");
    }

    #[test]
    fn nutrition_hint_and_bonsai_stages_work() {
        assert!(infer_sticker_nutrition_hint("海鲜自助配冰啤酒").contains("嘌呤"));
        assert!(infer_sticker_nutrition_hint("日式豚骨拉面").contains("钠"));
        assert!(infer_sticker_nutrition_hint("清蒸鲈鱼配小米粥").contains("清欢"));
        assert!(infer_sticker_nutrition_hint_localized("Seafood hotpot & beer", "en").contains("Purine"));

        assert_eq!(bonsai_stage_for_dew(0).0, "萌芽");
        assert_eq!(bonsai_stage_for_dew(5).0, "抽枝");
        assert_eq!(bonsai_stage_for_dew(15).0, "含苞");
        assert_eq!(bonsai_stage_for_dew(30).0, "满树金桂");
        assert_eq!(bonsai_stage_for_dew_localized(30, "en").0, "Golden bloom");
    }

    #[test]
    fn expired_read_only_vault_keeps_previously_unlocked_letters_readable() {
        let now = 1_760_000_000;
        let (_claims, token) =
            mint_oset_for_tier("u1", SubscriptionTier::ProMonthly, now, false, false).unwrap();
        let expired = verify_oset_with_clock(&token, now + 45 * 86_400, now, None);
        assert_eq!(expired.state, OfflineEntitlementState::ExpiredReadOnlyVault);

        let snap = crate::analytics::compute_snapshot(
            &[],
            &[],
            &[],
            crate::analytics::HardwareTier::TierA,
            4,
        );
        let solar = compute_solar_term_state("2026-11-13", "sunny_crisp");
        let letters = build_companion_weekly_letters("xiaoman", &snap, &solar, 2, 1, &expired);
        assert_eq!(letters.len(), 4);
        assert!(letters.iter().all(|l| l.unlocked));
    }

    #[test]
    fn custom_pet_validation_and_bilingual_starlight_letter_work() {
        assert!(validate_pet_photo_data_url("data:image/png;base64,iVBORw0KGgo=", "en").is_ok());
        assert!(validate_pet_photo_data_url("data:image/svg+xml;base64,PHN2Zz4=", "en").is_err());
        assert!(validate_pet_photo_data_url("https://example.com/cat.png", "zh-CN").is_err());

        let pet = CustomPetCompanion {
            id: "pet_1".into(),
            name: "Mochi".into(),
            species: "cat".into(),
            persona: "sunbeam naps by the kettle".into(),
            photo_data_url: "local://mochi.png".into(),
            die_cut_frame: "starlight_halo".into(),
            starlight_memorial: true,
            paw_stamp_title: "Mochi Ex Libris".into(),
            is_active: true,
            created_at: "2026-11-13T08:00:00Z".into(),
            updated_at: "2026-11-13T08:00:00Z".into(),
        };
        let snap = crate::analytics::compute_snapshot(
            &[],
            &[],
            &[],
            crate::analytics::HardwareTier::TierA,
            4,
        );
        let solar_en = compute_solar_term_state_localized("2026-11-13", "sunny_crisp", "en");
        let letter = generate_pet_weekly_letter_draft(
            &pet,
            &snap,
            &solar_en,
            1,
            "en",
            "2026-11-13T20:00:00Z",
        );
        assert!(letter.starlight_memorial);
        assert_eq!(letter.ex_libris_seal, "Mochi Ex Libris");
        assert!(letter.body.contains("Mochi"));
        assert!(!contains_cjk(&letter.body));
    }
}
