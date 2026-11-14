//! `qi-cloud` 全流程端到端集成测试：
//! 验证零健康明文表、Free 配额 429 拦截、Pro 解锁、E2EE 密文推拉鉴权、安全加固与 5.1.1(v) 物理销毁。

use std::collections::BTreeSet;
use std::sync::Arc;

use base64::{
    Engine as _,
    engine::general_purpose::{STANDARD as B64_STANDARD, URL_SAFE_NO_PAD},
};
use chrono::Utc;
use ed25519_dalek::{Signature, Signer, Verifier, VerifyingKey};
use qi_cloud::db::{
    ControlPlaneDb, DEFAULT_ATELIER_SIGNING_SEED, FREE_LIMIT_LAB_PARSES,
    HEARTH_GIFT_PASS_PRODUCT_ID, HEARTH_GIFT_PASS_TITLE_EN, HEARTH_GIFT_PASS_TITLE_ZH,
    OFFLINE_GRACE_SECONDS, OsetClaims, SubscriptionTier, default_signing_key,
    default_verifying_key, validate_production_signing_key,
};
use qi_cloud::{AppState, build_router, build_router_with_rate_limit};
use ring::rand::SystemRandom;
use ring::signature::{ECDSA_P256_SHA256_FIXED_SIGNING, EcdsaKeyPair};
use serde_json::{Value, json};

const TEST_ROOT_DER: &[u8] = &[
    0x30, 0x82, 0x02, 0x13, 0x30, 0x82, 0x01, 0x99, 0xa0, 0x03, 0x02, 0x01,
    0x02, 0x02, 0x14, 0x0a, 0x28, 0x75, 0x8b, 0x9d, 0xd1, 0x76, 0x15, 0xa4,
    0x80, 0x64, 0x4d, 0x12, 0xd7, 0xb3, 0x71, 0xe8, 0x3d, 0x83, 0x8b, 0x30,
    0x0a, 0x06, 0x08, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x03, 0x30,
    0x48, 0x31, 0x23, 0x30, 0x21, 0x06, 0x03, 0x55, 0x04, 0x03, 0x0c, 0x1a,
    0x51, 0x69, 0x20, 0x54, 0x65, 0x73, 0x74, 0x20, 0x41, 0x70, 0x70, 0x6c,
    0x65, 0x20, 0x52, 0x6f, 0x6f, 0x74, 0x20, 0x43, 0x41, 0x20, 0x2d, 0x20,
    0x47, 0x33, 0x31, 0x14, 0x30, 0x12, 0x06, 0x03, 0x55, 0x04, 0x0a, 0x0c,
    0x0b, 0x51, 0x69, 0x20, 0x54, 0x65, 0x73, 0x74, 0x20, 0x50, 0x4b, 0x49,
    0x31, 0x0b, 0x30, 0x09, 0x06, 0x03, 0x55, 0x04, 0x06, 0x13, 0x02, 0x55,
    0x53, 0x30, 0x20, 0x17, 0x0d, 0x32, 0x36, 0x31, 0x31, 0x31, 0x34, 0x30,
    0x38, 0x31, 0x33, 0x33, 0x35, 0x5a, 0x18, 0x0f, 0x32, 0x30, 0x35, 0x31,
    0x30, 0x37, 0x30, 0x36, 0x30, 0x38, 0x31, 0x33, 0x33, 0x35, 0x5a, 0x30,
    0x48, 0x31, 0x23, 0x30, 0x21, 0x06, 0x03, 0x55, 0x04, 0x03, 0x0c, 0x1a,
    0x51, 0x69, 0x20, 0x54, 0x65, 0x73, 0x74, 0x20, 0x41, 0x70, 0x70, 0x6c,
    0x65, 0x20, 0x52, 0x6f, 0x6f, 0x74, 0x20, 0x43, 0x41, 0x20, 0x2d, 0x20,
    0x47, 0x33, 0x31, 0x14, 0x30, 0x12, 0x06, 0x03, 0x55, 0x04, 0x0a, 0x0c,
    0x0b, 0x51, 0x69, 0x20, 0x54, 0x65, 0x73, 0x74, 0x20, 0x50, 0x4b, 0x49,
    0x31, 0x0b, 0x30, 0x09, 0x06, 0x03, 0x55, 0x04, 0x06, 0x13, 0x02, 0x55,
    0x53, 0x30, 0x76, 0x30, 0x10, 0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d,
    0x02, 0x01, 0x06, 0x05, 0x2b, 0x81, 0x04, 0x00, 0x22, 0x03, 0x62, 0x00,
    0x04, 0xc5, 0x4e, 0xfa, 0xda, 0xb2, 0x38, 0xa9, 0xca, 0xd9, 0x0e, 0xce,
    0xe3, 0x2c, 0x9b, 0xa3, 0xdd, 0xd7, 0x44, 0xec, 0xbe, 0x31, 0x01, 0x0b,
    0x35, 0x7d, 0x48, 0xa4, 0x89, 0x2d, 0xac, 0x90, 0xe9, 0xec, 0xec, 0x14,
    0x24, 0x6b, 0x6e, 0xf9, 0x8d, 0x99, 0x93, 0xbe, 0xef, 0xec, 0xdf, 0x1d,
    0x55, 0x84, 0xb3, 0x90, 0xc3, 0xbf, 0xfc, 0xd1, 0x68, 0x4b, 0x1a, 0x03,
    0xc2, 0x59, 0x69, 0x7a, 0xb0, 0x8b, 0x91, 0x5f, 0xfb, 0x59, 0xe2, 0xa0,
    0x07, 0x2a, 0x0e, 0x4c, 0xf1, 0xe6, 0x4d, 0x55, 0xb1, 0xb9, 0xf4, 0x3c,
    0x2e, 0xbc, 0x57, 0x08, 0x71, 0x1b, 0x44, 0x52, 0xe5, 0xd5, 0x5b, 0xe7,
    0x83, 0xa3, 0x42, 0x30, 0x40, 0x30, 0x1d, 0x06, 0x03, 0x55, 0x1d, 0x0e,
    0x04, 0x16, 0x04, 0x14, 0xf1, 0x99, 0xe1, 0x5e, 0x9f, 0x8a, 0x17, 0xde,
    0xba, 0xcd, 0xb3, 0xe5, 0x1b, 0x6b, 0x8a, 0xbf, 0xe7, 0x00, 0x75, 0xed,
    0x30, 0x0f, 0x06, 0x03, 0x55, 0x1d, 0x13, 0x01, 0x01, 0xff, 0x04, 0x05,
    0x30, 0x03, 0x01, 0x01, 0xff, 0x30, 0x0e, 0x06, 0x03, 0x55, 0x1d, 0x0f,
    0x01, 0x01, 0xff, 0x04, 0x04, 0x03, 0x02, 0x01, 0x06, 0x30, 0x0a, 0x06,
    0x08, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x03, 0x03, 0x68, 0x00,
    0x30, 0x65, 0x02, 0x30, 0x2d, 0x65, 0xaa, 0xb5, 0xaf, 0xe0, 0x88, 0x77,
    0xec, 0xb0, 0x8b, 0x99, 0x79, 0xd5, 0xd0, 0x80, 0x97, 0xc2, 0x93, 0x38,
    0xaf, 0xad, 0x1e, 0xd8, 0xaf, 0xdb, 0xb9, 0x6f, 0x81, 0xde, 0x87, 0x3c,
    0xe6, 0x46, 0xa1, 0xa3, 0x33, 0xda, 0xf3, 0xb6, 0x34, 0xc8, 0x81, 0x67,
    0xd2, 0xab, 0x0f, 0xe0, 0x02, 0x31, 0x00, 0xd9, 0x10, 0xc3, 0x0c, 0x91,
    0x9f, 0xb3, 0xc8, 0xaf, 0xd3, 0x58, 0x50, 0x5c, 0x06, 0x97, 0xc0, 0xb8,
    0x74, 0x35, 0xbb, 0x31, 0x87, 0xb0, 0x23, 0xeb, 0x6b, 0x2b, 0x8a, 0xd0,
    0x37, 0x88, 0x16, 0xc0, 0x79, 0xd1, 0x42, 0x81, 0xa2, 0x10, 0x54, 0x3f,
    0xff, 0xfb, 0x1d, 0x67, 0xc8, 0xa1, 0x98,
];

