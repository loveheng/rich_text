//! rich_text_uniffi — 方案 B 绑定层（proc-macro 模式）。
//!
//! 把 [rich_text_core]（唯一事实源）的纯函数暴露给 Kotlin/Swift/Dart。
//! 块树 / 行内 spans 跨 FFI 边界走 **JSON 序列化形态**（评估 native-rewrite-eval.md §4 推荐），
//! 纯文本/标量函数直接返回，零拷贝。
//!
//! 坐标契约：行内 span 坐标为 UTF-8 字节偏移（与核心一致）。

use rich_text_core::*;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

#[uniffi::export]
fn rt_version() -> String {
    version().to_string()
}

/// Markdown 子集 → 块树 JSON（供各端反序列化为本地模型）。
#[uniffi::export]
fn parse_to_json(markdown: String) -> String {
    blocks_to_json(&parse(&markdown))
}

/// 块树 JSON → Markdown 子集串（编辑器回写唯一出口）。
#[uniffi::export]
fn serialize_blocks_from_json(json: String) -> String {
    match blocks_from_json(&json) {
        Ok(blocks) => serialize_blocks(&blocks),
        Err(e) => format!("<!-- rich_text error: {e} -->"),
    }
}

/// 整篇 Markdown → 纯文本（预览/检索）。
#[uniffi::export]
fn rt_markdown_to_plain(markdown: String) -> String {
    markdown_to_plain(&markdown)
}

/// 标题文本 → 纯文本（顶栏标题/列表预览标题）。
#[uniffi::export]
fn rt_title_to_plain(source: String) -> String {
    title_to_plain(&source)
}

/// 行内文本 → 纯文本 + 字节坐标样式 run（JSON：{plain, runs}），供编辑态渲染与选区套格式。
#[uniffi::export]
fn rt_inline_spans_of(source: String) -> String {
    serde_json::to_string(&inline_spans_of(&source)).expect("serialize inline spans")
}

/// 媒体 url → 分类名（audioPlayable / audioDegrade / video / unknown），与核心 [MediaSuffix] 对应。
#[uniffi::export]
fn rt_classify_media_url(url: String) -> String {
    match classify_media_url(&url) {
        MediaSuffix::AudioPlayable => "audioPlayable",
        MediaSuffix::AudioDegrade => "audioDegrade",
        MediaSuffix::Video => "video",
        MediaSuffix::Unknown => "unknown",
    }
    .to_string()
}

/// AI 写入归一结果。
#[derive(uniffi::Record)]
pub struct NormalizeResult {
    pub markdown: String,
    pub notes: Vec<String>,
}

/// AI 写入归一：Markdown 子集外语法（脚注等）语义映射 + R1 降级 note。
#[uniffi::export]
fn rt_normalize_ai_markdown(markdown: String) -> NormalizeResult {
    let r = normalize_ai_markdown(&markdown);
    NormalizeResult {
        markdown: r.markdown,
        notes: r.notes,
    }
}

/// 扫描 Markdown 中的待办项纯文本。
#[uniffi::export]
fn rt_scan_todo_texts(markdown: String) -> Vec<String> {
    scan_todo_texts(&markdown)
}

/// AI 回写护城河：对比原 md 与 AI 产出 md 的行内媒体 url 集合，返回丢失的 url（非空 = 拒绝整替）。
#[uniffi::export]
fn rt_lost_media_urls(original: String, incoming: String) -> Vec<String> {
    lost_media_urls(&original, &incoming)
}

// ---------- 自定义 C-ABI 桥（仅供 Dart 侧 dart:ffi 直接调用，避免依赖 uniffi-dart-bindgen）----------
// 约定：入参为 UTF-8 C 字符串，返回值为 malloc 的 UTF-8 C 字符串，调用方必须用 [rt_free_string] 释放。

/// Markdown 子集 → 块树 JSON（Rust 核心，方案 B 单一事实源）。
#[no_mangle]
pub extern "C" fn rt_parse_to_json(md: *const c_char) -> *mut c_char {
    let md = unsafe { CStr::from_ptr(md) }.to_string_lossy().into_owned();
    let json = blocks_to_json(&parse(&md));
    CString::new(json).expect("rich_text json must not contain NUL").into_raw()
}

/// 块树 JSON → Markdown 子集串（对应 UniFFI `serialize_blocks_from_json`）。
#[no_mangle]
pub extern "C" fn rt_serialize_blocks_from_json(json: *const c_char) -> *mut c_char {
    let json = unsafe { CStr::from_ptr(json) }.to_string_lossy().into_owned();
    let out = match blocks_from_json(&json) {
        Ok(blocks) => serialize_blocks(&blocks),
        Err(e) => format!("<!-- rich_text error: {e} -->"),
    };
    CString::new(out).expect("rich_text output must not contain NUL").into_raw()
}

/// 释放 [rt_parse_to_json] / [rt_serialize_blocks_from_json] 返回的字符串。
#[no_mangle]
pub extern "C" fn rt_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe { drop(CString::from_raw(s)) };
    }
}

uniffi::setup_scaffolding!("rich_text_uniffi");
