// i18n.rs — multi-language support: 中文 / English / 日本語 / 한국어 / हिन्दी
use crate::config::ShapeMode;
use std::collections::HashMap;
use std::sync::LazyLock;

// ═══════════════════════════════════════════
// Lang enum
// ═══════════════════════════════════════════

#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Lang {
    Zh, // 中文
    En, // English
    Ja, // 日本語
    Ko, // 한국어
    Hi, // हिन्दी (Devanagari labels; fallback to transliterated where font support uncertain)
}

impl Lang {
    pub fn all() -> &'static [Lang] {
        &[Lang::Zh, Lang::En, Lang::Ja, Lang::Ko, Lang::Hi]
    }

    /// Human-readable label in its own language
    pub fn label(&self) -> &'static str {
        match self {
            Lang::Zh => "中文",
            Lang::En => "English",
            Lang::Ja => "日本語",
            Lang::Ko => "한국어",
            Lang::Hi => "हिन्दी",
        }
    }
}

impl Default for Lang {
    fn default() -> Self { Lang::Zh }
}

// ═══════════════════════════════════════════
// Translation tables (built once, cheap lookup)
// ═══════════════════════════════════════════

macro_rules! trans_map {
    ($($k:expr => $v:expr),* $(,)?) => {{
        let mut m = HashMap::new();
        $( m.insert($k, $v); )*
        m
    }};
}

static ZH: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| trans_map! {
    // toolbar
    "monitor_label" => "显示器：",
    "shape_label"   => "形状：",
    "grid_label"    => "网格密度：",
    "spacing_label" => "形状稀疏度：",
    "add_btn"       => "+ 添加",
    "del_btn"       => "删除",
    "clear_btn"     => "清空区域",
    "clear_status"  => "已清空所有区域，用形状工具绘制新布局吧",
    "save_btn"      => "保存",
    "save_status"   => "已保存",
    "reset_btn"     => "重置",
    "reset_status"  => "已重置为默认",
    "organize_btn"  => "🚀 整理",
    "organize_msg"  => "✅ 整理完成 — {}/{} 图标 → {}/{} 区域 ({} 父区域 + {} 子区域)",
    "organize_msg_simple" => "✅ 整理完成 — {}/{} 图标 → {}/{} 区域",
    "organize_error"      => "❌ 错误：{}",

    // grid density
    "density_compact"  => "紧凑",
    "density_standard" => "标准",
    "density_loose"    => "宽松",
    "density_wide"     => "超宽",
    "density_status"   => "网格密度: {} (边距={}px)",

    // shapes
    "shape_rect"    => "矩形",
    "shape_ellipse" => "椭圆",
    "shape_star"    => "星形",
    "shape_polygon" => "多边形",

    // editor
    "zone_list_label"         => "区域：",
    "props_label"             => "属性：",
    "name_label"              => "名称：",
    "type_label"              => "匹配类型：",
    "custom_label"            => "自定义：",
    "custom_hint"             => "用逗号分隔，如 编程开发, 游戏",
    "spx_label"               => "间距 宽：",
    "spy_label"               => "高：",
    "pos_x_label"             => "位置 X：",
    "pos_y_label"             => "Y：",
    "size_w_label"            => "宽：",
    "size_h_label"            => "高：",
    "enabled_label"           => "启用",
    "sort_label"              => "排序：",
    "sort_none"               => "无",
    "sort_name"               => "按名称",
    "sort_type"               => "按类型",
    "other_label"             => "收纳未分类",
    "shape_layout_label"      => "排列形状：",
    "text_label"              => "文字：",
    "text_hint"               => "输入文字",
    "child_label"             => "子区域：",
    "add_child_btn"           => "+ 子区域",
    "add_child_status"        => "添加了子区域{} 到 {}",
    "del_child_btn"           => "删除子区域",
    "back_parent_btn"         => "← 返回父区域",
    "preview_label"           => "图标预览：",
    "no_icons"                => "当前屏幕无桌面图标",
    "select_zone_for_preview" => "选择一个区域查看预览",
    "other_collect"           => "将收纳 {} 个未分类图标",
    "match_count"             => "匹配 {} 个图标：",
    "category_type"           => "类型: {}",

    // type presets
    "pt_编程开发" => "编程开发",
    "pt_设计创作" => "设计创作",
    "pt_办公学习" => "办公学习",
    "pt_浏览器"   => "浏览器",
    "pt_社交聊天" => "社交聊天",
    "pt_影音娱乐" => "影音娱乐",
    "pt_游戏"     => "游戏",
    "pt_系统工具" => "系统工具",
    "pt_程序"     => "程序",
    "pt_文档"     => "文档",
    "pt_图片"     => "图片",
    "pt_视频"     => "视频",
    "pt_音频"     => "音频",
    "pt_代码"     => "代码",
    "pt_压缩包"   => "压缩包",
    "pt_快捷方式"  => "快捷方式",
    "pt_文件夹"   => "文件夹",
    "pt_系统图标"  => "系统图标",
    "pt_网络位置"  => "网络位置",

    // monitor naming
    "monitor_main" => "主屏{} {}x{}",
    "monitor_aux"  => "副屏{} {}x{}",

    // shortcuts
    "shortcuts_label" => "启用快捷键",
    "undo_btn"        => "↩ 撤销",
    "redo_btn"        => "↪ 重做",
    "undo_status"     => "已撤销",
    "redo_status"     => "已重做",
    "undo_depth"      => "撤销: {}  重做: {}",
    "key_ctrlz"       => "Ctrl+Z — 撤销 / Ctrl+Y — 重做",
    "key_del"         => "Del — 删除选中区域",
    "key_arrow"       => "方向键 — 微调区域位置",
    "key_f5"          => "F5 — 刷新图标预览",
    "key_f2"          => "F2 — 整理选中显示器",
    "del_zone_status"  => "已删除区域",
    "del_child_status" => "已删除子区域",

    // organize
    "organizing"    => "正在整理 {}/{}: {}…",
    "moved_count"   => "已移动 {} 个图标",
    "preview_fail"  => "预览失败: {}",
    "refresh_status" => "已刷新预览",

    // default naming
    "zone_default"  => "区域{}",
    "child_default" => "子区域{}",

    // language
    "language" => "语言",

    // error messages
    "no_listview"     => "在{}找不到桌面图标列表",
    "no_listview_mon" => "在当前显示器上找不到桌面图标列表\n请确认该显示器已启用'显示桌面图标'",

    // import / export
    "import_btn"   => "📥 导入",
    "export_btn"   => "📤 导出",
    "import_title" => "导入区域配置",
    "export_title" => "导出区域配置",
    "import_status" => "已从 {} 导入配置",
    "export_status" => "已导出配置到 {}",
    "import_error"  => "导入失败：{}",
    "export_error"  => "导出失败：{}",

    // tray
    "tray_btn"        => "🔽 托盘",
    "tray_minimized"  => "已最小化到托盘，检测到新文件自动排列",
    "tray_auto_organized" => "检测到新文件，已自动排列",
    "tray_organized" => "已一键整理完成",
    "already_running_title" => "桌面整理已运行",
    "already_running_msg"   => "桌面整理助手已经在运行中，请检查托盘图标。",

    // zone name translations
    "zone_uncategorized" => "程序(未分类)",
    "zone_other_files"   => "其他文件",
});

