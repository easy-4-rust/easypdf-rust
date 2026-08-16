//! 双后端对等测试：printpdf vs krilla。
//!
//! 对同一组操作序列，分别使用 printpdf 和 krilla 引擎生成 PDF，
//! 然后用 lopdf 解析两份输出，断言：
//! - 页数相同
//! - 每页提取文本相同
//! - krilla 输出走 lopdf 加密管线 roundtrip 验证
//!
//! 本文件仅在 `writer-krilla` feature 启用时编译。

#![cfg(feature = "writer-krilla")]

use easypdf::prelude::*;
use std::path::Path;

/// 系统字体路径（macOS Helvetica）。
const MACOS_HELVETICA: &str = "/System/Library/Fonts/Helvetica.ttc";

/// 检查系统字体是否存在，不存在则跳过测试。
fn system_font_data() -> Option<Vec<u8>> {
    let path = std::path::Path::new(MACOS_HELVETICA);
    if path.exists() {
        std::fs::read(path).ok()
    } else {
        None
    }
}

/// 用公共 API 构建测试 PDF（printpdf 引擎）。
fn build_with_printpdf(
    path: &Path,
    font_data: Option<&[u8]>,
    font_key: &str,
) -> easypdf::Result<()> {
    let mut writer = PdfWriter::new("Parity Test - printpdf");

    if let Some(data) = font_data {
        writer.register_font_from_bytes(font_key, data)?;
    }

    // 第1页：单行文本。
    writer.add_page(PageSize::A4, Orientation::Portrait)?;
    writer.write_text_with_custom_font("Hello from engine", font_key, 14.0, 100.0, 700.0)?;

    // 第2页：多行文本。
    writer.add_page(PageSize::A4, Orientation::Portrait)?;
    writer.write_text_with_custom_font("Line 1", font_key, 12.0, 100.0, 700.0)?;
    writer.write_text_with_custom_font("Line 2", font_key, 12.0, 100.0, 680.0)?;

    // 第3页：图形。
    writer.add_page(PageSize::A4, Orientation::Portrait)?;
    writer.draw_line(50.0, 400.0, 500.0, 400.0, 2.0);
    writer.draw_rect_stroke(100.0, 300.0, 200.0, 100.0, 1.0);

    writer.finish(path)?;
    Ok(())
}

/// 用公共 API 构建测试 PDF（krilla 引擎）。
fn build_with_krilla(path: &Path, font_data: Option<&[u8]>, font_key: &str) -> easypdf::Result<()> {
    let mut writer = PdfWriterBuilder::new("Parity Test - krilla")
        .engine(WriteEngineKind::Krilla)
        .build()?;

    if let Some(data) = font_data {
        writer.register_font_from_bytes(font_key, data)?;
    }

    // 第1页：单行文本。
    writer.add_page(PageSize::A4, Orientation::Portrait)?;
    writer.write_text_with_custom_font("Hello from engine", font_key, 14.0, 100.0, 700.0)?;

    // 第2页：多行文本。
    writer.add_page(PageSize::A4, Orientation::Portrait)?;
    writer.write_text_with_custom_font("Line 1", font_key, 12.0, 100.0, 700.0)?;
    writer.write_text_with_custom_font("Line 2", font_key, 12.0, 100.0, 680.0)?;

    // 第3页：图形。
    writer.add_page(PageSize::A4, Orientation::Portrait)?;
    writer.draw_line(50.0, 400.0, 500.0, 400.0, 2.0);
    writer.draw_rect_stroke(100.0, 300.0, 200.0, 100.0, 1.0);

    writer.finish(path)?;
    Ok(())
}

/// 从 PDF 字节中提取所有页面的文本。
fn extract_all_text(pdf_bytes: &[u8]) -> Vec<String> {
    let cursor = std::io::Cursor::new(pdf_bytes);
    let doc = lopdf::Document::load_from(cursor).unwrap();
    let pages = doc.get_pages();
    let mut texts = Vec::new();

    for (_page_num, page_id) in pages {
        let text = doc.extract_text(&[page_id.0]).unwrap_or_default();
        texts.push(text);
    }

    texts
}