const TEST_INTER_DER: &[u8] = &[
    0x30, 0x82, 0x02, 0x1b, 0x30, 0x82, 0x01, 0xa2, 0xa0, 0x03, 0x02, 0x01,
    0x02, 0x02, 0x14, 0x3d, 0xc5, 0xbb, 0x98, 0xc7, 0x28, 0x9b, 0x79, 0x1c,
    0xa4, 0xed, 0x25, 0xbf, 0x22, 0x88, 0xeb, 0x24, 0x50, 0xce, 0x47, 0x30,
    0x0a, 0x06, 0x08, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x03, 0x30,
    0x48, 0x31, 0x23, 0x30, 0x21, 0x06, 0x03, 0x55, 0x04, 0x03, 0x0c, 0x1a,
    0x51, 0x69, 0x20, 0x54, 0x65, 0x73, 0x74, 0x20, 0x41, 0x70, 0x70, 0x6c,
    0x65, 0x20, 0x52, 0x6f, 0x6f, 0x74, 0x20, 0x43, 0x41, 0x20, 0x2d, 0x20,
    0x47, 0x33, 0x31, 0x14, 0x30, 0x12, 0x06, 0x03, 0x55, 0x04, 0x0a, 0x0c,
    0x0b, 0x51, 0x69, 0x20, 0x54, 0x65, 0x73, 0x74, 0x20, 0x50, 0x4b, 0x49,
    0x31, 0x0b, 0x30, 0x09, 0x06, 0x03, 0x55, 0x04, 0x06, 0x13, 0x02, 0x55,
    0x53, 0x30, 0x20, 0x17, 0x0d, 0x32, 0x36, 0x31, 0x31, 0x31, 0x34, 0x30,
    0x38, 0x31, 0x33, 0x33, 0x35, 0x5a, 0x18, 0x0f, 0x32, 0x30, 0x35, 0x31,
    0x30, 0x37, 0x30, 0x36, 0x30, 0x38, 0x31, 0x33, 0x33, 0x35, 0x5a, 0x30,
    0x4a, 0x31, 0x25, 0x30, 0x23, 0x06, 0x03, 0x55, 0x04, 0x03, 0x0c, 0x1c,
    0x51, 0x69, 0x20, 0x54, 0x65, 0x73, 0x74, 0x20, 0x57, 0x57, 0x44, 0x52,
    0x20, 0x49, 0x6e, 0x74, 0x65, 0x72, 0x6d, 0x65, 0x64, 0x69, 0x61, 0x74,
    0x65, 0x20, 0x43, 0x41, 0x31, 0x14, 0x30, 0x12, 0x06, 0x03, 0x55, 0x04,
    0x0a, 0x0c, 0x0b, 0x51, 0x69, 0x20, 0x54, 0x65, 0x73, 0x74, 0x20, 0x50,
    0x4b, 0x49, 0x31, 0x0b, 0x30, 0x09, 0x06, 0x03, 0x55, 0x04, 0x06, 0x13,
    0x02, 0x55, 0x53, 0x30, 0x59, 0x30, 0x13, 0x06, 0x07, 0x2a, 0x86, 0x48,
    0xce, 0x3d, 0x02, 0x01, 0x06, 0x08, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03,
    0x01, 0x07, 0x03, 0x42, 0x00, 0x04, 0xb1, 0x1c, 0xa7, 0xb1, 0x2f, 0xae,
    0x63, 0xcc, 0x31, 0x23, 0xaa, 0x56, 0x4b, 0xf1, 0xdc, 0x25, 0x8e, 0x7b,
    0x23, 0x9c, 0x48, 0xcb, 0x74, 0x9d, 0x62, 0x23, 0xe0, 0x7f, 0xc4, 0xfc,
    0x75, 0x73, 0x99, 0x86, 0x1b, 0xb8, 0xd4, 0x78, 0x1e, 0x19, 0x48, 0xe7,
    0x8c, 0xfb, 0x26, 0xa7, 0xea, 0xdd, 0xfd, 0x63, 0x63, 0x50, 0x9e, 0x3c,
    0xf3, 0x24, 0x51, 0x04, 0x97, 0xc0, 0x80, 0x7b, 0x88, 0x92, 0xa3, 0x66,
    0x30, 0x64, 0x30, 0x1d, 0x06, 0x03, 0x55, 0x1d, 0x0e, 0x04, 0x16, 0x04,
    0x14, 0x35, 0xc7, 0x9b, 0x6c, 0x65, 0xaa, 0xec, 0x75, 0xa6, 0xfe, 0xf0,
    0x72, 0xf7, 0x46, 0xde, 0x3c, 0xe2, 0xf9, 0xd2, 0xa7, 0x30, 0x1f, 0x06,
    0x03, 0x55, 0x1d, 0x23, 0x04, 0x18, 0x30, 0x16, 0x80, 0x14, 0xf1, 0x99,
    0xe1, 0x5e, 0x9f, 0x8a, 0x17, 0xde, 0xba, 0xcd, 0xb3, 0xe5, 0x1b, 0x6b,
    0x8a, 0xbf, 0xe7, 0x00, 0x75, 0xed, 0x30, 0x12, 0x06, 0x03, 0x55, 0x1d,
    0x13, 0x01, 0x01, 0xff, 0x04, 0x08, 0x30, 0x06, 0x01, 0x01, 0xff, 0x02,
    0x01, 0x00, 0x30, 0x0e, 0x06, 0x03, 0x55, 0x1d, 0x0f, 0x01, 0x01, 0xff,
    0x04, 0x04, 0x03, 0x02, 0x01, 0x06, 0x30, 0x0a, 0x06, 0x08, 0x2a, 0x86,
    0x48, 0xce, 0x3d, 0x04, 0x03, 0x03, 0x03, 0x67, 0x00, 0x30, 0x64, 0x02,
    0x30, 0x2c, 0x9a, 0x88, 0xb6, 0xe2, 0x16, 0x97, 0xfc, 0x72, 0xe4, 0x4c,
    0x5f, 0xd0, 0xb1, 0x96, 0xfa, 0x82, 0x7d, 0xcc, 0xd1, 0x46, 0x55, 0xcf,
    0x29, 0xcd, 0x5b, 0xfd, 0xb9, 0x7f, 0x8f, 0x5c, 0x8f, 0xd0, 0x2b, 0x62,
    0xc7, 0x25, 0x66, 0x26, 0xf0, 0x3b, 0x52, 0x4e, 0x9a, 0x5f, 0x70, 0xda,
    0x38, 0x02, 0x30, 0x30, 0x09, 0xba, 0xa2, 0x64, 0xf4, 0x9f, 0x47, 0x27,
    0x04, 0x1a, 0xb9, 0x15, 0x88, 0xbd, 0x04, 0xb9, 0xb1, 0x5e, 0x11, 0x04,
    0x79, 0x8b, 0x30, 0x9c, 0xf5, 0xed, 0x68, 0x10, 0xa1, 0x49, 0x09, 0x9d,
    0xff, 0x4d, 0xdd, 0x10, 0x47, 0xa7, 0xba, 0x71, 0x05, 0x94, 0x96, 0x60,
    0xb8, 0xdc, 0x8b,
];

const TEST_LEAF_DER: &[u8] = &[
    0x30, 0x82, 0x02, 0x13, 0x30, 0x82, 0x01, 0xb8, 0xa0, 0x03, 0x02, 0x01,
    0x02, 0x02, 0x14, 0x40, 0x9a, 0xdc, 0x0e, 0x50, 0xd0, 0x8f, 0xb3, 0x1c,
    0x7f, 0x9c, 0x36, 0x6c, 0x79, 0x21, 0xf2, 0x02, 0xd2, 0x43, 0x47, 0x30,
    0x0a, 0x06, 0x08, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x02, 0x30,
    0x4a, 0x31, 0x25, 0x30, 0x23, 0x06, 0x03, 0x55, 0x04, 0x03, 0x0c, 0x1c,
    0x51, 0x69, 0x20, 0x54, 0x65, 0x73, 0x74, 0x20, 0x57, 0x57, 0x44, 0x52,
    0x20, 0x49, 0x6e, 0x74, 0x65, 0x72, 0x6d, 0x65, 0x64, 0x69, 0x61, 0x74,
    0x65, 0x20, 0x43, 0x41, 0x31, 0x14, 0x30, 0x12, 0x06, 0x03, 0x55, 0x04,
    0x0a, 0x0c, 0x0b, 0x51, 0x69, 0x20, 0x54, 0x65, 0x73, 0x74, 0x20, 0x50,
    0x4b, 0x49, 0x31, 0x0b, 0x30, 0x09, 0x06, 0x03, 0x55, 0x04, 0x06, 0x13,
    0x02, 0x55, 0x53, 0x30, 0x20, 0x17, 0x0d, 0x32, 0x36, 0x31, 0x31, 0x31,
    0x34, 0x30, 0x38, 0x31, 0x33, 0x33, 0x35, 0x5a, 0x18, 0x0f, 0x32, 0x30,
    0x35, 0x31, 0x30, 0x37, 0x30, 0x36, 0x30, 0x38, 0x31, 0x33, 0x33, 0x35,
    0x5a, 0x30, 0x4f, 0x31, 0x2a, 0x30, 0x28, 0x06, 0x03, 0x55, 0x04, 0x03,
    0x0c, 0x21, 0x51, 0x69, 0x20, 0x54, 0x65, 0x73, 0x74, 0x20, 0x53, 0x74,
    0x6f, 0x72, 0x65, 0x4b, 0x69, 0x74, 0x32, 0x20, 0x52, 0x65, 0x63, 0x65,
    0x69, 0x70, 0x74, 0x20, 0x53, 0x69, 0x67, 0x6e, 0x69, 0x6e, 0x67, 0x31,
    0x14, 0x30, 0x12, 0x06, 0x03, 0x55, 0x04, 0x0a, 0x0c, 0x0b, 0x51, 0x69,
    0x20, 0x54, 0x65, 0x73, 0x74, 0x20, 0x50, 0x4b, 0x49, 0x31, 0x0b, 0x30,
    0x09, 0x06, 0x03, 0x55, 0x04, 0x06, 0x13, 0x02, 0x55, 0x53, 0x30, 0x59,
    0x30, 0x13, 0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01, 0x06,
    0x08, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07, 0x03, 0x42, 0x00,
    0x04, 0x26, 0x32, 0x15, 0x3f, 0xa7, 0x4a, 0x99, 0xbe, 0xb1, 0xdf, 0x7d,
    0x5d, 0x95, 0xb3, 0xfb, 0x65, 0xea, 0x3f, 0x61, 0xb1, 0xf6, 0xcd, 0x15,
    0x12, 0x68, 0x4c, 0x0e, 0xb2, 0xa4, 0x2f, 0x09, 0x0d, 0xd7, 0x36, 0x60,
    0xb1, 0xe8, 0x01, 0x80, 0xac, 0x3d, 0xa8, 0x62, 0xa1, 0xa7, 0x77, 0x6f,
    0x6e, 0x09, 0x92, 0xbf, 0xba, 0x42, 0xf8, 0x51, 0x80, 0x08, 0x50, 0xfe,
    0x86, 0x1f, 0x9a, 0xd4, 0x28, 0xa3, 0x75, 0x30, 0x73, 0x30, 0x1d, 0x06,
    0x03, 0x55, 0x1d, 0x0e, 0x04, 0x16, 0x04, 0x14, 0x41, 0xa9, 0xeb, 0x52,
    0xef, 0xc3, 0x76, 0x73, 0xcb, 0x95, 0xdb, 0xc4, 0x62, 0xd5, 0x26, 0x96,
    0x02, 0xbb, 0xc2, 0xca, 0x30, 0x1f, 0x06, 0x03, 0x55, 0x1d, 0x23, 0x04,
    0x18, 0x30, 0x16, 0x80, 0x14, 0x35, 0xc7, 0x9b, 0x6c, 0x65, 0xaa, 0xec,
    0x75, 0xa6, 0xfe, 0xf0, 0x72, 0xf7, 0x46, 0xde, 0x3c, 0xe2, 0xf9, 0xd2,
    0xa7, 0x30, 0x0c, 0x06, 0x03, 0x55, 0x1d, 0x13, 0x01, 0x01, 0xff, 0x04,
    0x02, 0x30, 0x00, 0x30, 0x0e, 0x06, 0x03, 0x55, 0x1d, 0x0f, 0x01, 0x01,
    0xff, 0x04, 0x04, 0x03, 0x02, 0x07, 0x80, 0x30, 0x13, 0x06, 0x03, 0x55,
    0x1d, 0x25, 0x04, 0x0c, 0x30, 0x0a, 0x06, 0x08, 0x2b, 0x06, 0x01, 0x05,
    0x05, 0x07, 0x03, 0x03, 0x30, 0x0a, 0x06, 0x08, 0x2a, 0x86, 0x48, 0xce,
    0x3d, 0x04, 0x03, 0x02, 0x03, 0x49, 0x00, 0x30, 0x46, 0x02, 0x21, 0x00,
    0xae, 0x26, 0xcb, 0xc5, 0xc9, 0x2c, 0xa5, 0x79, 0x80, 0x25, 0x6a, 0xc0,
    0x77, 0xda, 0xb7, 0xef, 0x96, 0x68, 0xd9, 0x72, 0xbe, 0xb2, 0x28, 0x95,
    0x72, 0x2d, 0x21, 0x36, 0x9f, 0x17, 0xf3, 0x8c, 0x02, 0x21, 0x00, 0xac,
    0x47, 0xbd, 0xab, 0x67, 0x6b, 0x6f, 0x8c, 0x62, 0xe6, 0x22, 0xa1, 0x54,
    0xac, 0xe4, 0x15, 0xf7, 0x63, 0x4d, 0x75, 0x9c, 0x29, 0x3a, 0x28, 0x76,
    0x03, 0xfd, 0x9e, 0x7a, 0x26, 0x5b, 0x80,
];

const TEST_LEAF_PKCS8_DER: &[u8] = &[
    0x30, 0x81, 0x87, 0x02, 0x01, 0x00, 0x30, 0x13, 0x06, 0x07, 0x2a, 0x86,
    0x48, 0xce, 0x3d, 0x02, 0x01, 0x06, 0x08, 0x2a, 0x86, 0x48, 0xce, 0x3d,
    0x03, 0x01, 0x07, 0x04, 0x6d, 0x30, 0x6b, 0x02, 0x01, 0x01, 0x04, 0x20,
    0xc7, 0xf5, 0x55, 0x20, 0x9c, 0xb2, 0x22, 0xc1, 0x16, 0x68, 0x16, 0xd8,
    0xa6, 0x3d, 0x04, 0x50, 0x6f, 0xcd, 0x1d, 0xf3, 0x4b, 0x96, 0x3c, 0x77,
    0xd1, 0x03, 0xdb, 0x80, 0x6e, 0x83, 0x92, 0x33, 0xa1, 0x44, 0x03, 0x42,
    0x00, 0x04, 0x26, 0x32, 0x15, 0x3f, 0xa7, 0x4a, 0x99, 0xbe, 0xb1, 0xdf,
    0x7d, 0x5d, 0x95, 0xb3, 0xfb, 0x65, 0xea, 0x3f, 0x61, 0xb1, 0xf6, 0xcd,
    0x15, 0x12, 0x68, 0x4c, 0x0e, 0xb2, 0xa4, 0x2f, 0x09, 0x0d, 0xd7, 0x36,
    0x60, 0xb1, 0xe8, 0x01, 0x80, 0xac, 0x3d, 0xa8, 0x62, 0xa1, 0xa7, 0x77,
    0x6f, 0x6e, 0x09, 0x92, 0xbf, 0xba, 0x42, 0xf8, 0x51, 0x80, 0x08, 0x50,
    0xfe, 0x86, 0x1f, 0x9a, 0xd4, 0x28,
];