// ───── English ─────

static EN: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| trans_map! {
    "monitor_label" => "Monitor:",
    "shape_label"   => "Shape:",
    "grid_label"    => "Grid:",
    "spacing_label" => "Spacing:",
    "add_btn"       => "+ Add",
    "del_btn"       => "Delete",
    "clear_btn"     => "Clear All",
    "clear_status"  => "Cleared all zones. Draw new layout with shape tools.",
    "save_btn"      => "Save",
    "save_status"   => "Saved",
    "reset_btn"     => "Reset",
    "reset_status"  => "Reset to default",
    "organize_btn"  => "🚀 Organize",
    "organize_msg"  => "✅ Organized — {}/{} icons → {}/{} zones ({} parents + {} children)",
    "organize_msg_simple" => "✅ Organized — {}/{} icons → {}/{} zones",
    "organize_error"      => "❌ Error: {}",

    "density_compact"  => "Compact",
    "density_standard" => "Standard",
    "density_loose"    => "Loose",
    "density_wide"     => "Wide",
    "density_status"   => "Grid: {} (padding={}px)",

    "shape_rect"    => "Rectangle",
    "shape_ellipse" => "Ellipse",
    "shape_star"    => "Star",
    "shape_polygon" => "Polygon",

    "zone_list_label"         => "Zones:",
    "props_label"             => "Properties:",
    "name_label"              => "Name:",
    "type_label"              => "Match Type:",
    "custom_label"            => "Custom:",
    "custom_hint"             => "comma-separated, e.g. Game, Music",
    "spx_label"               => "Spacing X:",
    "spy_label"               => "Y:",
    "pos_x_label"             => "Pos X:",
    "pos_y_label"             => "Y:",
    "size_w_label"            => "W:",
    "size_h_label"            => "H:",
    "enabled_label"           => "Enabled",
    "sort_label"              => "Sort:",
    "sort_none"               => "None",
    "sort_name"               => "By Name",
    "sort_type"               => "By Type",
    "other_label"             => "Catch-all",
    "shape_layout_label"      => "Layout:",
    "text_label"              => "Text:",
    "text_hint"               => "Enter text",
    "child_label"             => "Children:",
    "add_child_btn"           => "+ Child",
    "add_child_status"        => "Added child {} to {}",
    "del_child_btn"           => "Delete Child",
    "back_parent_btn"         => "← Back to parent",
    "preview_label"           => "Preview:",
    "no_icons"                => "No desktop icons on this monitor",
    "select_zone_for_preview" => "Select a zone to preview",
    "other_collect"           => "Will collect {} unclassified icons",
    "match_count"             => "Matches {} icons:",
    "category_type"           => "Type: {}",

    "pt_编程开发" => "Development",
    "pt_设计创作" => "Design",
    "pt_办公学习" => "Office",
    "pt_浏览器"   => "Browser",
    "pt_社交聊天" => "Social",
    "pt_影音娱乐" => "Media",
    "pt_游戏"     => "Games",
    "pt_系统工具" => "System",
    "pt_程序"     => "Apps",
    "pt_文档"     => "Documents",
    "pt_图片"     => "Images",
    "pt_视频"     => "Videos",
    "pt_音频"     => "Audio",
    "pt_代码"     => "Code",
    "pt_压缩包"   => "Archives",
    "pt_快捷方式"  => "Shortcuts",
    "pt_文件夹"   => "Folders",
    "pt_系统图标"  => "System Icons",
    "pt_网络位置"  => "Network",

    "monitor_main" => "Main {} {}x{}",
    "monitor_aux"  => "Aux {} {}x{}",

    "shortcuts_label" => "Shortcuts",
    "undo_btn"        => "↩ Undo",
    "redo_btn"        => "↪ Redo",
    "undo_status"     => "Undone",
    "redo_status"     => "Redone",
    "undo_depth"      => "Undo: {}  Redo: {}",
    "key_ctrlz"       => "Ctrl+Z — Undo / Ctrl+Y — Redo",
    "key_del"         => "Del — Delete selected zone",
    "key_arrow"       => "Arrow keys — Nudge zone",
    "key_f5"          => "F5 — Refresh icon preview",
    "key_f2"          => "F2 — Organize selected monitors",
    "del_zone_status"  => "Zone deleted",
    "del_child_status" => "Child deleted",

    "organizing"    => "Organizing {}/{}: {}…",
    "moved_count"   => "Moved {} icons",
    "preview_fail"  => "Preview failed: {}",
    "refresh_status" => "Preview refreshed",

    "zone_default"  => "Zone {}",
    "child_default" => "Child {}",

    "language" => "Language",

    "no_listview"     => "Cannot find desktop icon list on {}",
    "no_listview_mon" => "Cannot find desktop icon list on this monitor.\nMake sure 'Show desktop icons' is enabled.",

    // import / export
    "import_btn"   => "📥 Import",
    "export_btn"   => "📤 Export",
    "import_title" => "Import Zone Config",
    "export_title" => "Export Zone Config",
    "import_status" => "Imported config from {}",
    "export_status" => "Exported config to {}",
    "import_error"  => "Import failed: {}",
    "export_error"  => "Export failed: {}",

    "tray_btn"        => "🔽 Tray",
    "tray_minimized"  => "Minimized to tray. Watching for new icons.",
    "tray_auto_organized" => "New file detected — auto-organized.",
    "tray_organized" => "One-click organize complete.",
    "already_running_title" => "Already Running",
    "already_running_msg"   => "Desktop Organizer is already running. Please check the tray icon.",

    "zone_uncategorized" => "Apps (Uncat.)",
    "zone_other_files"   => "Other Files",
});

