//! 「知气雅集 · Qi Atelier」情感化节律引擎：
//! 1. 24 节气 × 月相 × 窗外天色心境
//! 2. 岁时风物双轨季票（清欢轨 + 雅集轨）与晨露账本（含静养日免费 +3 💧）
//! 3. 食光拍立得贴纸与月历餐垫（免费每月 5 张，雅集无限 + 嘌呤/钠轻提示）
//! 4. 伴侣周日夜话慢递 & 节气藏书票
//! 5. 隐私优先双人同窗（1 字节心情枚举 + 窗台岁时金桂盆栽）
//! 6. 抬腕嘴替桌面微件（诗意隐喻 vs 数值视图隐私开关）
//! 7. 我的四时身体风物长卷 & 宋瓷勋章（含「证伪小达人」）

use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::analytics::AnalyticsSnapshot;
use crate::entitlements::{EntitlementVerificationResult, OfflineEntitlementState, has_feature};
use crate::experiments::ExperimentEvaluation;
use crate::storage::AppSettings;

pub const FREE_STICKER_MONTHLY_QUOTA: usize = 5;
pub const FREE_PRESET_MAX: usize = 2;
pub const MAX_EQUIPPED_KEEPSAKES: usize = 3;
pub const RECOVERY_PRIORITY_DEW_BONUS: i64 = 3;

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

fn lunar_phase_for_date(date: NaiveDate) -> (String, u32) {
    // 以 2000-01-06 朔日为基准，按平均朔望月 29.53058867 天估算月相
    let ref_new_moon = NaiveDate::from_ymd_opt(2000, 1, 6).expect("valid ref date");
    let diff_days = (date - ref_new_moon).num_days() as f64;
    let synodic = 29.53058867;
    let age = ((diff_days % synodic) + synodic) % synodic;
    let lunar_day = (age.floor() as u32).clamp(0, 29) + 1;
    let phase = if age < 1.85 || age >= 27.68 {
        "朔月"
    } else if age < 5.54 {
        "蛾眉月"
    } else if age < 9.23 {
        "上弦月"
    } else if age < 12.92 {
        "盈凸月"
    } else if age < 16.61 {
        "望月"
    } else if age < 20.30 {
        "亏凸月"
    } else if age < 23.99 {
        "下弦月"
    } else {
        "残月"
    };
    (phase.to_owned(), lunar_day)
}