const TEST_ROGUE_LEAF_DER: &[u8] = &[
    0x30, 0x82, 0x02, 0x10, 0x30, 0x82, 0x01, 0xb6, 0xa0, 0x03, 0x02, 0x01,
    0x02, 0x02, 0x14, 0x38, 0x35, 0x61, 0xa2, 0x34, 0xb9, 0x8a, 0x3b, 0x56,
    0x4b, 0x79, 0x74, 0x1e, 0x2f, 0x07, 0x72, 0xde, 0xa3, 0x57, 0x8b, 0x30,
    0x0a, 0x06, 0x08, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x02, 0x30,
    0x48, 0x31, 0x23, 0x30, 0x21, 0x06, 0x03, 0x55, 0x04, 0x03, 0x0c, 0x1a,
    0x51, 0x69, 0x20, 0x54, 0x65, 0x73, 0x74, 0x20, 0x41, 0x70, 0x70, 0x6c,
    0x65, 0x20, 0x52, 0x6f, 0x6f, 0x74, 0x20, 0x43, 0x41, 0x20, 0x2d, 0x20,
    0x47, 0x33, 0x31, 0x14, 0x30, 0x12, 0x06, 0x03, 0x55, 0x04, 0x0a, 0x0c,
    0x0b, 0x51, 0x69, 0x20, 0x54, 0x65, 0x73, 0x74, 0x20, 0x50, 0x4b, 0x49,
    0x31, 0x0b, 0x30, 0x09, 0x06, 0x03, 0x55, 0x04, 0x06, 0x13, 0x02, 0x55,
    0x53, 0x30, 0x20, 0x17, 0x0d, 0x32, 0x36, 0x31, 0x31, 0x31, 0x34, 0x30,
    0x38, 0x31, 0x33, 0x33, 0x36, 0x5a, 0x18, 0x0f, 0x32, 0x30, 0x35, 0x31,
    0x30, 0x37, 0x30, 0x36, 0x30, 0x38, 0x31, 0x33, 0x33, 0x36, 0x5a, 0x30,
    0x4f, 0x31, 0x2a, 0x30, 0x28, 0x06, 0x03, 0x55, 0x04, 0x03, 0x0c, 0x21,
    0x51, 0x69, 0x20, 0x54, 0x65, 0x73, 0x74, 0x20, 0x53, 0x74, 0x6f, 0x72,
    0x65, 0x4b, 0x69, 0x74, 0x32, 0x20, 0x52, 0x65, 0x63, 0x65, 0x69, 0x70,
    0x74, 0x20, 0x53, 0x69, 0x67, 0x6e, 0x69, 0x6e, 0x67, 0x31, 0x14, 0x30,
    0x12, 0x06, 0x03, 0x55, 0x04, 0x0a, 0x0c, 0x0b, 0x51, 0x69, 0x20, 0x54,
    0x65, 0x73, 0x74, 0x20, 0x50, 0x4b, 0x49, 0x31, 0x0b, 0x30, 0x09, 0x06,
    0x03, 0x55, 0x04, 0x06, 0x13, 0x02, 0x55, 0x53, 0x30, 0x59, 0x30, 0x13,
    0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01, 0x06, 0x08, 0x2a,
    0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07, 0x03, 0x42, 0x00, 0x04, 0xff,
    0x8e, 0xa8, 0xfa, 0x0f, 0xf8, 0xb6, 0x21, 0xef, 0x76, 0x4a, 0x14, 0x79,
    0x71, 0xd8, 0x69, 0x2a, 0xc7, 0xa6, 0x5b, 0x59, 0xfa, 0xf7, 0xf3, 0x9b,
    0x1a, 0x3c, 0x79, 0xe2, 0xd1, 0x90, 0x95, 0x88, 0xc8, 0x25, 0x8f, 0xcf,
    0x20, 0x28, 0x6c, 0xe1, 0xc8, 0x45, 0xf8, 0xcf, 0x06, 0xdc, 0x61, 0xd2,
    0x55, 0x4a, 0x30, 0x23, 0x32, 0xfe, 0x67, 0xad, 0xdb, 0x03, 0xe7, 0xce,
    0x3c, 0xe7, 0x6c, 0xa3, 0x75, 0x30, 0x73, 0x30, 0x1d, 0x06, 0x03, 0x55,
    0x1d, 0x0e, 0x04, 0x16, 0x04, 0x14, 0x56, 0x09, 0x14, 0x2b, 0x03, 0xc0,
    0x98, 0xbe, 0xa7, 0xb6, 0xe2, 0xab, 0xc0, 0xa1, 0x72, 0x2f, 0x6d, 0x62,
    0xa5, 0x6e, 0x30, 0x1f, 0x06, 0x03, 0x55, 0x1d, 0x23, 0x04, 0x18, 0x30,
    0x16, 0x80, 0x14, 0xcb, 0xc9, 0xae, 0x01, 0x41, 0x92, 0x75, 0x68, 0x84,
    0xb3, 0xc9, 0x42, 0xec, 0xd3, 0xdc, 0x00, 0x9d, 0x3e, 0xa1, 0xae, 0x30,
    0x0c, 0x06, 0x03, 0x55, 0x1d, 0x13, 0x01, 0x01, 0xff, 0x04, 0x02, 0x30,
    0x00, 0x30, 0x0e, 0x06, 0x03, 0x55, 0x1d, 0x0f, 0x01, 0x01, 0xff, 0x04,
    0x04, 0x03, 0x02, 0x07, 0x80, 0x30, 0x13, 0x06, 0x03, 0x55, 0x1d, 0x25,
    0x04, 0x0c, 0x30, 0x0a, 0x06, 0x08, 0x2b, 0x06, 0x01, 0x05, 0x05, 0x07,
    0x03, 0x03, 0x30, 0x0a, 0x06, 0x08, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04,
    0x03, 0x02, 0x03, 0x48, 0x00, 0x30, 0x45, 0x02, 0x21, 0x00, 0xef, 0xef,
    0xde, 0xc9, 0xd9, 0x23, 0x70, 0x5e, 0x34, 0x25, 0x4b, 0x53, 0xe9, 0xf3,
    0xfc, 0x56, 0x25, 0xac, 0xf0, 0xbb, 0xf5, 0xc4, 0xfd, 0x8c, 0xff, 0x04,
    0x8b, 0x23, 0x05, 0x51, 0x69, 0x43, 0x02, 0x20, 0x63, 0x0f, 0x69, 0x47,
    0x6d, 0x76, 0xc3, 0xa2, 0xe4, 0x29, 0xc1, 0x76, 0x78, 0xe9, 0xfd, 0xae,
    0x9b, 0x16, 0x9c, 0x60, 0xe2, 0xdd, 0x2b, 0xe2, 0xe4, 0x73, 0xea, 0x81,
    0x2b, 0x48, 0xdf, 0x45,
];

const TEST_ROGUE_LEAF_PKCS8_DER: &[u8] = &[
    0x30, 0x81, 0x87, 0x02, 0x01, 0x00, 0x30, 0x13, 0x06, 0x07, 0x2a, 0x86,
    0x48, 0xce, 0x3d, 0x02, 0x01, 0x06, 0x08, 0x2a, 0x86, 0x48, 0xce, 0x3d,
    0x03, 0x01, 0x07, 0x04, 0x6d, 0x30, 0x6b, 0x02, 0x01, 0x01, 0x04, 0x20,
    0xa2, 0x33, 0x01, 0x80, 0xf8, 0x50, 0x48, 0x31, 0xe1, 0x18, 0x43, 0xec,
    0x5c, 0x7b, 0x93, 0x1f, 0xcf, 0x1a, 0x16, 0x3f, 0xbd, 0x20, 0xca, 0xb5,
    0x2a, 0x39, 0xbb, 0xb2, 0x9d, 0xaf, 0x9c, 0x53, 0xa1, 0x44, 0x03, 0x42,
    0x00, 0x04, 0xff, 0x8e, 0xa8, 0xfa, 0x0f, 0xf8, 0xb6, 0x21, 0xef, 0x76,
    0x4a, 0x14, 0x79, 0x71, 0xd8, 0x69, 0x2a, 0xc7, 0xa6, 0x5b, 0x59, 0xfa,
    0xf7, 0xf3, 0x9b, 0x1a, 0x3c, 0x79, 0xe2, 0xd1, 0x90, 0x95, 0x88, 0xc8,
    0x25, 0x8f, 0xcf, 0x20, 0x28, 0x6c, 0xe1, 0xc8, 0x45, 0xf8, 0xcf, 0x06,
    0xdc, 0x61, 0xd2, 0x55, 0x4a, 0x30, 0x23, 0x32, 0xfe, 0x67, 0xad, 0xdb,
    0x03, 0xe7, 0xce, 0x3c, 0xe7, 0x6c,
];