// ───── 日本語 (Japanese) ─────

static JA: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| trans_map! {
    "monitor_label" => "モニター：",
    "shape_label"   => "形状：",
    "grid_label"    => "グリッド：",
    "spacing_label" => "間隔：",
    "add_btn"       => "+ 追加",
    "del_btn"       => "削除",
    "clear_btn"     => "全解除",
    "clear_status"  => "すべてのゾーンを解除しました。新しいレイアウトを描画してください。",
    "save_btn"      => "保存",
    "save_status"   => "保存しました",
    "reset_btn"     => "リセット",
    "reset_status"  => "デフォルトにリセットしました",
    "organize_btn"  => "🚀 整理",
    "organize_msg"  => "✅ 整理完了 — {}/{} アイコン → {}/{} ゾーン ({} 親 + {} 子)",
    "organize_msg_simple" => "✅ 整理完了 — {}/{} アイコン → {}/{} ゾーン",
    "organize_error"      => "❌ エラー：{}",

    "density_compact"  => "密集",
    "density_standard" => "標準",
    "density_loose"    => "ゆったり",
    "density_wide"     => "広め",
    "density_status"   => "グリッド: {} (余白={}px)",

    "shape_rect"    => "長方形",
    "shape_ellipse" => "楕円",
    "shape_star"    => "星形",
    "shape_polygon" => "多角形",

    "zone_list_label"         => "ゾーン：",
    "props_label"             => "プロパティ：",
    "name_label"              => "名前：",
    "type_label"              => "マッチタイプ：",
    "custom_label"            => "カスタム：",
    "custom_hint"             => "カンマ区切り、例: ゲーム, 音楽",
    "spx_label"               => "間隔 X：",
    "spy_label"               => "Y：",
    "pos_x_label"             => "位置 X：",
    "pos_y_label"             => "Y：",
    "size_w_label"            => "幅：",
    "size_h_label"            => "高さ：",
    "enabled_label"           => "有効",
    "sort_label"              => "並び替え：",
    "sort_none"               => "なし",
    "sort_name"               => "名前順",
    "sort_type"               => "種類順",
    "other_label"             => "その他",
    "shape_layout_label"      => "レイアウト：",
    "text_label"              => "テキスト：",
    "text_hint"               => "テキストを入力",
    "child_label"             => "子ゾーン：",
    "add_child_btn"           => "+ 子ゾーン",
    "add_child_status"        => "子ゾーン{}を{}に追加しました",
    "del_child_btn"           => "子ゾーンを削除",
    "back_parent_btn"         => "← 親に戻る",
    "preview_label"           => "プレビュー：",
    "no_icons"                => "このモニターにアイコンがありません",
    "select_zone_for_preview" => "ゾーンを選択してプレビューを表示",
    "other_collect"           => "{} 個の未分類アイコンを収納します",
    "match_count"             => "{} 個のアイコンがマッチ：",
    "category_type"           => "種類: {}",

    "pt_编程开发" => "開発",
    "pt_设计创作" => "デザイン",
    "pt_办公学习" => "オフィス",
    "pt_浏览器"   => "ブラウザ",
    "pt_社交聊天" => "SNS",
    "pt_影音娱乐" => "メディア",
    "pt_游戏"     => "ゲーム",
    "pt_系统工具" => "システム",
    "pt_程序"     => "アプリ",
    "pt_文档"     => "文書",
    "pt_图片"     => "画像",
    "pt_视频"     => "動画",
    "pt_音频"     => "音声",
    "pt_代码"     => "コード",
    "pt_压缩包"   => "圧縮",
    "pt_快捷方式"  => "ショートカット",
    "pt_文件夹"   => "フォルダ",
    "pt_系统图标"  => "システムアイコン",
    "pt_网络位置"  => "ネットワーク",

    "monitor_main" => "メイン{} {}x{}",
    "monitor_aux"  => "サブ{} {}x{}",

    "shortcuts_label" => "ショートカット",
    "undo_btn"        => "↩ 元に戻す",
    "redo_btn"        => "↪ やり直し",
    "undo_status"     => "元に戻しました",
    "redo_status"     => "やり直しました",
    "undo_depth"      => "元に戻す: {}  やり直し: {}",
    "key_ctrlz"       => "Ctrl+Z — 元に戻す / Ctrl+Y — やり直し",
    "key_del"         => "Del — 選択ゾーンを削除",
    "key_arrow"       => "矢印キー — ゾーンを微調整",
    "key_f5"          => "F5 — プレビューを更新",
    "key_f2"          => "F2 — 選択モニターを整理",
    "del_zone_status"  => "ゾーンを削除しました",
    "del_child_status" => "子ゾーンを削除しました",

    "organizing"    => "整理中 {}/{}: {}…",
    "moved_count"   => "{} 個のアイコンを移動しました",
    "preview_fail"  => "プレビュー失敗: {}",
    "refresh_status" => "プレビューを更新しました",

    "zone_default"  => "ゾーン{}",
    "child_default" => "子ゾーン{}",

    "language" => "言語",

    "no_listview"     => "{}にデスクトップアイコンリストが見つかりません",
    "no_listview_mon" => "このモニターにデスクトップアイコンリストが見つかりません\n「デスクトップアイコンの表示」が有効か確認してください",

    "import_btn"   => "📥 インポート",
    "export_btn"   => "📤 エクスポート",
    "import_title" => "ゾーン設定をインポート",
    "export_title" => "ゾーン設定をエクスポート",
    "import_status" => "{} から設定をインポートしました",
    "export_status" => "設定を {} にエクスポートしました",
    "import_error"  => "インポート失敗：{}",
    "export_error"  => "エクスポート失敗：{}",

    "tray_btn"        => "🔽 トレイ",
    "tray_minimized"  => "トレイに最小化。新しいファイルを監視中。",
    "tray_auto_organized" => "新しいファイルを検出 — 自動整理しました。",
    "tray_organized" => "ワンクリック整理が完了しました。",
    "already_running_title" => "起動済み",
    "already_running_msg"   => "デスクトップ整理はすでに起動中です。トレイアイコンを確認してください。",

    "zone_uncategorized" => "アプリ（未分類）",
    "zone_other_files"   => "その他のファイル",
});

