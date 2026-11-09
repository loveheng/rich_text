//! 媒体 url 后缀分类（rich-text-media.md §2 白名单两档）。

use crate::ast::MediaSuffix;

const AUDIO_PLAYABLE: &[&str] = &[".mp3", ".m4a", ".aac", ".wav", ".opus"];
const AUDIO_DEGRADE: &[&str] = &[".amr"];
const VIDEO: &[&str] = &[".mp4", ".mov", ".webm", ".m3u8"];

/// url → 后缀归类。取 path（天然剥离 query 与 fragment）、toLowerCase 后与白名单比对；
/// parse 与呈现层共用此单一事实源。
pub fn classify_media_url(url: &str) -> MediaSuffix {
    let path = url_path(url);
    match path.rfind('.') {
        Some(d) if d < path.len() - 1 => {
            let ext = path[d..].to_lowercase();
            if AUDIO_PLAYABLE.contains(&ext.as_str()) {
                MediaSuffix::AudioPlayable
            } else if AUDIO_DEGRADE.contains(&ext.as_str()) {
                MediaSuffix::AudioDegrade
            } else if VIDEO.contains(&ext.as_str()) {
                MediaSuffix::Video
            } else {
                MediaSuffix::Unknown
            }
        }
        _ => MediaSuffix::Unknown,
    }
}

/// 取 url 的 path 末段（最后一个 `/` 之后），等价 Dart `Uri.parse(url).path` 的扩展名提取口径。
fn url_path(url: &str) -> String {
    let s = match url.find('#') {
        Some(i) => &url[..i],
        None => url,
    };
    let s = match s.find('?') {
        Some(i) => &s[..i],
        None => s,
    };
    match s.rfind('/') {
        Some(i) => s[i + 1..].to_string(),
        None => s.to_string(),
    }
}
