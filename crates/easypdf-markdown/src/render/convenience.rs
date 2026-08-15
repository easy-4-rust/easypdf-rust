//! 便捷渲染函数：单页 PNG、全页导出、内存渲染。

use std::path::{Path, PathBuf};

use super::backend::RenderBackend;
use super::config::{ImageFormat, RenderConfig};
use super::error::{RenderError, Result};
use super::traits::RenderedImage;

/// 将 PDF 单页渲染为 PNG 文件。
///
/// 使用默认后端（文本回退）按指定 DPI 渲染。
///
/// # Errors
///
/// 当 PDF 无法打开、页码无效或输出文件无法写入时返回 [`RenderError`]。
///
/// # Examples
///
/// ```no_run
/// use easypdf_markdown::render::render_page_to_png;
///
/// render_page_to_png("input.pdf".as_ref(), 0, "page_0.png".as_ref(), 150)?;
/// # Ok::<(), easypdf_markdown::render::RenderError>(())
/// ```
pub fn render_page_to_png(
    pdf_path: &Path,
    page_index: usize,
    output: &Path,
    dpi: u32,
) -> Result<()> {
    let config = RenderConfig {
        dpi,
        format: ImageFormat::Png,
        ..RenderConfig::default()
    };
    let renderer = RenderBackend::default_backend().build_renderer(pdf_path)?;
    renderer.render_page_to_path(page_index, &config, output)
}

/// 将 PDF 全部页面渲染为目录下的 PNG 文件。
///
/// 输出文件命名为 `page_000.png`、`page_001.png` 等。
/// 若输出目录不存在则自动创建。
///
/// # Errors
///
/// 当 PDF 无法打开、页面渲染失败或输出目录/文件无法写入时返回 [`RenderError`]。
///
/// # Examples
///
/// ```no_run
/// use easypdf_markdown::render::render_all_pages_to_dir;
///
/// let paths = render_all_pages_to_dir("input.pdf".as_ref(), "output/".as_ref(), 150)?;
/// for p in &paths {
///     println!("rendered: {}", p.display());
/// }
/// # Ok::<(), easypdf_markdown::render::RenderError>(())
/// ```
pub fn render_all_pages_to_dir(
    pdf_path: &Path,
    output_dir: &Path,
    dpi: u32,
) -> Result<Vec<PathBuf>> {
    std::fs::create_dir_all(output_dir)?;

    let config = RenderConfig {
        dpi,
        format: ImageFormat::Png,
        ..RenderConfig::default()
    };
    let renderer = RenderBackend::default_backend().build_renderer(pdf_path)?;

    // 通过逐页探测确定总页数。
    let mut page_count = 0usize;
    loop {
        match renderer.render_page(page_count, &config) {
            Ok(_) => page_count += 1,
            Err(RenderError::InvalidPage { .. }) => break,
            Err(e) => return Err(e),
        }
    }

    // 重新渲染并保存（上面的探测已消耗图像）。
    let mut paths = Vec::with_capacity(page_count);
    for i in 0..page_count {
        let filename = format!("page_{i:03}.png");
        let path = output_dir.join(&filename);
        renderer.render_page_to_path(i, &config, &path)?;
        paths.push(path);
    }
    Ok(paths)
}

/// 将 PDF 单页渲染为内存中的 [`RenderedImage`]。
///
/// 使用默认后端按指定配置渲染。
///
/// # Errors
///
/// 当 PDF 无法打开或页码无效时返回 [`RenderError`]。
pub fn render_page(
    pdf_path: &Path,
    page_index: usize,
    config: &RenderConfig,
) -> Result<RenderedImage> {
    let renderer = RenderBackend::default_backend().build_renderer(pdf_path)?;
    renderer.render_page(page_index, config)
}