// ───── 한국어 (Korean) ─────

static KO: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| trans_map! {
    "monitor_label" => "모니터：",
    "shape_label"   => "모양：",
    "grid_label"    => "격자：",
    "spacing_label" => "간격：",
    "add_btn"       => "+ 추가",
    "del_btn"       => "삭제",
    "clear_btn"     => "전체 해제",
    "clear_status"  => "모든 영역을 해제했습니다. 새 레이아웃을 그려주세요.",
    "save_btn"      => "저장",
    "save_status"   => "저장됨",
    "reset_btn"     => "초기화",
    "reset_status"  => "기본값으로 초기화됨",
    "organize_btn"  => "🚀 정리",
    "organize_msg"  => "✅ 정리 완료 — {}/{} 아이콘 → {}/{} 영역 ({} 상위 + {} 하위)",
    "organize_msg_simple" => "✅ 정리 완료 — {}/{} 아이콘 → {}/{} 영역",
    "organize_error"      => "❌ 오류：{}",

    "density_compact"  => "좁게",
    "density_standard" => "표준",
    "density_loose"    => "넓게",
    "density_wide"     => "아주 넓게",
    "density_status"   => "격자: {} (여백={}px)",

    "shape_rect"    => "사각형",
    "shape_ellipse" => "타원",
    "shape_star"    => "별",
    "shape_polygon" => "다각형",

    "zone_list_label"         => "영역：",
    "props_label"             => "속성：",
    "name_label"              => "이름：",
    "type_label"              => "일치 유형：",
    "custom_label"            => "사용자：",
    "custom_hint"             => "쉼표로 구분, 예: 게임, 음악",
    "spx_label"               => "간격 X：",
    "spy_label"               => "Y：",
    "pos_x_label"             => "위치 X：",
    "pos_y_label"             => "Y：",
    "size_w_label"            => "너비：",
    "size_h_label"            => "높이：",
    "enabled_label"           => "활성화",
    "sort_label"              => "정렬：",
    "sort_none"               => "없음",
    "sort_name"               => "이름순",
    "sort_type"               => "유형순",
    "other_label"             => "기타",
    "shape_layout_label"      => "레이아웃：",
    "text_label"              => "텍스트：",
    "text_hint"               => "텍스트 입력",
    "child_label"             => "하위 영역：",
    "add_child_btn"           => "+ 하위",
    "add_child_status"        => "하위 영역{}을 {}에 추가했습니다",
    "del_child_btn"           => "하위 영역 삭제",
    "back_parent_btn"         => "← 상위로 돌아가기",
    "preview_label"           => "미리보기：",
    "no_icons"                => "이 모니터에 아이콘이 없습니다",
    "select_zone_for_preview" => "미리보기할 영역을 선택하세요",
    "other_collect"           => "{} 개의 미분류 아이콘을 수집합니다",
    "match_count"             => "{} 개의 아이콘이 일치：",
    "category_type"           => "유형: {}",

    "pt_编程开发" => "개발",
    "pt_设计创作" => "디자인",
    "pt_办公学习" => "오피스",
    "pt_浏览器"   => "브라우저",
    "pt_社交聊天" => "SNS",
    "pt_影音娱乐" => "미디어",
    "pt_游戏"     => "게임",
    "pt_系统工具" => "시스템",
    "pt_程序"     => "앱",
    "pt_文档"     => "문서",
    "pt_图片"     => "이미지",
    "pt_视频"     => "동영상",
    "pt_音频"     => "오디오",
    "pt_代码"     => "코드",
    "pt_压缩包"   => "압축",
    "pt_快捷方式"  => "바로가기",
    "pt_文件夹"   => "폴더",
    "pt_系统图标"  => "시스템 아이콘",
    "pt_网络位置"  => "네트워크",

    "monitor_main" => "메인{} {}x{}",
    "monitor_aux"  => "보조{} {}x{}",

    "shortcuts_label" => "단축키",
    "undo_btn"        => "↩ 실행 취소",
    "redo_btn"        => "↪ 다시 실행",
    "undo_status"     => "실행 취소됨",
    "redo_status"     => "다시 실행됨",
    "undo_depth"      => "취소: {}  다시 실행: {}",
    "key_ctrlz"       => "Ctrl+Z — 실행 취소 / Ctrl+Y — 다시 실행",
    "key_del"         => "Del — 선택 영역 삭제",
    "key_arrow"       => "방향키 — 영역 미세 조정",
    "key_f5"          => "F5 — 미리보기 새로고침",
    "key_f2"          => "F2 — 선택 모니터 정리",
    "del_zone_status"  => "영역 삭제됨",
    "del_child_status" => "하위 영역 삭제됨",

    "organizing"    => "정리 중 {}/{}: {}…",
    "moved_count"   => "{} 개의 아이콘 이동됨",
    "preview_fail"  => "미리보기 실패: {}",
    "refresh_status" => "미리보기 새로고침됨",

    "zone_default"  => "영역{}",
    "child_default" => "하위{}",

    "language" => "언어",

    "no_listview"     => "{}에서 데스크톱 아이콘 목록을 찾을 수 없습니다",
    "no_listview_mon" => "이 모니터에서 데스크톱 아이콘 목록을 찾을 수 없습니다\n'바탕 화면 아이콘 표시'가 활성화되어 있는지 확인하세요",

    "import_btn"   => "📥 가져오기",
    "export_btn"   => "📤 내보내기",
    "import_title" => "영역 설정 가져오기",
    "export_title" => "영역 설정 내보내기",
    "import_status" => "{} 에서 설정을 가져왔습니다",
    "export_status" => "설정을 {} 로 내보냈습니다",
    "import_error"  => "가져오기 실패：{}",
    "export_error"  => "내보내기 실패：{}",

    "tray_btn"        => "🔽 트레이",
    "tray_minimized"  => "트레이로 최소화. 새 파일 감시 중.",
    "tray_auto_organized" => "새 파일 감지 — 자동 정리 완료.",
    "tray_organized" => "원클릭 정리 완료.",
    "already_running_title" => "이미 실행 중",
    "already_running_msg"   => "데스크톱 정리 도우미가 이미 실행 중입니다. 트레이 아이콘을 확인해 주세요.",

    "zone_uncategorized" => "앱 (미분류)",
    "zone_other_files"   => "기타 파일",
});

