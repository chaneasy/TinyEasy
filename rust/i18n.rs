use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[serde(rename = "zh-CN")]
    Chinese,
    #[default]
    #[serde(rename = "en", other)]
    English,
}
impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::English => "English",
            Self::Chinese => "中文",
        })
    }
}
macro_rules! translations {
    ($( $key:literal => ($en:literal, $zh:literal) ),* $(,)?) => {
        pub const TRANSLATIONS: &[(&str, &str, &str)] = &[$(($key, $en, $zh)),*];
    };
}
translations! {
    "output.title" => ("Output", "输出"),
    "output.compress" => ("Start compression", "开始压缩"),
    "output.quota" => ("{count} compressions left", "可用额度 {count} 次"),
    "stats.original" => ("Original", "压缩前"),
    "stats.result" => ("Compressed", "压缩后"),
    "stats.saved_label" => ("Space saved", "节省空间"),
    "keys.quota" => ("Monthly quota", "本月额度"),
    "preview.original" => ("Original image", "原图预览"),
    "action.import" => ("Import", "导入"),
    "action.images" => ("Images", "图片"),
    "action.folder" => ("Folder", "文件夹"),
    "action.compress" => ("Compress", "压缩"),
    "action.clear" => ("Clear", "清空"),
    "action.retry" => ("Retry", "重试"),
    "action.remove" => ("Remove", "移除"),
    "action.save" => ("Save", "保存"),
    "action.cancel" => ("Cancel", "取消"),
    "action.close" => ("Close", "关闭"),
    "action.show" => ("Show", "查看"),
    "action.hide" => ("Hide", "隐藏"),
    "action.details" => ("Details", "详情"),
    "action.reset" => ("Reset", "重置"),
    "action.zoom_in" => ("Zoom in", "放大"),
    "action.zoom_out" => ("Zoom out", "缩小"),
    "action.previous" => ("Previous", "上一页"),
    "action.next" => ("Next", "下一页"),
    "keys.title" => ("Configuration", "配置"),
    "keys.tab" => ("API Keys", "API Key"),
    "keys.add" => ("Add Key", "添加 Key"),
    "keys.note" => ("Note (optional)", "备注（选填）"),
    "keys.validating" => ("Checking…", "校验中…"),
    "keys.saving" => ("Saving…", "保存中…"),
    "keys.check" => ("Check", "校验"),
    "keys.remaining" => ("{count} remaining", "剩余 {count}"),
    "settings.title" => ("Settings", "设置"),
    "settings.general" => ("General", "常规"),
    "settings.theme" => ("Theme", "主题"),
    "settings.language" => ("Language", "语言"),
    "theme.light" => ("Light", "浅色"),
    "theme.dark" => ("Dark", "深色"),
    "theme.system" => ("System", "跟随系统"),
    "save.overwrite" => ("Overwrite originals", "覆盖原图"),
    "save.overwrite_hint" => ("Replace original files", "压缩后替换原图"),
    "save.directory" => ("Save a copy", "另存到文件夹"),
    "save.directory_hint" => ("Keep the original images", "保留原图，另存压缩图片"),
    "save.choose" => ("Choose folder", "选择目录"),
    "save.change" => ("Change folder", "更改文件夹"),
    "save.open" => ("Open folder", "打开目录"),
    "save.structure" => ("Keep folder structure", "保留文件夹结构"),
    "status.all" => ("All", "全部"),
    "status.pending" => ("Pending", "待处理"),
    "status.processing" => ("Processing", "处理中"),
    "status.done" => ("Done", "完成"),
    "status.failed" => ("Failed", "失败"),
    "status.loading" => ("Loading…", "加载中…"),
    "status.importing" => ("Importing…", "导入中…"),
    "status.progress" => ("Progress", "进度"),
    "stats.processed" => ("{done}/{total} processed", "已处理 {done}/{total}"),
    "stats.saved" => ("Saved {size}", "节省 {size}"),
    "empty.title" => ("Drop images or folders here", "拖入图片或文件夹"),
    "empty.formats" => ("Supports JPG PNG WebP", "支持 JPG PNG WebP"),
    "dialog.images" => ("Import images", "导入图片"),
    "dialog.folder" => ("Import image folder", "导入图片文件夹"),
    "dialog.output" => ("Choose output folder", "选择输出目录"),
    "error.file" => ("Cannot access the file. Check permissions and retry.", "无法读写文件，请检查权限后重试。"),
    "error.image" => ("Cannot decode this image. Check the file and retry.", "无法解码图片，请检查文件后重试。"),
    "error.image_limit" => ("Image exceeds the preview limit. Compression is still available.", "图片超过预览限制，仍可压缩。"),
    "error.network" => ("Connection failed. Check your network and retry.", "连接失败，请检查网络后重试。"),
    "error.invalid_key" => ("Invalid API Key. Update it and check again.", "API Key 无效，请修改后重新校验。"),
    "error.quota" => ("No quota remaining. Add another Key or check next month.", "额度已用完，请添加其他 Key 或下月重新校验。"),
    "error.service" => ("Tinify is unavailable. Retry later.", "Tinify 服务暂不可用，请稍后重试。"),
    "error.unsupported" => ("Unsupported image. Use JPG, PNG or WebP.", "不支持的图片格式，请使用 JPG、PNG 或 WebP。"),
    "error.storage" => ("Secure storage unavailable. Unlock your keychain and retry.", "安全存储不可用，请解锁系统凭据存储后重试。"),
    "error.config" => ("Cannot read settings. Existing data was kept; restore the file to save changes.", "无法读取配置，原数据已保留；请恢复配置文件后再保存。"),
    "error.legacy" => ("Previous Key configuration was kept. Re-enter your API Keys.", "旧 Key 配置已保留，请重新填写 API Key。"),
    "error.key_format" => ("Use a complete alphanumeric API Key (up to 256 characters).", "请填写完整的字母数字 API Key（最多 256 字符）。"),
    "error.duplicate" => ("This API Key is already in the list.", "此 API Key 已在列表中。"),
    "error.key_limit" => ("You can add up to 50 API Keys.", "最多可添加 50 个 API Key。"),
    "error.note_limit" => ("Keep notes within 100 characters.", "备注不能超过 100 个字。"),
    "error.path" => ("Unsafe output path. Choose another folder.", "输出路径不安全，请选择其他目录。"),
    "error.changed" => ("Original changed during compression. It was kept; import it again.", "原图在压缩期间发生变化，已保留原图，请重新导入。"),
}
pub fn tr(language: Language, key: &str) -> &'static str {
    TRANSLATIONS
        .iter()
        .find(|(k, _, _)| *k == key)
        .map(|(_, en, zh)| {
            if language == Language::Chinese {
                *zh
            } else {
                *en
            }
        })
        .unwrap_or("?")
}
pub fn format(language: Language, key: &str, args: &[(&str, String)]) -> String {
    args.iter()
        .fold(tr(language, key).to_owned(), |text, (name, value)| {
            text.replace(&format!("{{{name}}}"), value)
        })
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    #[test]
    fn translations_have_matching_placeholders_and_unique_keys() {
        let placeholders = |s: &str| {
            s.split('{')
                .skip(1)
                .filter_map(|s| s.split_once('}').map(|(p, _)| p.to_owned()))
                .collect::<HashSet<_>>()
        };
        let mut keys = HashSet::new();
        for (key, en, zh) in TRANSLATIONS {
            assert!(keys.insert(key), "duplicate translation {key}");
            assert!(
                !en.is_empty() && !zh.is_empty(),
                "missing translation {key}"
            );
            assert_eq!(placeholders(en), placeholders(zh), "{key}");
        }
    }
    #[test]
    fn unsupported_language_falls_back_to_english() {
        assert_eq!(
            serde_json::from_str::<Language>("\"fr\"").unwrap(),
            Language::English
        );
    }
}