async fn spawn_test_server_with_db(
    db: ControlPlaneDb,
    rate_limit_per_min: Option<usize>,
) -> (String, Arc<ControlPlaneDb>) {
    let db = Arc::new(db);
    let state = AppState {
        db: db.clone(),
        zdr_upstream: None,
    };
    let app = match rate_limit_per_min {
        Some(limit) => build_router_with_rate_limit(state, limit),
        None => build_router(state),
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    (format!("http://{addr}"), db)
}

async fn spawn_test_server() -> (String, Arc<ControlPlaneDb>) {
    let db = ControlPlaneDb::open_memory_with_config(false, None, Some(TEST_ROOT_DER.to_vec()))
        .unwrap();
    spawn_test_server_with_db(db, None).await
}

#[tokio::test]
async fn cloud_control_plane_quota_e2ee_and_physical_wipe() {
    let (base_url, db) = spawn_test_server().await;

    // 1. 审计服务端表清单：严禁出现任何健康明文表
    let tables = db.table_names().unwrap();
    assert_eq!(
        tables,
        vec![
            "accounts",
            "apple_iap_transactions",
            "e2ee_sync_envelopes",
            "monthly_quotas"
        ]
    );

    // 使用 blocking 线程跑 ureq 客户端模拟真实 HTTP 请求
    let base = base_url.clone();
    tokio::task::spawn_blocking(move || {
        let h_resp: Value = ureq::get(&format!("{base}/healthz"))
            .call()
            .unwrap()
            .into_json()
            .unwrap();
        assert_eq!(h_resp["health_plaintext_retained"], false);

        // 2. 注册并获取 Bearer Token + Free 权益
        let session: Value = ureq::post(&format!("{base}/v1/auth/session"))
            .send_json(json!({ "handle": "qi-tester@example.com" }))
            .unwrap()
            .into_json()
            .unwrap();
        let token = session["token"].as_str().unwrap().to_owned();
        let auth = format!("Bearer {token}");
        assert_eq!(session["plan_tier"], "free");
        assert_eq!(
            session["entitlements"]["lab_parses_remaining"],
            FREE_LIMIT_LAB_PARSES
        );

        // 3. Free 用户 ZDR 体检解读：第 1 次成功并带 no-store 响应头；第 2 次触发 429
        let ok_resp = ureq::post(&format!("{base}/v1/zdr/complete"))
            .set("Authorization", &auth)
            .send_json(json!({
                "task": "lab",
                "scrubbed_prompt": "姓名：陈默 身份证：31010419920518432X 尿酸 468 umol/L"
            }))
            .unwrap();
        assert_eq!(ok_resp.header("cache-control"), Some("no-store"));
        assert_eq!(
            ok_resp.header("x-qi-zdr"),
            Some("memory-only-zero-retention")
        );
        let ok_json: Value = ok_resp.into_json().unwrap();
        assert!(!ok_json["content"].as_str().unwrap().contains("陈默"));
        assert!(!ok_json["content"].as_str().unwrap().contains("31010419920518432X"));

        let err_429 = ureq::post(&format!("{base}/v1/zdr/complete"))
            .set("Authorization", &auth)
            .send_json(json!({ "task": "lab", "scrubbed_prompt": "second attempt" }))
            .unwrap_err();
        match err_429 {
            ureq::Error::Status(code, _) => assert_eq!(code, 429),
            other => panic!("expected 429 quota error, got {other}"),
        }

        // 4. 激活 Pro 后，ZDR 配额无限放行，且跨端 WebDAV / Ollama 权益解锁
        let pro_ent: Value = ureq::post(&format!("{base}/v1/billing/activate-pro"))
            .set("Authorization", &auth)
            .send_json(json!({ "tier": "pro" }))
            .unwrap()
            .into_json()
            .unwrap();
        assert_eq!(pro_ent["plan_tier"], "pro");
        assert_eq!(pro_ent["lab_parses_remaining"], -1);
        assert_eq!(pro_ent["local_ollama_unlocked"], true);

        let pro_ok = ureq::post(&format!("{base}/v1/zdr/complete"))
            .set("Authorization", &auth)
            .send_json(json!({ "task": "lab", "scrubbed_prompt": "pro unlimited" }))
            .unwrap();
        assert_eq!(pro_ok.status(), 200);

        // 5. E2EE 推送严格拒绝非 XChaCha20-Poly1305 明文载荷
        let bad_push = ureq::post(&format!("{base}/v1/sync/ops/push"))
            .set("Authorization", &auth)
            .send_json(json!({
                "device_id": "dev-a",
                "batch_seq": 1,
                "path": "ops/dev-a/000001.enc",
                "protocol": "open-health-sync-v2",
                "cipher": "plaintext",
                "nonce_hex": "00",
                "ciphertext_hex": "not-encrypted",
                "payload_bytes": 13
            }))
            .unwrap_err();
        match bad_push {
            ureq::Error::Status(code, _) => assert_eq!(code, 400),
            other => panic!("expected 400 invalid_e2ee_envelope, got {other}"),
        }

        // 6. 合规 XChaCha20 密文推送与跨设备排除自身拉取
        let valid_nonce = "a".repeat(48);
        let push_ok: Value = ureq::post(&format!("{base}/v1/sync/ops/push"))
            .set("Authorization", &auth)
            .send_json(json!({
                "device_id": "dev-iphone",
                "batch_seq": 1,
                "path": "ops/dev-iphone/000001.enc",
                "protocol": "open-health-sync-v2",
                "cipher": "XChaCha20-Poly1305",
                "nonce_hex": valid_nonce,
                "ciphertext_hex": "deadbeefcafe01020304",
                "payload_bytes": 64
            }))
            .unwrap()
            .into_json()
            .unwrap();
        assert_eq!(push_ok["status"], "stored_ciphertext_only");

        let pull_b: Value = ureq::get(&format!("{base}/v1/sync/ops/pull?after_id=0&exclude_device=dev-mac"))
            .set("Authorization", &auth)
            .call()
            .unwrap()
            .into_json()
            .unwrap();
        assert_eq!(pull_b["count"], 1);
        assert_eq!(pull_b["envelopes"][0]["device_id"], "dev-iphone");

        // 7. Apple 5.1.1(v) 一键物理销毁账号及全部密文信封
        let wipe: Value = ureq::delete(&format!("{base}/v1/account"))
            .set("Authorization", &auth)
            .call()
            .unwrap()
            .into_json()
            .unwrap();
        assert_eq!(wipe["deleted_accounts"], 1);
        assert_eq!(wipe["deleted_e2ee_envelopes"], 1);

        // 销毁后旧 Token 立即 401
        let after_del = ureq::get(&format!("{base}/v1/entitlements"))
            .set("Authorization", &auth)
            .call()
            .unwrap_err();
        match after_del {
            ureq::Error::Status(code, _) => assert_eq!(code, 401),
            other => panic!("expected 401 after wipe, got {other}"),
        }
    })
    .await
    .unwrap();
}

fn verify_and_decode_oset_with_key(token: &str, vk: &VerifyingKey) -> OsetClaims {
    let parts: Vec<&str> = token.split('.').collect();
    assert_eq!(parts.len(), 3, "OSET token must have 3 dot-separated parts");
    assert_eq!(parts[0], "qient1");

    let sig_bytes = URL_SAFE_NO_PAD.decode(parts[2]).unwrap();
    let sig = Signature::from_slice(&sig_bytes).unwrap();
    let signing_input = format!("{}.{}", parts[0], parts[1]);
    vk.verify(signing_input.as_bytes(), &sig)
        .expect("Ed25519 OSET signature must verify with expected verifying key");

    let claims_bytes = URL_SAFE_NO_PAD.decode(parts[1]).unwrap();
    serde_json::from_slice(&claims_bytes).unwrap()
}

fn verify_and_decode_oset(token: &str) -> OsetClaims {
    let vk = default_verifying_key();
    verify_and_decode_oset_with_key(token, &vk)
}

#[tokio::test]
async fn oset_ed25519_minting_schema_aliases_and_refresh() {
    let (base_url, db) = spawn_test_server().await;
    assert_eq!(
        db.table_names().unwrap(),
        vec![
            "accounts",
            "apple_iap_transactions",
            "e2ee_sync_envelopes",
            "monthly_quotas"
        ]
    );

    let base = base_url.clone();
    tokio::task::spawn_blocking(move || {
        let session: Value = ureq::post(&format!("{base}/v1/auth/session"))
            .send_json(json!({ "handle": "atelier-oset@example.com" }))
            .unwrap()
            .into_json()
            .unwrap();
        let account_id = session["account_id"].as_str().unwrap().to_owned();
        let auth = format!("Bearer {}", session["token"].as_str().unwrap());

        // Free tier aliases & empty OSET token
        let free_ent = &session["entitlements"];
        assert_eq!(free_ent["year_month"], free_ent["month"]);
        assert_eq!(
            free_ent["cross_platform_webdav_unlocked"],
            free_ent["webdav_unlocked"]
        );
        assert_eq!(free_ent["webdav_unlocked"], false);
        assert_eq!(free_ent["entitlement_token"], "");
        assert_eq!(free_ent["entitlement_expires_epoch"], 0);
        assert_eq!(free_ent["seats"], 1);
        assert_eq!(free_ent["guardian_sponsor"], false);
        assert_eq!(free_ent["companion_loyalty_days"], 0);

        // Reject invalid tier on mock-upgrade
        let bad_tier = ureq::post(&format!("{base}/v1/billing/mock-upgrade"))
            .set("Authorization", &auth)
            .send_json(json!({ "tier": "nonexistent_tier" }))
            .unwrap_err();
        match bad_tier {
            ureq::Error::Status(code, _) => assert_eq!(code, 400),
            other => panic!("expected 400 invalid_tier, got {other}"),
        }

        // Verify all paid tiers mint valid Ed25519 OSET tokens
        let cases = [
            ("seasonal", 1u8, true, 35i64, true, true),
            ("pro_annual", 1u8, false, 12i64, false, true),
            ("duo", 2u8, true, 30i64, true, true),
            ("family", 4u8, false, 45i64, true, true),
            ("lifetime_local", 1u8, true, 60i64, true, false),
        ];

        for (tier, expected_seats, sponsor, loyalty_days, expect_loyalty, expect_cloud_sync) in
            cases
        {
            let ent: Value = ureq::post(&format!("{base}/v1/billing/mock-upgrade"))
                .set("Authorization", &auth)
                .send_json(json!({
                    "tier": tier,
                    "guardian_sponsor": sponsor,
                    "companion_loyalty_days": loyalty_days
                }))
                .unwrap()
                .into_json()
                .unwrap();

            assert_eq!(ent["plan_tier"], tier);
            assert_eq!(ent["year_month"], ent["month"]);
            assert_eq!(ent["cross_platform_webdav_unlocked"], true);
            assert_eq!(ent["webdav_unlocked"], true);
            assert_eq!(ent["seats"], expected_seats);
            assert_eq!(ent["guardian_sponsor"], sponsor);
            assert_eq!(ent["companion_loyalty_days"], loyalty_days);
            assert!(!ent["tier_display"].as_str().unwrap().is_empty());
            assert!(ent["entitlement_expires_epoch"].as_i64().unwrap() > 0);

            let oset_str = ent["entitlement_token"].as_str().unwrap();
            assert!(oset_str.starts_with("qient1."));
            let claims = verify_and_decode_oset(oset_str);
            assert_eq!(claims.ver, 1);
            assert_eq!(claims.iss, "qi-atelier-auth");
            assert_eq!(claims.sub, account_id);
            assert_eq!(claims.tier, tier);
            assert_eq!(claims.seats, expected_seats);
            assert_eq!(claims.guardian_sponsor, sponsor);
            assert_eq!(claims.companion_loyalty_eligible, expect_loyalty);
            assert_eq!(
                claims.grace_until,
                claims.exp_epoch + OFFLINE_GRACE_SECONDS
            );
            assert_eq!(
                claims.features.iter().any(|f| f == "cloud_e2ee_sync"),
                expect_cloud_sync
            );
            assert!(claims.features.iter().any(|f| f == "raw_biometrics"));
        }

        // Refresh endpoint issues a fresh valid OSET token with distinct nonce
        let prev_ent: Value = ureq::get(&format!("{base}/v1/entitlements"))
            .set("Authorization", &auth)
            .call()
            .unwrap()
            .into_json()
            .unwrap();
        let prev_claims = verify_and_decode_oset(prev_ent["entitlement_token"].as_str().unwrap());

        let refreshed: Value = ureq::post(&format!("{base}/v1/entitlements/refresh"))
            .set("Authorization", &auth)
            .send_json(json!({}))
            .unwrap()
            .into_json()
            .unwrap();
        let refreshed_claims =
            verify_and_decode_oset(refreshed["entitlement_token"].as_str().unwrap());
        assert_eq!(refreshed_claims.tier, "lifetime_local");
        assert_ne!(refreshed_claims.nonce, prev_claims.nonce);

        // Paid seasonal/family/etc. tiers unlock unlimited ZDR quota
        let zdr_ok = ureq::post(&format!("{base}/v1/zdr/complete"))
            .set("Authorization", &auth)
            .send_json(json!({ "task": "lab", "scrubbed_prompt": "atelier lab check" }))
            .unwrap();
        assert_eq!(zdr_ok.status(), 200);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn cosanctuary_privacy_first_pair_mood_care_and_dew() {
    let (base_url, db) = spawn_test_server().await;
    assert_eq!(
        db.table_names().unwrap(),
        vec![
            "accounts",
            "apple_iap_transactions",
            "e2ee_sync_envelopes",
            "monthly_quotas"
        ]
    );

    let base = base_url.clone();
    tokio::task::spawn_blocking(move || {
        let sess_a: Value = ureq::post(&format!("{base}/v1/auth/session"))
            .send_json(json!({ "handle": "partner-a@example.com" }))
            .unwrap()
            .into_json()
            .unwrap();
        let auth_a = format!("Bearer {}", sess_a["token"].as_str().unwrap());

        let sess_b: Value = ureq::post(&format!("{base}/v1/auth/session"))
            .send_json(json!({ "handle": "partner-b@example.com" }))
            .unwrap()
            .into_json()
            .unwrap();
        let auth_b = format!("Bearer {}", sess_b["token"].as_str().unwrap());

        // 0. Free 用户未升级房主前不允许创建房间或同步 mood，返回 403
        let free_pair_err = ureq::post(&format!("{base}/v1/cosanctuary/pair"))
            .set("Authorization", &auth_a)
            .send_json(json!({}))
            .unwrap_err();
        match free_pair_err {
            ureq::Error::Status(status, _) => assert_eq!(status, 403),
            other => panic!("expected 403 cosanctuary host required for free user, got {other}"),
        }
        let free_mood_err = ureq::post(&format!("{base}/v1/cosanctuary/mood"))
            .set("Authorization", &auth_a)
            .send_json(json!({ "mood": 1 }))
            .unwrap_err();
        match free_mood_err {
            ureq::Error::Status(status, _) => assert_eq!(status, 403),
            other => panic!("expected 403 cosanctuary mood rejected for unpaired free user, got {other}"),
        }

        // 将房主 partner-a 升级为 Duo，partner-b 保持 Free 免费受邀加入
        let host_ent: Value = ureq::post(&format!("{base}/v1/billing/mock-upgrade"))
            .set("Authorization", &auth_a)
            .send_json(json!({ "tier": "duo" }))
            .unwrap()
            .into_json()
            .unwrap();
        assert_eq!(host_ent["plan_tier"], "duo");

        // 1. Account A generates 6-char uppercase pair code
        let pair_a: Value = ureq::post(&format!("{base}/v1/cosanctuary/pair"))
            .set("Authorization", &auth_a)
            .send_json(json!({}))
            .unwrap()
            .into_json()
            .unwrap();
        assert_eq!(pair_a["paired"], false);
        let code = pair_a["pair_code"].as_str().unwrap().to_owned();
        assert_eq!(code.len(), 6);
        assert!(
            code.chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        );

        // Self-pairing is rejected
        let self_pair = ureq::post(&format!("{base}/v1/cosanctuary/pair"))
            .set("Authorization", &auth_a)
            .send_json(json!({ "pair_code": code }))
            .unwrap_err();
        match self_pair {
            ureq::Error::Status(status, _) => assert_eq!(status, 400),
            other => panic!("expected 400 self-pair error, got {other}"),
        }

        // 2. Account B pairs symmetrically using A's code
        let pair_b: Value = ureq::post(&format!("{base}/v1/cosanctuary/pair"))
            .set("Authorization", &auth_b)
            .send_json(json!({ "pair_code": code }))
            .unwrap()
            .into_json()
            .unwrap();
        assert_eq!(pair_b["paired"], true);
        assert_eq!(pair_b["pair_code"], code);
        assert_eq!(pair_b["bonsai_dew"], 0);

        // 3. Account A sets 1-byte mood enum + outfit; invalid >2 rejected
        let bad_mood = ureq::post(&format!("{base}/v1/cosanctuary/mood"))
            .set("Authorization", &auth_a)
            .send_json(json!({ "mood": 3 }))
            .unwrap_err();
        match bad_mood {
            ureq::Error::Status(status, _) => assert_eq!(status, 400),
            other => panic!("expected 400 invalid mood, got {other}"),
        }

        let mood_a: Value = ureq::post(&format!("{base}/v1/cosanctuary/mood"))
            .set("Authorization", &auth_a)
            .send_json(json!({ "mood": 2, "outfit_id": "linen_haori" }))
            .unwrap()
            .into_json()
            .unwrap();
        assert_eq!(mood_a["self"]["mood"], 2);
        assert_eq!(mood_a["self"]["mood_label"], "COZY_RECOVER");
        assert_eq!(mood_a["self"]["outfit_id"], "linen_haori");

        // 4. Mutual care delivery & bonsai_dew increments on both sides
        let bad_care = ureq::post(&format!("{base}/v1/cosanctuary/care"))
            .set("Authorization", &auth_a)
            .send_json(json!({ "care_action": "send_biometrics" }))
            .unwrap_err();
        match bad_care {
            ureq::Error::Status(status, _) => assert_eq!(status, 400),
            other => panic!("expected 400 invalid care_action, got {other}"),
        }

        let care_a: Value = ureq::post(&format!("{base}/v1/cosanctuary/care"))
            .set("Authorization", &auth_a)
            .send_json(json!({ "care_action": "warm_tea" }))
            .unwrap()
            .into_json()
            .unwrap();
        assert_eq!(care_a["bonsai_dew"], 1);
        assert_eq!(care_a["partner"]["care_action"], "warm_tea");

        let care_b: Value = ureq::post(&format!("{base}/v1/cosanctuary/care"))
            .set("Authorization", &auth_b)
            .send_json(json!({ "care_action": "paw_stamp" }))
            .unwrap()
            .into_json()
            .unwrap();
        assert_eq!(care_b["bonsai_dew"], 2);
        assert_eq!(care_b["self"]["care_action"], "warm_tea");
        assert_eq!(care_b["partner"]["care_action"], "paw_stamp");

        // 5. Status payload strictly contains zero health/biometric/lab/diet fields
        let status_a: Value = ureq::get(&format!("{base}/v1/cosanctuary/status"))
            .set("Authorization", &auth_a)
            .call()
            .unwrap()
            .into_json()
            .unwrap();
        assert_eq!(status_a["paired"], true);
        assert_eq!(status_a["bonsai_dew"], 2);
        assert_eq!(status_a["self"]["care_action"], "paw_stamp");
        assert_eq!(status_a["partner"]["care_action"], "warm_tea");

        let top_keys: BTreeSet<&str> = status_a
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            top_keys,
            BTreeSet::from(["bonsai_dew", "pair_code", "paired", "partner", "self"])
        );
        let party_keys: BTreeSet<&str> = status_a["partner"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            party_keys,
            BTreeSet::from(["care_action", "mood", "mood_label", "outfit_id", "updated_at"])
        );
    })
    .await
    .unwrap();
}

#[test]
fn idempotent_alter_table_migration_from_legacy_accounts_schema() {
    let mut path = std::env::temp_dir();
    path.push(format!("qi_cloud_legacy_mig_{}.sqlite3", std::process::id()));
    let _ = std::fs::remove_file(&path);

    // Create a legacy 3-table schema without the new v4.0 columns
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE accounts (
                account_id TEXT PRIMARY KEY,
                handle TEXT UNIQUE NOT NULL,
                token_hash TEXT NOT NULL,
                plan_tier TEXT NOT NULL DEFAULT 'free',
                created_at TEXT NOT NULL
            );
            CREATE TABLE monthly_quotas (
                account_id TEXT NOT NULL,
                year_month TEXT NOT NULL,
                meal_scans INTEGER NOT NULL DEFAULT 0,
                lab_parses INTEGER NOT NULL DEFAULT 0,
                briefings INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (account_id, year_month)
            );
            CREATE TABLE e2ee_sync_envelopes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                account_id TEXT NOT NULL,
                device_id TEXT NOT NULL,
                batch_seq INTEGER NOT NULL,
                path TEXT NOT NULL,
                protocol TEXT NOT NULL,
                cipher TEXT NOT NULL,
                nonce_hex TEXT NOT NULL,
                ciphertext_hex TEXT NOT NULL,
                payload_bytes INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                UNIQUE(account_id, device_id, batch_seq)
            );
            "#,
        )
        .unwrap();
    }

    // Opening via ControlPlaneDb::open_file runs idempotent migrations & keeps zero-health-plaintext invariant
    let db = ControlPlaneDb::open_file(&path).unwrap();
    assert_eq!(
        db.table_names().unwrap(),
        vec![
            "accounts",
            "apple_iap_transactions",
            "e2ee_sync_envelopes",
            "monthly_quotas"
        ]
    );
    let (id, _, ent) = db.create_or_rotate_session("legacy-user@example.com").unwrap();
    assert_eq!(ent.plan_tier, "free");
    assert_eq!(ent.hearth_gift_credits, 0);
    let upgraded = db
        .upgrade_plan(&id, "duo", Some(true), Some(31))
        .unwrap();
    assert_eq!(upgraded.seats, 2);
    assert!(upgraded.entitlement_token.starts_with("qient1."));

    // Re-opening is completely idempotent
    drop(db);
    let db2 = ControlPlaneDb::open_file(&path).unwrap();
    assert_eq!(
        db2.table_names().unwrap(),
        vec![
            "accounts",
            "apple_iap_transactions",
            "e2ee_sync_envelopes",
            "monthly_quotas"
        ]
    );
    let _ = std::fs::remove_file(&path);
}