// ───── हिन्दी (Hindi) ─────
// Note: Devanagari script; CJK-capable fonts may not render all glyphs.
// We use ISO 15919 transliteration equivalents as fallback where needed.

static HI: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| trans_map! {
    "monitor_label" => "मॉनिटर：",
    "shape_label"   => "आकार：",
    "grid_label"    => "ग्रिड：",
    "spacing_label" => "अंतर：",
    "add_btn"       => "+ जोड़ें",
    "del_btn"       => "हटाएं",
    "clear_btn"     => "सब हटाएं",
    "clear_status"  => "सभी क्षेत्र हटा दिए गए। नया लेआउट बनाएं।",
    "save_btn"      => "सहेजें",
    "save_status"   => "सहेजा गया",
    "reset_btn"     => "रीसेट",
    "reset_status"  => "डिफ़ॉल्ट पर रीसेट",
    "organize_btn"  => "🚀 व्यवस्थित करें",
    "organize_msg"  => "✅ व्यवस्थित — {}/{} आइकन → {}/{} क्षेत्र ({} मूल + {} उप)",
    "organize_msg_simple" => "✅ व्यवस्थित — {}/{} आइकन → {}/{} क्षेत्र",
    "organize_error"      => "❌ त्रुटि：{}",

    "density_compact"  => "सघन",
    "density_standard" => "मानक",
    "density_loose"    => "ढीला",
    "density_wide"     => "चौड़ा",
    "density_status"   => "ग्रिड: {} (मार्जिन={}px)",

    "shape_rect"    => "आयत",
    "shape_ellipse" => "दीर्घवृत्त",
    "shape_star"    => "तारा",
    "shape_polygon" => "बहुभुज",

    "zone_list_label"         => "क्षेत्र：",
    "props_label"             => "गुण：",
    "name_label"              => "नाम：",
    "type_label"              => "मिलान प्रकार：",
    "custom_label"            => "कस्टम：",
    "custom_hint"             => "कॉमा से अलग, जैसे: गेम, संगीत",
    "spx_label"               => "अंतर X：",
    "spy_label"               => "Y：",
    "pos_x_label"             => "स्थिति X：",
    "pos_y_label"             => "Y：",
    "size_w_label"            => "चौड़ाई：",
    "size_h_label"            => "ऊंचाई：",
    "enabled_label"           => "सक्षम",
    "sort_label"              => "क्रम：",
    "sort_none"               => "कोई नहीं",
    "sort_name"               => "नाम से",
    "sort_type"               => "प्रकार से",
    "other_label"             => "अन्य",
    "shape_layout_label"      => "लेआउट：",
    "text_label"              => "पाठ：",
    "text_hint"               => "पाठ दर्ज करें",
    "child_label"             => "उप-क्षेत्र：",
    "add_child_btn"           => "+ उप",
    "add_child_status"        => "उप-क्षेत्र {} को {} में जोड़ा",
    "del_child_btn"           => "उप-क्षेत्र हटाएं",
    "back_parent_btn"         => "← मूल पर वापस",
    "preview_label"           => "पूर्वावलोकन：",
    "no_icons"                => "इस मॉनिटर पर कोई आइकन नहीं",
    "select_zone_for_preview" => "पूर्वावलोकन के लिए क्षेत्र चुनें",
    "other_collect"           => "{} अवर्गीकृत आइकन एकत्र करेंगे",
    "match_count"             => "{} आइकन मेल खाते हैं：",
    "category_type"           => "प्रकार: {}",

    "pt_编程开发" => "विकास",
    "pt_设计创作" => "डिज़ाइन",
    "pt_办公学习" => "कार्यालय",
    "pt_浏览器"   => "ब्राउज़र",
    "pt_社交聊天" => "सामाजिक",
    "pt_影音娱乐" => "मीडिया",
    "pt_游戏"     => "गेम",
    "pt_系统工具" => "सिस्टम",
    "pt_程序"     => "ऐप्स",
    "pt_文档"     => "दस्तावेज़",
    "pt_图片"     => "चित्र",
    "pt_视频"     => "वीडियो",
    "pt_音频"     => "ऑडियो",
    "pt_代码"     => "कोड",
    "pt_压缩包"   => "संग्रह",
    "pt_快捷方式"  => "शॉर्टकट",
    "pt_文件夹"   => "फ़ोल्डर",
    "pt_系统图标"  => "सिस्टम आइकन",
    "pt_网络位置"  => "नेटवर्क",

    "monitor_main" => "मुख्य{} {}x{}",
    "monitor_aux"  => "सहायक{} {}x{}",

    "shortcuts_label" => "शॉर्टकट",
    "undo_btn"        => "↩ पूर्ववत",
    "redo_btn"        => "↪ पुनः करें",
    "undo_status"     => "पूर्ववत किया",
    "redo_status"     => "पुनः किया",
    "undo_depth"      => "पूर्ववत: {}  पुनः: {}",
    "key_ctrlz"       => "Ctrl+Z — पूर्ववत / Ctrl+Y — पुनः करें",
    "key_del"         => "Del — चयनित क्षेत्र हटाएं",
    "key_arrow"       => "तीर — क्षेत्र को समायोजित करें",
    "key_f5"          => "F5 — पूर्वावलोकन ताज़ा करें",
    "key_f2"          => "F2 — चयनित मॉनिटर व्यवस्थित करें",
    "del_zone_status"  => "क्षेत्र हटाया गया",
    "del_child_status" => "उप-क्षेत्र हटाया गया",

    "organizing"    => "व्यवस्थित हो रहा है {}/{}: {}…",
    "moved_count"   => "{} आइकन स्थानांतरित",
    "preview_fail"  => "पूर्वावलोकन विफल: {}",
    "refresh_status" => "पूर्वावलोकन ताज़ा किया",

    "zone_default"  => "क्षेत्र{}",
    "child_default" => "उप{}",

    "language" => "भाषा",

    "no_listview"     => "{} पर डेस्कटॉप आइकन सूची नहीं मिली",
    "no_listview_mon" => "इस मॉनिटर पर डेस्कटॉप आइकन सूची नहीं मिली\nकृपया 'डेस्कटॉप आइकन दिखाएं' सक्षम करें",

    "import_btn"   => "📥 आयात",
    "export_btn"   => "📤 निर्यात",
    "import_title" => "क्षेत्र कॉन्फ़िग आयात करें",
    "export_title" => "क्षेत्र कॉन्फ़िग निर्यात करें",
    "import_status" => "{} से कॉन्फ़िग आयात किया",
    "export_status" => "कॉन्फ़िग को {} में निर्यात किया",
    "import_error"  => "आयात विफल：{}",
    "export_error"  => "निर्यात विफल：{}",

    "tray_btn"        => "🔽 ट्रे",
    "tray_minimized"  => "ट्रे में छोटा किया। नई फ़ाइलों की निगरानी।",
    "tray_auto_organized" => "नई फ़ाइल का पता चला — ऑटो-व्यवस्थित।",
    "tray_organized" => "एक-क्लिक व्यवस्थित पूर्ण।",
    "already_running_title" => "पहले से चल रहा है",
    "already_running_msg"   => "डेस्कटॉप व्यवस्थापक पहले से चल रहा है। कृपया ट्रे आइकन देखें।",

    "zone_uncategorized" => "ऐप्स (अवर्गीकृत)",
    "zone_other_files"   => "अन्य फ़ाइलें",
});