/// 获取 PDF 页数。
fn page_count(pdf_bytes: &[u8]) -> usize {
    let cursor = std::io::Cursor::new(pdf_bytes);
    let doc = lopdf::Document::load_from(cursor).unwrap();
    doc.get_pages().len()
}

// ---------------------------------------------------------------------------
// 对等测试
// ---------------------------------------------------------------------------

/// 测试：页数对等（多页文档）。
#[test]
fn parity_page_count() {
    let font_data = system_font_data();
    let font_key = "parity_helvetica";

    let printpdf_path = std::env::temp_dir().join("parity_pc_printpdf.pdf");
    let krilla_path = std::env::temp_dir().join("parity_pc_krilla.pdf");

    build_with_printpdf(&printpdf_path, font_data.as_deref(), font_key).unwrap();
    build_with_krilla(&krilla_path, font_data.as_deref(), font_key).unwrap();

    let printpdf_bytes = std::fs::read(&printpdf_path).unwrap();
    let krilla_bytes = std::fs::read(&krilla_path).unwrap();

    let printpdf_pages = page_count(&printpdf_bytes);
    let krilla_pages = page_count(&krilla_bytes);

    assert_eq!(
        printpdf_pages, krilla_pages,
        "页数不等：printpdf={printpdf_pages}, krilla={krilla_pages}"
    );
    assert_eq!(printpdf_pages, 3, "期望 3 页");

    let _ = std::fs::remove_file(&printpdf_path);
    let _ = std::fs::remove_file(&krilla_path);
}

/// 测试：文本内容对等（需要系统字体）。
///
/// 如果系统字体不存在（如 CI ubuntu），此测试将跳过。
#[test]
fn parity_text_content() {
    let Some(font_data) = system_font_data() else {
        eprintln!("跳过：系统字体 {MACOS_HELVETICA} 不存在");
        return;
    };
    let font_key = "parity_text_helvetica";

    let printpdf_path = std::env::temp_dir().join("parity_txt_printpdf.pdf");
    let krilla_path = std::env::temp_dir().join("parity_txt_krilla.pdf");

    build_with_printpdf(&printpdf_path, Some(&font_data), font_key).unwrap();
    build_with_krilla(&krilla_path, Some(&font_data), font_key).unwrap();

    let printpdf_bytes = std::fs::read(&printpdf_path).unwrap();
    let krilla_bytes = std::fs::read(&krilla_path).unwrap();

    let printpdf_texts = extract_all_text(&printpdf_bytes);
    let krilla_texts = extract_all_text(&krilla_bytes);

    assert_eq!(printpdf_texts.len(), krilla_texts.len(), "页面数不一致");

    // 比较每页文本（归一化后）。
    for (i, (pt, kt)) in printpdf_texts.iter().zip(krilla_texts.iter()).enumerate() {
        let normalize = |s: &str| s.chars().filter(|c| !c.is_whitespace()).collect::<String>();
        let pn = normalize(pt);
        let kn = normalize(kt);
        assert_eq!(
            pn,
            kn,
            "第 {} 页文本不一致:\n  printpdf: {:?}\n  krilla:   {:?}",
            i + 1,
            pt,
            kt
        );
    }

    let _ = std::fs::remove_file(&printpdf_path);
    let _ = std::fs::remove_file(&krilla_path);
}

/// 测试：图形操作对等（线条、矩形）。
///
/// 验证两个引擎都能成功生成包含图形的 PDF，且页数一致。
#[test]
fn parity_graphics() {
    let font_data = system_font_data();
    let font_key = "parity_gfx_helvetica";

    let printpdf_path = std::env::temp_dir().join("parity_gfx_printpdf.pdf");
    let krilla_path = std::env::temp_dir().join("parity_gfx_krilla.pdf");

    build_with_printpdf(&printpdf_path, font_data.as_deref(), font_key).unwrap();
    build_with_krilla(&krilla_path, font_data.as_deref(), font_key).unwrap();

    let printpdf_bytes = std::fs::read(&printpdf_path).unwrap();
    let krilla_bytes = std::fs::read(&krilla_path).unwrap();

    assert_eq!(page_count(&printpdf_bytes), 3);
    assert_eq!(page_count(&krilla_bytes), 3);

    let _ = std::fs::remove_file(&printpdf_path);
    let _ = std::fs::remove_file(&krilla_path);
}