fn make_jws_es256_fake_sig(claims: &Value, include_x5c: bool) -> String {
    let header_val = if include_x5c {
        json!({
            "alg": "ES256",
            "typ": "JWT",
            "x5c": [
                B64_STANDARD.encode(TEST_LEAF_DER),
                B64_STANDARD.encode(TEST_INTER_DER),
            ]
        })
    } else {
        json!({ "alg": "ES256", "typ": "JWT" })
    };
    let header = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header_val).unwrap());
    let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(claims).unwrap());
    let sig = URL_SAFE_NO_PAD.encode(&[0x42u8; 64]);
    format!("{header}.{payload}.{sig}")
}

fn make_jws_es256_signed_with_chain(
    claims: &Value,
    pkcs8_der: &[u8],
    x5c_certs: &[&[u8]],
) -> String {
    let rng = SystemRandom::new();
    let key_pair = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, pkcs8_der, &rng)
        .expect("valid PKCS8 P-256 test signing key");
    let x5c: Vec<String> = x5c_certs.iter().map(|c| B64_STANDARD.encode(c)).collect();
    let header_val = if x5c.is_empty() {
        json!({ "alg": "ES256", "typ": "JWT" })
    } else {
        json!({ "alg": "ES256", "typ": "JWT", "x5c": x5c })
    };
    let header = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header_val).unwrap());
    let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(claims).unwrap());
    let signing_input = format!("{header}.{payload}");
    let sig = key_pair.sign(&rng, signing_input.as_bytes()).unwrap();
    let sig_b64 = URL_SAFE_NO_PAD.encode(sig.as_ref());
    format!("{signing_input}.{sig_b64}")
}

fn make_jws_es256_signed(claims: &Value) -> String {
    make_jws_es256_signed_with_chain(claims, TEST_LEAF_PKCS8_DER, &[TEST_LEAF_DER, TEST_INTER_DER])
}

fn make_jws_eddsa_signed(claims: &Value) -> String {
    let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"EdDSA","typ":"JWT","kid":"qi-sandbox-ed25519"}"#);
    let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(claims).unwrap());
    let signing_input = format!("{header}.{payload}");
    let sig = default_signing_key().sign(signing_input.as_bytes());
    let sig_b64 = URL_SAFE_NO_PAD.encode(sig.to_bytes());
    format!("{signing_input}.{sig_b64}")
}