// ═══════════════════════════════════════════
// Public API
// ═══════════════════════════════════════════

/// Look up a translation by key. Returns the key itself as fallback.
pub fn t(lang: Lang, key: &str) -> &str {
    let map = match lang {
        Lang::Zh => &*ZH,
        Lang::En => &*EN,
        Lang::Ja => &*JA,
        Lang::Ko => &*KO,
        Lang::Hi => &*HI,
    };
    map.get(key).copied().unwrap_or(key)
}

/// Translate a type-preset name (stored as Chinese in FILE_TYPE_PRESETS)
pub fn preset_name(lang: Lang, chinese_name: &str) -> &str {
    let key = match chinese_name {
        "编程开发" => "pt_编程开发",
        "设计创作" => "pt_设计创作",
        "办公学习" => "pt_办公学习",
        "浏览器"   => "pt_浏览器",
        "社交聊天" => "pt_社交聊天",
        "影音娱乐" => "pt_影音娱乐",
        "游戏"     => "pt_游戏",
        "系统工具" => "pt_系统工具",
        "程序"     => "pt_程序",
        "文档"     => "pt_文档",
        "图片"     => "pt_图片",
        "视频"     => "pt_视频",
        "音频"     => "pt_音频",
        "代码"     => "pt_代码",
        "压缩包"   => "pt_压缩包",
        "快捷方式"  => "pt_快捷方式",
        "文件夹"   => "pt_文件夹",
        "系统图标"  => "pt_系统图标",
        "网络位置"  => "pt_网络位置",
        _ => return chinese_name,
    };
    t(lang, key)
}