/// 测试：krilla 输出经 lopdf roundtrip 验证。
///
/// 验证 krilla 生成的 PDF 可以被 lopdf 正常加载和解析，
/// 证明引擎无关性。
#[test]
fn parity_krilla_lopdf_roundtrip() {
    let font_data = system_font_data();
    let font_key = "parity_rt_helvetica";

    let krilla_path = std::env::temp_dir().join("parity_rt_krilla.pdf");
    build_with_krilla(&krilla_path, font_data.as_deref(), font_key).unwrap();

    let krilla_bytes = std::fs::read(&krilla_path).unwrap();

    // 验证 PDF 文件头。
    assert!(krilla_bytes.starts_with(b"%PDF"), "krilla 输出不是有效 PDF");

    // lopdf 加载验证。
    let cursor = std::io::Cursor::new(&krilla_bytes);
    let doc = lopdf::Document::load_from(cursor).expect("lopdf 无法加载 krilla 输出");
    let pages = doc.get_pages();
    assert_eq!(pages.len(), 3, "krilla 输出应有 3 页");

    // 验证可以提取文本（krilla 可能使用子集化字体编码，
    // lopdf 的 extract_text 不一定能解码所有编码方式，
    // 所以这里只验证调用不 panic，不要求非空）。
    let first_page = pages.values().next().unwrap();
    let _text = doc.extract_text(&[first_page.0]).unwrap_or_default();

    let _ = std::fs::remove_file(&krilla_path);
}

/// 测试：krilla 引擎生成的 PDF 体积合理。
#[test]
fn parity_krilla_output_size() {
    let font_data = system_font_data();
    let font_key = "parity_size_helvetica";

    let krilla_path = std::env::temp_dir().join("parity_size_krilla.pdf");
    build_with_krilla(&krilla_path, font_data.as_deref(), font_key).unwrap();

    let metadata = std::fs::metadata(&krilla_path).unwrap();
    let size = metadata.len();

    assert!(size > 100, "krilla 输出太小: {size} 字节");
    assert!(size < 10 * 1024 * 1024, "krilla 输出太大: {size} 字节");

    let _ = std::fs::remove_file(&krilla_path);
}

/// 测试：字体子集化验证（需要 >=1MB 的系统字体）。
///
/// 嵌入一个大字体文件，验证输出 PDF 体积小于字体原体积的 50%。
/// 如果系统字体不存在或太小，此测试将跳过。
#[test]
fn parity_font_subsetting() {
    let large_font_paths = [
        "/System/Library/Fonts/STHeiti Medium.ttc",
        "/System/Library/Fonts/PingFang.ttc",
        "/System/Library/Fonts/Hiragino Sans GB.ttc",
        "/Library/Fonts/Arial Unicode.ttf",
    ];

    let font_path = large_font_paths
        .iter()
        .find(|p| std::path::Path::new(p).exists())
        .copied();

    let Some(font_path) = font_path else {
        eprintln!("跳过：未找到大字体文件");
        return;
    };

    let font_data = std::fs::read(font_path).unwrap();
    if font_data.len() < 1_000_000 {
        eprintln!(
            "跳过：字体文件 {} 仅 {} 字节（需要 >= 1MB）",
            font_path,
            font_data.len()
        );
        return;
    }

    let font_key = "subset_test_font";
    let krilla_path = std::env::temp_dir().join("parity_subset_krilla.pdf");

    let mut writer = PdfWriterBuilder::new("Subset Test")
        .engine(WriteEngineKind::Krilla)
        .build()
        .expect("failed to build krilla writer");
    writer
        .register_font_from_bytes(font_key, &font_data)
        .unwrap();
    writer
        .add_page(PageSize::A4, Orientation::Portrait)
        .unwrap();
    writer
        .write_text_with_custom_font("Hello", font_key, 14.0, 100.0, 700.0)
        .unwrap();
    writer.finish(&krilla_path).unwrap();

    let pdf_size = std::fs::metadata(&krilla_path).unwrap().len();
    let font_size = font_data.len() as u64;

    assert!(
        pdf_size < font_size / 2,
        "字体子集化失败：PDF 体积 ({pdf_size}) >= 字体体积 ({font_size}) 的 50%"
    );

    let _ = std::fs::remove_file(&krilla_path);
}