#[tokio::test]
async fn apple_iap_verify_subscription_idempotency_hearth_gift_pass_and_rejections() {
    let (base_url, db) = spawn_test_server().await;
    let base = base_url.clone();

    tokio::task::spawn_blocking(move || {
        // 0. Unauthenticated request is rejected with 401
        let unauth_err = ureq::post(&format!("{base}/v1/billing/apple/verify"))
            .send_json(json!({ "product_id": "com.qi.atelier.pro_annual" }))
            .unwrap_err();
        match unauth_err {
            ureq::Error::Status(code, _) => assert_eq!(code, 401),
            other => panic!("expected 401 unauthenticated, got {other}"),
        }

        // Register accounts A, B, C
        let sess_a: Value = ureq::post(&format!("{base}/v1/auth/session"))
            .send_json(json!({ "handle": "apple-iap-a@example.com" }))
            .unwrap()
            .into_json()
            .unwrap();
        let account_a = sess_a["account_id"].as_str().unwrap().to_owned();
        let auth_a = format!("Bearer {}", sess_a["token"].as_str().unwrap());

        let sess_b: Value = ureq::post(&format!("{base}/v1/auth/session"))
            .send_json(json!({ "handle": "apple-iap-b@example.com" }))
            .unwrap()
            .into_json()
            .unwrap();
        let auth_b = format!("Bearer {}", sess_b["token"].as_str().unwrap());

        let sess_c: Value = ureq::post(&format!("{base}/v1/auth/session"))
            .send_json(json!({ "handle": "apple-iap-c@example.com" }))
            .unwrap()
            .into_json()
            .unwrap();
        let account_c = sess_c["account_id"].as_str().unwrap().to_owned();
        let auth_c = format!("Bearer {}", sess_c["token"].as_str().unwrap());

        let future_ms = (Utc::now().timestamp() + 365 * 86_400) * 1000;
        let sub_jws = make_jws_es256_signed(&json!({
            "transactionId": "2000000100001",
            "originalTransactionId": "2000000100001",
            "bundleId": "eu.org.cluas.qi",
            "productId": "com.qi.atelier.pro_annual",
            "type": "Auto-Renewable Subscription",
            "environment": "Sandbox",
            "expiresDate": future_ms,
            "quantity": 1
        }));

        // 1. Subscription JWS verification mints Ed25519 OSET (qient1.*)
        let sub_resp: Value = ureq::post(&format!("{base}/v1/billing/apple/verify"))
            .set("Authorization", &auth_a)
            .send_json(json!({
                "jws_representation": sub_jws,
                "companion_loyalty_days": 35
            }))
            .unwrap()
            .into_json()
            .unwrap();

        assert_eq!(sub_resp["verified"], true);
        assert_eq!(sub_resp["verification_mode"], "storekit2_jws_es256");
        assert_eq!(sub_resp["product_id"], "com.qi.atelier.pro_annual");
        assert_eq!(sub_resp["plan_tier"], "pro_annual");
        assert_eq!(sub_resp["idempotent_replay"], false);
        assert_eq!(sub_resp["seats_credited"], 0);
        assert!(sub_resp["guardian_receipt_token"].is_null());
        let oset = sub_resp["entitlement_token"].as_str().unwrap();
        assert!(oset.starts_with("qient1."));
        let claims = verify_and_decode_oset(oset);
        assert_eq!(claims.sub, account_a);
        assert_eq!(claims.tier, "pro_annual");
        assert_eq!(claims.companion_loyalty_eligible, true);
        assert_eq!(db.count_apple_transactions_for_account(&account_a).unwrap(), 1);

        // Pro entitlement unlocks ZDR unlimited calls
        let zdr_ok = ureq::post(&format!("{base}/v1/zdr/complete"))
            .set("Authorization", &auth_a)
            .send_json(json!({ "task": "lab", "scrubbed_prompt": "apple iap pro check" }))
            .unwrap();
        assert_eq!(zdr_ok.status(), 200);

        // 2. Restore / replay idempotency with same transaction_id
        let replay_resp: Value = ureq::post(&format!("{base}/v1/billing/apple/verify"))
            .set("Authorization", &auth_a)
            .send_json(json!({
                "jws_representation": sub_jws,
                "product_id": "com.qi.atelier.pro_annual",
                "transaction_id": "2000000100001",
                "original_transaction_id": "2000000100001"
            }))
            .unwrap()
            .into_json()
            .unwrap();
        assert_eq!(replay_resp["idempotent_replay"], true);
        assert_eq!(replay_resp["plan_tier"], "pro_annual");
        assert!(replay_resp["entitlement_token"].as_str().unwrap().starts_with("qient1."));
        assert_eq!(db.count_apple_transactions_for_account(&account_a).unwrap(), 1);

        // 3. Renewal on same original_transaction_id with new transaction_id & extended expiry
        let extended_sec = Utc::now().timestamp() + 400 * 86_400;
        let renew_jws = make_jws_eddsa_signed(&json!({
            "transactionId": "2000000100002",
            "originalTransactionId": "2000000100001",
            "bundleId": "eu.org.cluas.qi",
            "productId": "com.qi.atelier.pro_annual",
            "environment": "Sandbox",
            "expiresDate": extended_sec * 1000
        }));
        let renew_resp: Value = ureq::post(&format!("{base}/v1/billing/apple/verify"))
            .set("Authorization", &auth_a)
            .send_json(json!({ "jws_representation": renew_jws }))
            .unwrap()
            .into_json()
            .unwrap();
        assert_eq!(renew_resp["idempotent_replay"], false);
        assert_eq!(renew_resp["verification_mode"], "storekit2_jws_eddsa");
        assert_eq!(
            renew_resp["entitlements"]["entitlement_expires_epoch"],
            extended_sec
        );
        assert_eq!(db.count_apple_transactions_for_account(&account_a).unwrap(), 2);

        // 4. Commercial Consumable Hearth Gift Pass credit + idempotency + guideline 3.2.1(vi) compliance
        let gift_jws_1 = make_jws_es256_signed(&json!({
            "transactionId": "3000000100001",
            "originalTransactionId": "3000000100001",
            "bundleId": "eu.org.cluas.qi",
            "productId": HEARTH_GIFT_PASS_PRODUCT_ID,
            "type": "Consumable",
            "environment": "Xcode",
            "quantity": 1
        }));
        let gift_resp_1: Value = ureq::post(&format!("{base}/v1/billing/apple/verify"))
            .set("Authorization", &auth_a)
            .send_json(json!({ "jws_representation": gift_jws_1 }))
            .unwrap()
            .into_json()
            .unwrap();
        assert_eq!(gift_resp_1["product_id"], HEARTH_GIFT_PASS_PRODUCT_ID);
        assert_eq!(gift_resp_1["purchase_type"], "consumable");
        assert_eq!(gift_resp_1["idempotent_replay"], false);
        assert_eq!(gift_resp_1["seats_credited"], 1);
        assert_eq!(gift_resp_1["hearth_gift_credits_total"], 1);
        assert_eq!(gift_resp_1["plan_tier"], "pro_annual");
        assert_eq!(gift_resp_1["hearth_gift_pass_title_zh"], HEARTH_GIFT_PASS_TITLE_ZH);
        assert_eq!(gift_resp_1["hearth_gift_pass_title_en"], HEARTH_GIFT_PASS_TITLE_EN);
        let gift_code_1 = gift_resp_1["community_gift_code"].as_str().unwrap().to_owned();
        assert!(gift_code_1.starts_with("QI-SEAT-"));

        let resp_str = serde_json::to_string(&gift_resp_1).unwrap().to_lowercase();
        for forbidden in [
            "\u{516c}\u{76ca}",
            "\u{6148}\u{5584}",
            "\u{6350}",
            concat!("char", "ity"),
            concat!("don", "at"),
        ] {
            assert!(!resp_str.contains(forbidden), "forbidden wording found: {forbidden}");
        }

        // Replay same consumable transaction -> idempotent, must NOT double-credit
        let gift_replay: Value = ureq::post(&format!("{base}/v1/billing/apple/verify"))
            .set("Authorization", &auth_a)
            .send_json(json!({ "jws_representation": gift_jws_1 }))
            .unwrap()
            .into_json()
            .unwrap();
        assert_eq!(gift_replay["idempotent_replay"], true);
        assert_eq!(gift_replay["seats_credited"], 0);
        assert_eq!(gift_replay["hearth_gift_credits_total"], 1);
        assert_eq!(gift_replay["community_gift_code"], gift_code_1);

        // Second distinct consumable transaction increments credits to 2
        let gift_jws_2 = make_jws_es256_signed(&json!({
            "transactionId": "3000000100002",
            "originalTransactionId": "3000000100002",
            "bundleId": "eu.org.cluas.qi",
            "productId": HEARTH_GIFT_PASS_PRODUCT_ID,
            "type": "Consumable",
            "environment": "Sandbox",
            "quantity": 1
        }));
        let gift_resp_2: Value = ureq::post(&format!("{base}/v1/billing/apple/verify"))
            .set("Authorization", &auth_a)
            .send_json(json!({ "jws_representation": gift_jws_2 }))
            .unwrap()
            .into_json()
            .unwrap();
        assert_eq!(gift_resp_2["idempotent_replay"], false);
        assert_eq!(gift_resp_2["seats_credited"], 1);
        assert_eq!(gift_resp_2["hearth_gift_credits_total"], 2);

        // Free user C buying Hearth Gift Pass in Sandbox keeps plan_tier == "free" and "" subscription token,
        // while minting a verifiable free/guardian_sponsor guardian_receipt_token
        let gift_free_req = json!({
            "environment": "Sandbox",
            "product_id": HEARTH_GIFT_PASS_PRODUCT_ID,
            "transaction_id": "3000000900001",
            "original_transaction_id": "3000000900001",
            "bundle_id": "eu.org.cluas.qi"
        });
        let gift_free: Value = ureq::post(&format!("{base}/v1/billing/apple/verify"))
            .set("Authorization", &auth_c)
            .send_json(gift_free_req.clone())
            .unwrap()
            .into_json()
            .unwrap();
        assert_eq!(gift_free["plan_tier"], "free");
        assert_eq!(gift_free["entitlement_token"], "");
        assert_eq!(gift_free["hearth_gift_credits_total"], 1);
        assert_eq!(gift_free["entitlements"]["guardian_sponsor"], true);
        let receipt_tok = gift_free["guardian_receipt_token"].as_str().unwrap();
        assert!(receipt_tok.starts_with("qient1."));
        let receipt_claims = verify_and_decode_oset(receipt_tok);
        assert_eq!(receipt_claims.sub, account_c);
        assert_eq!(receipt_claims.tier, "free");
        assert_eq!(receipt_claims.guardian_sponsor, true);

        let gift_free_replay: Value = ureq::post(&format!("{base}/v1/billing/apple/verify"))
            .set("Authorization", &auth_c)
            .send_json(gift_free_req)
            .unwrap()
            .into_json()
            .unwrap();
        assert_eq!(gift_free_replay["idempotent_replay"], true);
        assert_eq!(gift_free_replay["entitlement_token"], "");
        assert!(
            gift_free_replay["guardian_receipt_token"]
                .as_str()
                .unwrap()
                .starts_with("qient1.")
        );

        // 5. Cross-account ownership conflict rejected with 409
        let hijack_err = ureq::post(&format!("{base}/v1/billing/apple/verify"))
            .set("Authorization", &auth_b)
            .send_json(json!({ "jws_representation": sub_jws }))
            .unwrap_err();
        match hijack_err {
            ureq::Error::Status(code, _) => assert_eq!(code, 409),
            other => panic!("expected 409 conflict on cross-account replay, got {other}"),
        }

        // 6. Invalid payloads rejected with 400
        let tampered_eddsa = {
            let mut parts: Vec<String> = renew_jws.split('.').map(ToOwned::to_owned).collect();
            let tampered_claims = URL_SAFE_NO_PAD.encode(
                serde_json::to_vec(&json!({
                    "transactionId": "2000000100099",
                    "originalTransactionId": "2000000100099",
                    "bundleId": "eu.org.cluas.qi",
                    "productId": "com.qi.atelier.family",
                    "expiresDate": extended_sec * 1000
                }))
                .unwrap(),
            );
            parts[1] = tampered_claims;
            parts.join(".")
        };
        let tampered_es256 = {
            let mut parts: Vec<String> = sub_jws.split('.').map(ToOwned::to_owned).collect();
            let tampered_claims = URL_SAFE_NO_PAD.encode(
                serde_json::to_vec(&json!({
                    "transactionId": "2000000100088",
                    "originalTransactionId": "2000000100088",
                    "bundleId": "eu.org.cluas.qi",
                    "productId": "com.qi.atelier.family",
                    "environment": "Sandbox",
                    "expiresDate": future_ms
                }))
                .unwrap(),
            );
            parts[1] = tampered_claims;
            parts.join(".")
        };
        let alg_none_jws = format!(
            "{}.{}.{}",
            URL_SAFE_NO_PAD.encode(br#"{"alg":"none"}"#),
            URL_SAFE_NO_PAD.encode(br#"{"transactionId":"x1","productId":"com.qi.atelier.pro_annual"}"#),
            URL_SAFE_NO_PAD.encode(b"sig")
        );
        let short_sig_jws = format!(
            "{}.{}.{}",
            URL_SAFE_NO_PAD.encode(br#"{"alg":"ES256"}"#),
            URL_SAFE_NO_PAD.encode(br#"{"transactionId":"x2","productId":"com.qi.atelier.pro_annual"}"#),
            URL_SAFE_NO_PAD.encode(b"too_short")
        );
        let fake_64b_es256_jws = make_jws_es256_fake_sig(
            &json!({
                "transactionId": "2000000888000",
                "originalTransactionId": "2000000888000",
                "bundleId": "eu.org.cluas.qi",
                "productId": "com.qi.atelier.pro_annual",
                "environment": "Sandbox",
                "expiresDate": future_ms
            }),
            true,
        );
        let rogue_chain_es256_jws = make_jws_es256_signed_with_chain(
            &json!({
                "transactionId": "2000000888010",
                "originalTransactionId": "2000000888010",
                "bundleId": "eu.org.cluas.qi",
                "productId": "com.qi.atelier.pro_annual",
                "environment": "Sandbox",
                "expiresDate": future_ms
            }),
            TEST_ROGUE_LEAF_PKCS8_DER,
            &[TEST_ROGUE_LEAF_DER],
        );
        let missing_x5c_es256_jws = make_jws_es256_signed_with_chain(
            &json!({
                "transactionId": "2000000888011",
                "originalTransactionId": "2000000888011",
                "bundleId": "eu.org.cluas.qi",
                "productId": "com.qi.atelier.pro_annual",
                "environment": "Sandbox",
                "expiresDate": future_ms
            }),
            TEST_LEAF_PKCS8_DER,
            &[],
        );
        let missing_bundle_jws = make_jws_es256_signed(&json!({
            "transactionId": "2000000888012",
            "originalTransactionId": "2000000888012",
            "productId": "com.qi.atelier.pro_annual",
            "environment": "Sandbox",
            "expiresDate": future_ms
        }));
        let wrong_bundle_jws = make_jws_es256_signed(&json!({
            "transactionId": "2000000888001",
            "originalTransactionId": "2000000888001",
            "bundleId": "com.evil.clone",
            "productId": "com.qi.atelier.pro_annual",
            "environment": "Sandbox",
            "expiresDate": future_ms
        }));
        let unknown_product_jws = make_jws_es256_signed(&json!({
            "transactionId": "2000000888002",
            "originalTransactionId": "2000000888002",
            "bundleId": "eu.org.cluas.qi",
            "productId": "com.qi.unknown_sku",
            "environment": "Sandbox",
            "expiresDate": future_ms
        }));
        let revoked_jws = make_jws_es256_signed(&json!({
            "transactionId": "2000000888003",
            "originalTransactionId": "2000000888003",
            "bundleId": "eu.org.cluas.qi",
            "productId": "com.qi.atelier.pro_annual",
            "environment": "Sandbox",
            "expiresDate": future_ms,
            "revocationDate": Utc::now().timestamp() * 1000
        }));
        let expired_jws = make_jws_es256_signed(&json!({
            "transactionId": "2000000888004",
            "originalTransactionId": "2000000888004",
            "bundleId": "eu.org.cluas.qi",
            "productId": "com.qi.atelier.pro_annual",
            "environment": "Sandbox",
            "expiresDate": (Utc::now().timestamp() - 3600) * 1000
        }));

        let bad_payloads = vec![
            json!({ "jws_representation": "not-three-parts" }),
            json!({ "jws_representation": alg_none_jws }),
            json!({ "jws_representation": short_sig_jws }),
            json!({ "jws_representation": fake_64b_es256_jws }),
            json!({ "jws_representation": tampered_eddsa }),
            json!({ "jws_representation": tampered_es256 }),
            json!({ "jws_representation": rogue_chain_es256_jws }),
            json!({ "jws_representation": missing_x5c_es256_jws }),
            json!({ "jws_representation": missing_bundle_jws }),
            json!({ "jws_representation": wrong_bundle_jws }),
            json!({ "jws_representation": unknown_product_jws }),
            json!({ "jws_representation": revoked_jws }),
            json!({ "jws_representation": expired_jws }),
            json!({ "jws_representation": sub_jws, "product_id": "com.qi.atelier.pro_monthly" }),
            json!({
                "environment": "Production",
                "product_id": "com.qi.atelier.pro_annual",
                "transaction_id": "prod_unsigned_1",
                "original_transaction_id": "prod_unsigned_1"
            }),
            json!({
                "environment": "InvalidEnv",
                "product_id": "com.qi.atelier.pro_annual",
                "transaction_id": "bad_env_1",
                "original_transaction_id": "bad_env_1"
            }),
        ];

        for bad in bad_payloads {
            let err = ureq::post(&format!("{base}/v1/billing/apple/verify"))
                .set("Authorization", &auth_b)
                .send_json(bad.clone())
                .unwrap_err();
            match err {
                ureq::Error::Status(code, _) => assert_eq!(code, 400, "expected 400 for {bad}"),
                other => panic!("expected 400 error for {bad}, got {other}"),
            }
        }

        // 7. Physical account wipe also deletes apple_iap_transactions for account A
        let wipe_a: Value = ureq::delete(&format!("{base}/v1/account"))
            .set("Authorization", &auth_a)
            .call()
            .unwrap()
            .into_json()
            .unwrap();
        assert_eq!(wipe_a["deleted_accounts"], 1);
        assert_eq!(wipe_a["deleted_apple_transactions"], 4);
        assert_eq!(db.count_apple_transactions_for_account(&account_a).unwrap(), 0);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn open_source_security_hardening_auth_takeover_expiry_sync_and_prod_lockdown() {
    // 1. 生产模式密钥门禁与功能矩阵核验
    if std::env::var("QI_CLOUD_SIGNING_SEED_HEX").ok().as_deref().unwrap_or("").is_empty() {
        assert!(ControlPlaneDb::open_memory_with_production_mode(true).is_err());
    }
    assert!(validate_production_signing_key(true, Some(DEFAULT_ATELIER_SIGNING_SEED)).is_err());
    assert!(validate_production_signing_key(true, Some([0x77u8; 32])).is_ok());

    for tier in [
        SubscriptionTier::ProMonthly,
        SubscriptionTier::Seasonal,
        SubscriptionTier::ProAnnual,
        SubscriptionTier::Duo,
        SubscriptionTier::Family,
        SubscriptionTier::LifetimeLocal,
    ] {
        let feats = tier.default_features();
        for pet_feat in [
            "custom_pet_1",
            "custom_pets_3",
            "custom_pet_ex_libris",
            "pet_weekly_letters",
            "co_sanctuary_host",
        ] {
            assert!(
                feats.iter().any(|f| f == pet_feat),
                "{:?} must contain {pet_feat}",
                tier
            );
        }
        assert_eq!(
            feats.iter().any(|f| f == "cloud_e2ee_sync"),
            tier != SubscriptionTier::LifetimeLocal
        );
    }

    // 2. 防顶号会话轮换、订阅过期不延寿 + atelier_synthesis 配额隔离、E2EE 同步门禁与限流
    let temp_db_path = std::env::temp_dir().join(format!(
        "qi_cloud_sec_hardening_{}_{}.sqlite",
        std::process::id(),
        Utc::now().timestamp_nanos_opt().unwrap_or(1)
    ));
    let file_db = ControlPlaneDb::open_file_with_config(
        &temp_db_path,
        false,
        None,
        Some(TEST_ROOT_DER.to_vec()),
    )
    .unwrap();
    assert_eq!(
        file_db.table_names().unwrap(),
        vec![
            "accounts",
            "apple_iap_transactions",
            "e2ee_sync_envelopes",
            "monthly_quotas",
        ]
    );

    let (base_url, _db) = spawn_test_server_with_db(file_db, None).await;
    let db_path_for_test = temp_db_path.clone();

    tokio::task::spawn_blocking(move || {
        // 2a. 防顶号：注册带 auth_proof_hex 的 handle
        let sess1: Value = ureq::post(&format!("{base_url}/v1/auth/session"))
            .send_json(json!({
                "handle": "shield-user@example.com",
                "auth_proof_hex": "deadbeef12345678"
            }))
            .unwrap()
            .into_json()
            .unwrap();
        let account_id = sess1["account_id"].as_str().unwrap().to_owned();
        let tok1 = sess1["token"].as_str().unwrap().to_owned();
        let auth1 = format!("Bearer {tok1}");

        // 无凭据/错误凭据重复请求同一 handle -> 401 handle_already_claimed
        for bad_req in [
            (None, json!({ "handle": "shield-user@example.com" })),
            (
                None,
                json!({ "handle": "shield-user@example.com", "auth_proof_hex": "wrong-proof" }),
            ),
            (
                Some("Bearer qitok_wrong_token"),
                json!({ "handle": "shield-user@example.com" }),
            ),
        ] {
            let mut req = ureq::post(&format!("{base_url}/v1/auth/session"));
            if let Some(h) = bad_req.0 {
                req = req.set("Authorization", h);
            }
            let err = req.send_json(bad_req.1).unwrap_err();
            match err {
                ureq::Error::Status(code, resp) => {
                    assert_eq!(code, 401);
                    let body: Value = resp.into_json().unwrap();
                    assert_eq!(body["error"], "handle_already_claimed");
                }
                other => panic!("expected 401 handle_already_claimed, got {other}"),
            }
        }

        // 持有效 Bearer 轮换会话成功且旧 Token 立即失效
        let sess2: Value = ureq::post(&format!("{base_url}/v1/auth/session"))
            .set("Authorization", &auth1)
            .send_json(json!({ "handle": "shield-user@example.com" }))
            .unwrap()
            .into_json()
            .unwrap();
        let tok2 = sess2["token"].as_str().unwrap().to_owned();
        assert_ne!(tok2, tok1);
        assert!(matches!(
            ureq::get(&format!("{base_url}/v1/entitlements"))
                .set("Authorization", &auth1)
                .call()
                .unwrap_err(),
            ureq::Error::Status(401, _)
        ));

        // 持注册时的 auth_proof_hex 轮换会话同样成功
        let sess3: Value = ureq::post(&format!("{base_url}/v1/auth/session"))
            .send_json(json!({
                "handle": "shield-user@example.com",
                "auth_proof_hex": "deadbeef12345678"
            }))
            .unwrap()
            .into_json()
            .unwrap();
        let tok3 = sess3["token"].as_str().unwrap().to_owned();
        assert_ne!(tok3, tok2);
        let auth3 = format!("Bearer {tok3}");

        // 2b. Free 用户调用 Sync 403；atelier_synthesis 429 且不消耗 briefing 免费额度
        let free_sync_err = ureq::post(&format!("{base_url}/v1/sync/ops/push"))
            .set("Authorization", &auth3)
            .send_json(json!({
                "device_id": "dev-1",
                "batch_seq": 1,
                "path": "ops/dev-1/001.enc",
                "protocol": "open-health-sync-v2",
                "cipher": "XChaCha20-Poly1305",
                "nonce_hex": "a".repeat(48),
                "ciphertext_hex": "deadbeef",
                "payload_bytes": 4
            }))
            .unwrap_err();
        assert!(matches!(free_sync_err, ureq::Error::Status(403, _)));

        let free_synth_err = ureq::post(&format!("{base_url}/v1/zdr/complete"))
            .set("Authorization", &auth3)
            .send_json(json!({
                "task": "atelier_synthesis",
                "scrubbed_prompt": "free synthesis probe"
            }))
            .unwrap_err();
        assert!(matches!(free_synth_err, ureq::Error::Status(429, _)));

        let ent_after_synth: Value = ureq::get(&format!("{base_url}/v1/entitlements"))
            .set("Authorization", &auth3)
            .call()
            .unwrap()
            .into_json()
            .unwrap();
        assert_eq!(ent_after_synth["briefings_used"], 0);

        let brief_ok = ureq::post(&format!("{base_url}/v1/zdr/complete"))
            .set("Authorization", &auth3)
            .send_json(json!({ "task": "briefing", "scrubbed_prompt": "normal briefing" }))
            .unwrap();
        assert_eq!(brief_ok.status(), 200);

        // LifetimeLocal 同样无云端 E2EE 同步权益 (403)
        ureq::post(&format!("{base_url}/v1/billing/mock-upgrade"))
            .set("Authorization", &auth3)
            .send_json(json!({ "tier": "lifetime_local" }))
            .unwrap();
        let lifetime_sync_err = ureq::post(&format!("{base_url}/v1/sync/ops/push"))
            .set("Authorization", &auth3)
            .send_json(json!({
                "device_id": "dev-1",
                "batch_seq": 2,
                "path": "ops/dev-1/002.enc",
                "protocol": "open-health-sync-v2",
                "cipher": "XChaCha20-Poly1305",
                "nonce_hex": "b".repeat(48),
                "ciphertext_hex": "deadbeef",
                "payload_bytes": 4
            }))
            .unwrap_err();
        assert!(matches!(lifetime_sync_err, ureq::Error::Status(403, _)));

        // 2c. ProMonthly 解锁 Sync + 单片 > 256 KiB 返回 413 + 未过期 refresh 不延长到期时间
        let pro_ent: Value = ureq::post(&format!("{base_url}/v1/billing/mock-upgrade"))
            .set("Authorization", &auth3)
            .send_json(json!({ "tier": "pro_monthly" }))
            .unwrap()
            .into_json()
            .unwrap();
        let original_exp = pro_ent["entitlement_expires_epoch"].as_i64().unwrap();
        assert!(original_exp > Utc::now().timestamp());

        let pro_synth_ok = ureq::post(&format!("{base_url}/v1/zdr/complete"))
            .set("Authorization", &auth3)
            .send_json(json!({ "task": "atelier_synthesis", "scrubbed_prompt": "pro synthesis" }))
            .unwrap();
        assert_eq!(pro_synth_ok.status(), 200);

        let oversized_err = ureq::post(&format!("{base_url}/v1/sync/ops/push"))
            .set("Authorization", &auth3)
            .send_json(json!({
                "device_id": "dev-1",
                "batch_seq": 3,
                "path": "ops/dev-1/003.enc",
                "protocol": "open-health-sync-v2",
                "cipher": "XChaCha20-Poly1305",
                "nonce_hex": "c".repeat(48),
                "ciphertext_hex": "aa".repeat(262_145),
                "payload_bytes": 262_145
            }))
            .unwrap_err();
        assert!(matches!(oversized_err, ureq::Error::Status(413, _)));

        let refreshed_valid: Value = ureq::post(&format!("{base_url}/v1/entitlements/refresh"))
            .set("Authorization", &auth3)
            .send_json(json!({}))
            .unwrap()
            .into_json()
            .unwrap();
        assert_eq!(refreshed_valid["plan_tier"], "pro_monthly");
        assert_eq!(refreshed_valid["entitlement_expires_epoch"], original_exp);

        // 模拟订阅过期（将 DB 到期时间改到过去 60 秒）
        {
            let raw_conn = rusqlite::Connection::open(&db_path_for_test).unwrap();
            let past_epoch = Utc::now().timestamp() - 60;
            raw_conn
                .execute(
                    "UPDATE accounts SET entitlement_expires_epoch = ?1 WHERE account_id = ?2",
                    rusqlite::params![past_epoch, account_id],
                )
                .unwrap();
        }

        // 过期后 ZDR atelier_synthesis 立即 429，且不消耗 briefing 额度
        let expired_synth_err = ureq::post(&format!("{base_url}/v1/zdr/complete"))
            .set("Authorization", &auth3)
            .send_json(json!({ "task": "atelier_synthesis", "scrubbed_prompt": "expired synth" }))
            .unwrap_err();
        assert!(matches!(expired_synth_err, ureq::Error::Status(429, _)));

        // /v1/entitlements/refresh 将过期账号降回 free，绝不延寿
        let refreshed_expired: Value = ureq::post(&format!("{base_url}/v1/entitlements/refresh"))
            .set("Authorization", &auth3)
            .send_json(json!({}))
            .unwrap()
            .into_json()
            .unwrap();
        assert_eq!(refreshed_expired["plan_tier"], "free");
        assert_eq!(refreshed_expired["entitlement_expires_epoch"], 0);
        assert_eq!(refreshed_expired["entitlement_token"], "");
    })
    .await
    .unwrap();

    let _ = std::fs::remove_file(&temp_db_path);
    let _ = std::fs::remove_file(format!("{}-wal", temp_db_path.display()));
    let _ = std::fs::remove_file(format!("{}-shm", temp_db_path.display()));

    // 3. 生产模式严防：关闭 mock billing、禁用测试根 CA/Sandbox/EdDSA、收紧 CORS、校验独立私钥签名 OSET
    let prod_seed = [0x77u8; 32];
    let prod_db = ControlPlaneDb::open_memory_with_config(
        true,
        Some(prod_seed),
        Some(TEST_ROOT_DER.to_vec()),
    )
    .unwrap();
    let prod_vk = prod_db.verifying_key();
    assert!(prod_db.is_production_mode());
    let (prod_base, _) = spawn_test_server_with_db(prod_db, Some(5)).await;

    tokio::task::spawn_blocking(move || {
        // 3a. CORS 收紧：不返回 Access-Control-Allow-Origin: *
        let cors_resp = ureq::get(&format!("{prod_base}/healthz"))
            .set("Origin", "https://attacker.example")
            .call()
            .unwrap();
        assert_ne!(
            cors_resp.header("access-control-allow-origin"),
            Some("*")
        );

        let sess: Value = ureq::post(&format!("{prod_base}/v1/auth/session"))
            .send_json(json!({ "handle": "prod-user@example.com" }))
            .unwrap()
            .into_json()
            .unwrap();
        let prod_auth = format!("Bearer {}", sess["token"].as_str().unwrap());

        // 3b. /v1/billing/activate-pro 与 /v1/billing/mock-upgrade 返回 403
        for path in ["/v1/billing/activate-pro", "/v1/billing/mock-upgrade"] {
            let err = ureq::post(&format!("{prod_base}{path}"))
                .set("Authorization", &prod_auth)
                .send_json(json!({ "tier": "pro_annual" }))
                .unwrap_err();
            match err {
                ureq::Error::Status(code, resp) => {
                    assert_eq!(code, 403);
                    let body: Value = resp.into_json().unwrap();
                    assert_eq!(body["error"], "mock_billing_disabled_in_production");
                }
                other => panic!("expected 403 mock disabled on {path}, got {other}"),
            }
        }

        // 3c. 生产模式拒绝 Sandbox/EdDSA/测试自签根 CA/无 x5c 的 Apple IAP 验证请求
        let future_ms = (Utc::now().timestamp() + 30 * 86_400) * 1000;
        let test_root_prod_jws = make_jws_es256_signed(&json!({
            "transactionId": "9000000001",
            "originalTransactionId": "9000000001",
            "bundleId": "eu.org.cluas.qi",
            "productId": "com.qi.atelier.pro_annual",
            "environment": "Production",
            "expiresDate": future_ms
        }));
        let eddsa_jws = make_jws_eddsa_signed(&json!({
            "transactionId": "9000000002",
            "originalTransactionId": "9000000002",
            "bundleId": "eu.org.cluas.qi",
            "productId": "com.qi.atelier.pro_annual",
            "environment": "Sandbox",
            "expiresDate": future_ms
        }));
        for bad_iap in [
            json!({
                "environment": "Sandbox",
                "product_id": "com.qi.atelier.pro_annual",
                "transaction_id": "9000000003",
                "original_transaction_id": "9000000003",
                "bundle_id": "eu.org.cluas.qi"
            }),
            json!({ "jws_representation": eddsa_jws }),
            json!({ "jws_representation": test_root_prod_jws }),
        ] {
            let err = ureq::post(&format!("{prod_base}/v1/billing/apple/verify"))
                .set("Authorization", &prod_auth)
                .send_json(bad_iap)
                .unwrap_err();
            assert!(matches!(err, ureq::Error::Status(400, _)));
        }

        // 验证 prod_db 使用自定义种子而非 DEFAULT_ATELIER_SIGNING_SEED 签发凭证
        let free_gift_claims_preview = qi_cloud::db::mint_oset_token_with_key(
            "acc_prod_check",
            SubscriptionTier::ProAnnual,
            Utc::now().timestamp(),
            Utc::now().timestamp() + 3600,
            false,
            0,
            &ed25519_dalek::SigningKey::from_bytes(&prod_seed),
        )
        .unwrap();
        let decoded = verify_and_decode_oset_with_key(&free_gift_claims_preview, &prod_vk);
        assert_eq!(decoded.sub, "acc_prod_check");

        // 3d. ZDR 限流：限制 5 次/min（由于限流器对 prod_auth 生效，配额足够触发限流前拦截）
        // Free 用户用完 1 次 lab + 3 次 briefing 后若继续发 ZDR，rate_limiter 在第 6 次（>5）返回 429 rate_limit_exceeded
        let mut saw_rate_limit = false;
        for idx in 0..8 {
            match ureq::post(&format!("{prod_base}/v1/zdr/complete"))
                .set("Authorization", &prod_auth)
                .send_json(json!({ "task": "briefing", "scrubbed_prompt": format!("p-{idx}") }))
            {
                Ok(_) => {}
                Err(ureq::Error::Status(429, resp)) => {
                    let body: Value = resp.into_json().unwrap();
                    if body["error"] == "rate_limit_exceeded" {
                        saw_rate_limit = true;
                        break;
                    }
                }
                Err(other) => panic!("unexpected error on zdr rate-limit test: {other}"),
            }
        }
        assert!(saw_rate_limit, "expected 429 rate_limit_exceeded on /v1/zdr/complete");

        // /v1/auth/session 同样触发限流（前已发过 1 次，继续发送直达上限 5 后必出 rate_limit_exceeded）
        let mut saw_auth_rate_limit = false;
        for i in 0..8 {
            match ureq::post(&format!("{prod_base}/v1/auth/session"))
                .send_json(json!({ "handle": format!("rl-{i}@example.com") }))
            {
                Ok(_) => {}
                Err(ureq::Error::Status(429, resp)) => {
                    let body: Value = resp.into_json().unwrap();
                    if body["error"] == "rate_limit_exceeded" {
                        saw_auth_rate_limit = true;
                        break;
                    }
                }
                Err(other) => panic!("unexpected error on auth rate-limit test: {other}"),
            }
        }
        assert!(saw_auth_rate_limit, "expected 429 rate_limit_exceeded on /v1/auth/session");
    })
    .await
    .unwrap();
}