/// Translate shape mode to localized label
pub fn shape_mode_label(lang: Lang, mode: &ShapeMode) -> &'static str {
    match lang {
        Lang::Zh => match mode {
            ShapeMode::Rectangle => "矩形网格",
            ShapeMode::Heart => "爱心 ❤",
            ShapeMode::Circle => "圆形",
            ShapeMode::Triangle => "三角形",
            ShapeMode::Diamond => "菱形",
            ShapeMode::Star => "星形",
            ShapeMode::Spiral => "螺旋",
            ShapeMode::Sine => "正弦曲线",
            ShapeMode::VShape => "V 形",
            ShapeMode::Arc => "弧线",
            ShapeMode::Cross => "十字",
            ShapeMode::Text => "文字",
        },
        Lang::En => match mode {
            ShapeMode::Rectangle => "Grid",
            ShapeMode::Heart => "Heart ❤",
            ShapeMode::Circle => "Circle",
            ShapeMode::Triangle => "Triangle",
            ShapeMode::Diamond => "Diamond",
            ShapeMode::Star => "Star",
            ShapeMode::Spiral => "Spiral",
            ShapeMode::Sine => "Sine Wave",
            ShapeMode::VShape => "V Shape",
            ShapeMode::Arc => "Arc",
            ShapeMode::Cross => "Cross",
            ShapeMode::Text => "Text",
        },
        Lang::Ja => match mode {
            ShapeMode::Rectangle => "グリッド",
            ShapeMode::Heart => "ハート ❤",
            ShapeMode::Circle => "円形",
            ShapeMode::Triangle => "三角形",
            ShapeMode::Diamond => "菱形",
            ShapeMode::Star => "星形",
            ShapeMode::Spiral => "螺旋",
            ShapeMode::Sine => "正弦波",
            ShapeMode::VShape => "V字",
            ShapeMode::Arc => "弧",
            ShapeMode::Cross => "十字",
            ShapeMode::Text => "テキスト",
        },
        Lang::Ko => match mode {
            ShapeMode::Rectangle => "격자",
            ShapeMode::Heart => "하트 ❤",
            ShapeMode::Circle => "원형",
            ShapeMode::Triangle => "삼각형",
            ShapeMode::Diamond => "다이아몬드",
            ShapeMode::Star => "별형",
            ShapeMode::Spiral => "나선",
            ShapeMode::Sine => "사인파",
            ShapeMode::VShape => "V자",
            ShapeMode::Arc => "호",
            ShapeMode::Cross => "십자",
            ShapeMode::Text => "텍스트",
        },
        Lang::Hi => match mode {
            ShapeMode::Rectangle => "ग्रिड",
            ShapeMode::Heart => "दिल ❤",
            ShapeMode::Circle => "गोल",
            ShapeMode::Triangle => "त्रिभुज",
            ShapeMode::Diamond => "हीरा",
            ShapeMode::Star => "तारा",
            ShapeMode::Spiral => "सर्पिल",
            ShapeMode::Sine => "साइन तरंग",
            ShapeMode::VShape => "V आकार",
            ShapeMode::Arc => "चाप",
            ShapeMode::Cross => "क्रॉस",
            ShapeMode::Text => "पाठ",
        },
    }
}

/// Translate a default zone name for display (handles preset names stored as Chinese keys)
pub fn translate_zone_name(name: &str, lang: Lang) -> String {
    match name {
        "浏览器"       => preset_name(lang, "浏览器").to_string(),
        "编程开发"     => preset_name(lang, "编程开发").to_string(),
        "办公学习"     => preset_name(lang, "办公学习").to_string(),
        "社交聊天"     => preset_name(lang, "社交聊天").to_string(),
        "文件夹"       => preset_name(lang, "文件夹").to_string(),
        "影音娱乐"     => preset_name(lang, "影音娱乐").to_string(),
        "游戏"         => preset_name(lang, "游戏").to_string(),
        "系统工具"     => preset_name(lang, "系统工具").to_string(),
        "程序(未分类)" => t(lang, "zone_uncategorized").to_string(),
        "系统图标"     => preset_name(lang, "系统图标").to_string(),
        "网络位置"     => preset_name(lang, "网络位置").to_string(),
        "其他文件"     => t(lang, "zone_other_files").to_string(),
        _ => name.to_string(),
    }
}