/// ① 计算 24 节气、月相、天色心境与对应动态书房世界。
pub fn compute_solar_term_state(date_str: &str, weather_mood: &str) -> SolarTermState {
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

    let (_, _, term_name, season, season_cn, base_whisper) = TERM_ANCHORS[matched_idx];
    let (lunar_phase, lunar_day_approx) = lunar_phase_for_date(date);
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

    let mood_hint = match mood {
        "misty_rain" => "窗外烟雨微凉，适合泡一盏热茶静听雨声。",
        "first_snow" => "初雪落檐，炉火正暖，先把紧绷的肩膀放下来。",
        "clear_moon" => "月色澄澈如水，今夜宜少思虑、早歇息。",
        _ => "天光晴朗通透，顺着呼吸慢慢来就很好。",
    };

    SolarTermState {
        date: date.format("%Y-%m-%d").to_string(),
        term_name: term_name.to_owned(),
        term_index: matched_idx,
        season: season.to_owned(),
        season_cn: season_cn.to_owned(),
        lunar_phase: lunar_phase.clone(),
        lunar_day_approx,
        weather_mood: mood.to_owned(),
        active_world: active_world.to_owned(),
        whisper: format!("{term_name} · {lunar_phase}：{base_whisper}{mood_hint}"),
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

pub fn build_seasonal_pass(
    ent: &EntitlementVerificationResult,
    claimed_rewards: &[String],
) -> Vec<SeasonalPassNode> {
    let atelier_ok = has_feature(ent, "atelier_seasonal_track");
    seasonal_pass_definitions()
        .into_iter()
        .map(
            |(id, title, track, reward_kind, reward_id, cost_dew, term)| SeasonalPassNode {
                id: id.to_owned(),
                title: title.to_owned(),
                track: track.to_owned(),
                reward_kind: reward_kind.to_owned(),
                reward_id: reward_id.to_owned(),
                cost_dew,
                claimed: claimed_rewards.iter().any(|r| r == id || r == reward_id),
                unlocked_by_tier: track == "free" || atelier_ok,
                evergreen_term: term.to_owned(),
            },
        )
        .collect()
}

pub fn build_keepsake_catalog(
    settings: &AppSettings,
    ent: &EntitlementVerificationResult,
    claimed_rewards: &[String],
) -> KeepsakeCatalog {
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

    let raw_items = [
        ("tea_cup", "天青茶盏", "点亮第一缕暖茶香", "free"),
        ("osmanthus", "金桂小瓶", "窗台秋意暗香来", "atelier"),
        ("cushion", "编织软垫", "小满午睡更松软", "free"),
        ("night_lantern", "暖纸夜灯", "雨夜亮一盏安眠灯", "atelier"),
        ("celadon_cup", "汝窑香茗", "一盏清茶定心神", "free"),
        ("pine_twig", "迎客苍松", "四时青翠守呼吸", "atelier"),
        ("cinnabar_seal", "寿山闲章", "每日钤印留清记", "free"),
        ("stone_lantern", "庭院石灯", "听雨观心夜微明", "atelier"),
    ];
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

    let outfits = vec![
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
    ];

    let seal_unlock = has_feature(ent, "poster_custom_seals");
    let seal_inks = vec![
        SealInkSpec {
            id: "cinnabar".into(),
            name: "传统朱砂红".into(),
            hex_hint: "#C94A38".into(),
            unlocked: true,
        },
        SealInkSpec {
            id: "gold".into(),
            name: "宋金泥金印".into(),
            hex_hint: "#C59B38".into(),
            unlocked: seal_unlock || claimed_rewards.iter().any(|r| r == "gold"),
        },
        SealInkSpec {
            id: "lapis".into(),
            name: "青金石秘色".into(),
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
    let lower = title.to_ascii_lowercase();
    if ["海鲜", "牡蛎", "生蚝", "花甲", "基围虾", "螃蟹", "浓汤", "火锅", "啤酒", "内脏", "烧烤", "猪肝", "肉汤"]
        .iter()
        .any(|k| title.contains(k))
    {
        "嘌呤提示：偏中高嘌呤风味，建议搭配足量温水与深色叶菜帮助代谢。".to_owned()
    } else if ["咸", "腊", "卤", "咸菜", "泡菜", "拉面", "酱", "麻辣烫", "外卖", "炸鸡", "重口"]
        .iter()
        .any(|k| title.contains(k))
    {
        "钠含量提示：调味相对偏浓郁，餐后注意补充温水与富含钾的新鲜蔬果。".to_owned()
    } else if ["奶茶", "拿铁", "咖啡", "甜品", "蛋糕", "冰淇淋", "可乐"]
        .iter()
        .any(|k| title.contains(k))
        || lower.contains("latte")
        || lower.contains("coffee")
    {
        "糖分与咖啡因提示：浅尝即止更自在，15:00 后减少咖啡因更护深睡。".to_owned()
    } else if ["清蒸", "白灼", "沙拉", "青菜", "燕麦", "清茶", "豆浆", "粥", "水煮蛋", "三文鱼", "藜麦"]
        .iter()
        .any(|k| title.contains(k))
    {
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
    match companion_id {
        "shanfang" | "zen_garden" => ("shanfang", "山房松石守望者"),
        "meiqiou" | "playful_bento" => ("meiqiou", "软胶猫「煤球」"),
        _ => ("xiaoman", "三花猫「小满」"),
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
    let (cid, cname) = companion_profile(companion_id);
    let archive_unlocked = has_feature(ent, "weekly_letters_archive")
        || (ent.state == OfflineEntitlementState::ExpiredReadOnlyVault && ent.valid_signature);

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
                snapshot.qi_score, snapshot.state_badge
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
                snapshot.qi_score, snapshot.state_badge
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

pub fn mood_byte_from_state(state: &str) -> (u8, &'static str) {
    match state {
        "prime_ready" => (0, "SUNNY_PRIME · 晴朗充沛"),
        "recovery_priority" => (2, "COZY_RECOVER · 暖炉静养"),
        _ => (1, "BREEZY_STEADY · 微风守中"),
    }
}

pub fn mood_label_from_byte(b: i64) -> &'static str {
    match b {
        0 => "SUNNY_PRIME · 晴朗充沛",
        2 => "COZY_RECOVER · 暖炉静养",
        _ => "BREEZY_STEADY · 微风守中",
    }
}

pub fn bonsai_stage_for_dew(dew: i64) -> (&'static str, Option<i64>) {
    if dew < 5 {
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
    let (self_mood_byte, self_mood_label) = mood_byte_from_state(&snapshot.state);
    let partner_byte = settings.partner_mood.clamp(0, 2) as u8;
    let partner_label = mood_label_from_byte(settings.partner_mood);
    let (bonsai_stage, next_stage_dew) = bonsai_stage_for_dew(settings.bonsai_dew);

    CoSanctuaryState {
        pair_code: if settings.pair_code.trim().is_empty() {
            "QI-2026".to_owned()
        } else {
            settings.pair_code.clone()
        },
        partner_name: if settings.partner_name.trim().is_empty() {
            "窗边同窗".to_owned()
        } else {
            settings.partner_name.clone()
        },
        self_mood_byte,
        self_mood_label: self_mood_label.to_owned(),
        partner_mood_byte: partner_byte,
        partner_mood_label: partner_label.to_owned(),
        bonsai_name: "窗台岁时金桂盆栽".to_owned(),
        bonsai_dew: settings.bonsai_dew,
        bonsai_stage: bonsai_stage.to_owned(),
        next_stage_dew,
        available_actions: vec![
            CoSanctuaryActionSpec {
                id: "warm_tea".into(),
                label: "递热茶 +1 💧".into(),
                dew_reward: 1,
            },
            CoSanctuaryActionSpec {
                id: "tuck_blanket".into(),
                label: "盖薄被 +1 💧".into(),
                dew_reward: 1,
            },
            CoSanctuaryActionSpec {
                id: "paw_stamp".into(),
                label: "按爪印 +1 💧".into(),
                dew_reward: 1,
            },
        ],
        privacy_note: "隐私契约：仅交换 1 字节心情枚举 (0=晴朗 / 1=微风 / 2=静养)，零传输心率、步数或化验数值。".to_owned(),
    }
}

/// ⑥ 抬腕嘴替桌面微件载荷（支持 `"metaphor"` 诗意隐喻 vs `"metrics"` 数值开关）。
pub fn build_widget_preview(
    privacy_mode: &str,
    snapshot: &AnalyticsSnapshot,
    solar: &SolarTermState,
) -> WidgetStudioPayload {
    let mode = if privacy_mode.trim() == "metrics" {
        "metrics"
    } else {
        "metaphor"
    };

    let metaphor_headline = match snapshot.state.as_str() {
        "prime_ready" => "春风拂栏，宜乘兴而行",
        "recovery_priority" => "檐下煮茶，今日适宜留白",
        _ => "云淡风轻，步履守中自稳",
    };

    let hrv_str = snapshot
        .metrics
        .get("hrv_rmssd_ms")
        .or_else(|| snapshot.metrics.get("hrv_sdnn_ms"))
        .and_then(|m| m.value)
        .map(|v| format!("HRV {v:.0}ms"))
        .unwrap_or_else(|| "无连续HRV".to_owned());
    let rhr_str = snapshot
        .metrics
        .get("resting_heart_rate_bpm")
        .and_then(|m| m.value)
        .map(|v| format!("RHR {v:.0}bpm"))
        .unwrap_or_else(|| "RHR --".to_owned());
    let sleep_str = snapshot
        .sleep_last_night_hours
        .map(|h| format!("睡 {h:.1}h"))
        .unwrap_or_else(|| "纯步数模式".to_owned());
    let metrics_line = format!("{hrv_str} · {rhr_str} · {sleep_str}");

    if mode == "metrics" {
        WidgetStudioPayload {
            privacy_mode: "metrics".to_owned(),
            solar_term: solar.term_name.clone(),
            lunar_phase: solar.lunar_phase.clone(),
            headline: format!("元气 {} · {}", snapshot.qi_score, snapshot.state_badge),
            subline: snapshot.micro_action.clone(),
            badge_text: snapshot.state_badge.clone(),
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
            companion_line: "抬腕嘴替 · 已隐藏具体生理数值".to_owned(),
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
    let scroll = vec![
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
    ];

    let has_disproved_experiment = experiments
        .iter()
        .any(|e| e.outcome_verdict == "neutral" || e.outcome_verdict == "disproved");

    let badges = vec![
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
                "已点亮「窗台添柴计划」公益守望徽章，温暖另一扇窗前的长辈与青年。".into()
            } else {
                "参与自愿公益共建「窗台添柴计划」即可点亮此宋瓷徽章。".into()
            },
        },
    ];

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
    let solar_term = compute_solar_term_state(&snapshot.target_date, &settings.weather_mood);
    let seasonal_pass = build_seasonal_pass(ent, claimed_rewards);
    let keepsake_catalog = build_keepsake_catalog(settings, ent, claimed_rewards);
    let ym: String = snapshot.target_date.chars().take(7).collect();
    let sticker_quota = compute_sticker_quota(&ym, used_stickers_in_month, ent);
    let weekly_letters = build_companion_weekly_letters(
        "xiaoman",
        snapshot,
        &solar_term,
        intakes_count,
        stickers.len(),
        ent,
    );
    let co_sanctuary = build_co_sanctuary_state(settings, snapshot);
    let widget_preview = build_widget_preview(&settings.widget_privacy_mode, snapshot, &solar_term);
    let (four_seasons_scroll, ceramic_badges) = build_four_seasons_scroll_and_badges(
        settings,
        snapshot,
        &solar_term,
        experiments,
        stickers.len(),
        intakes_count,
        distinct_metric_days,
    );
    let preset_limit = if has_feature(ent, "preset_slots_unlimited") {
        None
    } else {
        Some(FREE_PRESET_MAX)
    };

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

        assert_eq!(bonsai_stage_for_dew(0).0, "萌芽");
        assert_eq!(bonsai_stage_for_dew(5).0, "抽枝");
        assert_eq!(bonsai_stage_for_dew(15).0, "含苞");
        assert_eq!(bonsai_stage_for_dew(30).0, "满树金桂");
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
}
