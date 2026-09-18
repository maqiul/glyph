//! PDF 操作模块（基于 lopdf，纯 Rust）
//!
//! 提供：合并 / 拆分(逐页) / 旋转 / 删页。
//! 注意：lopdf 无现成 merge，合并用 renumber + 重建页树实现，输出建议实测校验。

use std::path::Path;

use lopdf::{Dictionary, Document, Object, ObjectId};

use crate::error::GlyphError;

fn e<E: std::fmt::Debug>(x: E) -> GlyphError {
    GlyphError::Internal(format!("{x:?}"))
}

fn parse_pages(s: &str) -> Vec<u32> {
    s.split(',')
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .filter_map(|p| p.parse::<u32>().ok())
        .collect()
}

/// 合并多个 PDF → out
pub fn merge(paths: &[String], out: &Path) -> Result<(), GlyphError> {
    if paths.is_empty() {
        return Err(GlyphError::Internal("没有要合并的文件".into()));
    }
    let mut merged = Document::with_version("1.5");
    let mut all_pages: Vec<ObjectId> = Vec::new();
    let mut next_id: u32 = 1;

    for p in paths {
        let mut doc = Document::load(p).map_err(e)?;
        doc.renumber_objects_with(next_id);
        let page_ids: Vec<ObjectId> = doc.get_pages().values().copied().collect();
        for (id, obj) in doc.objects.into_iter() {
            if id.0 >= next_id {
                next_id = id.0 + 1;
            }
            // 关键：手动 insert 不更新 max_id，必须同步，否则后续 new_object_id
            // 生成的 Pages/Catalog id 会与内容对象撞车 → 覆盖 → 空白
            if id.0 > merged.max_id {
                merged.max_id = id.0;
            }
            merged.objects.insert(id, obj);
        }
        all_pages.extend(page_ids);
    }

    let pages_id = merged.new_object_id();
    let kids: Vec<Object> = all_pages.iter().map(|id| Object::Reference(*id)).collect();
    let count = all_pages.len() as i64;

    for pid in &all_pages {
        if let Ok(d) = merged.get_dictionary_mut(*pid) {
            d.set("Parent", Object::Reference(pages_id));
        }
    }

    let mut pages_dict = Dictionary::new();
    pages_dict.set("Type", Object::Name(b"Pages".to_vec()));
    pages_dict.set("Kids", Object::Array(kids));
    pages_dict.set("Count", Object::Integer(count));
    merged.objects.insert(pages_id, Object::Dictionary(pages_dict));

    let catalog_id = merged.new_object_id();
    let mut catalog = Dictionary::new();
    catalog.set("Type", Object::Name(b"Catalog".to_vec()));
    catalog.set("Pages", Object::Reference(pages_id));
    merged.objects.insert(catalog_id, Object::Dictionary(catalog));
    merged.trailer.set("Root", Object::Reference(catalog_id));

    // 不调 compress()：源流多已 FlateDecode 压缩，重压缩会损坏内容流/图片 → 空白页
    merged.save(out).map_err(e)?;
    Ok(())
}

/// 逐页拆分：每个源 PDF 的每页存为单独文件到 out_dir，返回生成的路径
pub fn split(path: &Path, out_dir: &Path) -> Result<Vec<String>, GlyphError> {
    let src = Document::load(path).map_err(e)?;
    let total = src.get_pages().len() as u32;
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("split");
    let mut outs = Vec::new();
    for i in 1..=total {
        let mut doc = Document::load(path).map_err(e)?;
        let del: Vec<u32> = (1..=total).filter(|n| *n != i).collect();
        doc.delete_pages(&del);
        doc.prune_objects();
        doc.delete_zero_length_streams();
        doc.renumber_objects();
        let out = out_dir.join(format!("{stem}-p{i}.pdf"));
        doc.save(&out).map_err(e)?;
        outs.push(out.to_string_lossy().to_string());
    }
    Ok(outs)
}

/// 旋转指定页（pages_csv 空 = 全部；angle 任意，规整到 0/90/180/270）
pub fn rotate(path: &Path, pages_csv: &str, angle: i32, out: &Path) -> Result<(), GlyphError> {
    let mut doc = Document::load(path).map_err(e)?;
    let page_map = doc.get_pages();
    let wanted = parse_pages(pages_csv);
    let ids: Vec<ObjectId> = if wanted.is_empty() {
        page_map.values().copied().collect()
    } else {
        wanted.iter().filter_map(|n| page_map.get(n).copied()).collect()
    };
    let norm = angle.rem_euclid(360);
    for id in ids {
        if let Ok(d) = doc.get_dictionary_mut(id) {
            let cur = match d.get(b"Rotate") {
                Ok(Object::Integer(i)) => *i as i32,
                _ => 0,
            };
            d.set("Rotate", Object::Integer((cur + norm).rem_euclid(360) as i64));
        }
    }
    doc.save(out).map_err(e)?;
    Ok(())
}

/// 删除指定页（pages_csv 逗号分隔，1-based）
pub fn delete(path: &Path, pages_csv: &str, out: &Path) -> Result<(), GlyphError> {
    let mut doc = Document::load(path).map_err(e)?;
    let pages = parse_pages(pages_csv);
    if pages.is_empty() {
        return Err(GlyphError::Internal("没有要删除的页".into()));
    }
    doc.delete_pages(&pages);
    doc.prune_objects();
    doc.delete_zero_length_streams();
    doc.renumber_objects();
    doc.save(out).map_err(e)?;
    Ok(())
}

/// 页数（前端展示用）
pub fn page_count(path: &Path) -> Result<u32, GlyphError> {
    let doc = Document::load(path).map_err(e)?;
    Ok(doc.get_pages().len() as u32)
}

/// 提取 PDF 纯文本（pdf-extract，纯 Rust）
pub fn extract_text(path: &Path) -> Result<String, GlyphError> {
    pdf_extract::extract_text(path)
        .map_err(|err| GlyphError::Internal(format!("提取文字失败: {err}")))
}

/// 解析 "1-3,5,8-10" → [[1,2,3],[5],[8,9,10]]，每个子数组为一组
fn parse_range_groups(s: &str) -> Vec<Vec<u32>> {
    s.split(',')
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
        .filter_map(|t| {
            if let Some((a, b)) = t.split_once('-') {
                let a = a.trim().parse::<u32>().ok()?;
                let b = b.trim().parse::<u32>().ok()?;
                let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
                Some((lo..=hi).collect::<Vec<u32>>())
            } else {
                Some(vec![t.parse::<u32>().ok()?])
            }
        })
        .collect()
}

/// 提取指定页为新 PDF（pages_csv 逗号分隔，1-based；与 delete 互补）
pub fn extract(path: &Path, pages_csv: &str, out: &Path) -> Result<(), GlyphError> {
    let mut doc = Document::load(path).map_err(e)?;
    let keep = parse_pages(pages_csv);
    if keep.is_empty() {
        return Err(GlyphError::Internal("没有要提取的页".into()));
    }
    let total = doc.get_pages().len() as u32;
    let del: Vec<u32> = (1..=total).filter(|n| !keep.contains(n)).collect();
    doc.delete_pages(&del);
    doc.prune_objects();
    doc.delete_zero_length_streams();
    doc.renumber_objects();
    doc.save(out).map_err(e)?;
    Ok(())
}

/// 按范围拆分：spec 形如 "1-3,4-8"，每个范围输出一个文件；返回生成路径
pub fn split_ranges(path: &Path, spec: &str, out_dir: &Path) -> Result<Vec<String>, GlyphError> {
    let total = Document::load(path).map_err(e)?.get_pages().len() as u32;
    let groups: Vec<Vec<u32>> = parse_range_groups(spec)
        .into_iter()
        .map(|g| g.into_iter().filter(|n| *n >= 1 && *n <= total).collect::<Vec<u32>>())
        .filter(|g| !g.is_empty())
        .collect();
    if groups.is_empty() {
        return Err(GlyphError::Internal("没有有效的拆分范围".into()));
    }
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("split");
    let mut outs = Vec::new();
    for group in &groups {
        let mut doc = Document::load(path).map_err(e)?;
        let del: Vec<u32> = (1..=total).filter(|n| !group.contains(n)).collect();
        doc.delete_pages(&del);
        doc.prune_objects();
        doc.delete_zero_length_streams();
        doc.renumber_objects();
        let name = if group.len() == 1 {
            format!("{stem}-p{}.pdf", group[0])
        } else {
            format!("{stem}-p{}-{}.pdf", group.first().unwrap(), group.last().unwrap())
        };
        let out = out_dir.join(name);
        doc.save(&out).map_err(e)?;
        outs.push(out.to_string_lossy().to_string());
    }
    Ok(outs)
}

/// 将 PDF 页面渲染为图片（纯 Rust，pdf_oxide + tiny-skia，无需 pdfium.dll）
///
/// - `pages_csv`：逗号分隔的 1-based 页码，空 = 全部
/// - `dpi`：渲染分辨率（建议 96~300）
/// - `format`："png" | "jpg"/"jpeg"
/// - 返回生成的图片路径列表
pub fn to_images(
    path: &Path,
    pages_csv: &str,
    dpi: u32,
    format: &str,
    out_dir: &Path,
) -> Result<Vec<String>, GlyphError> {
    use pdf_oxide::api::Pdf;
    use pdf_oxide::rendering::RenderOptions;

    let mut pdf = Pdf::open(path).map_err(e)?;
    let total = pdf.page_count().map_err(e)?;
    let wanted = parse_pages(pages_csv);
    let targets: Vec<usize> = if wanted.is_empty() {
        (1..=total).collect()
    } else {
        wanted
            .iter()
            .map(|n| *n as usize)
            .filter(|n| *n >= 1 && *n <= total)
            .collect()
    };
    if targets.is_empty() {
        return Err(GlyphError::Internal("没有要导出的页".into()));
    }
    let is_jpg = format.eq_ignore_ascii_case("jpg") || format.eq_ignore_ascii_case("jpeg");
    let ext = if is_jpg { "jpg" } else { "png" };
    let opts = if is_jpg {
        RenderOptions::with_dpi(dpi.max(1)).as_jpeg(90)
    } else {
        RenderOptions::with_dpi(dpi.max(1))
    };
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("page");
    let mut outs = Vec::new();
    for p in targets {
        let img = pdf.render_page_with_options(p - 1, &opts).map_err(e)?;
        let out = out_dir.join(format!("{stem}-p{p}.{ext}"));
        img.save(&out).map_err(e)?;
        // pdf_oxide/image 保存时不写密度元数据（PNG 无 pHYs / JPEG 无 JFIF 密度），
        // 导致像素虽是 dpi 对应尺寸但资源管理器仍按 96dpi 显示。这里回写真实 DPI。
        if is_jpg {
            let _ = embed_jpeg_dpi(&out, dpi);
        } else {
            let _ = embed_png_dpi(&out, dpi);
        }
        outs.push(out.to_string_lossy().to_string());
    }
    Ok(outs)
}

/// 标准 CRC32（zlib/PNG 多项式 0xEDB88320）
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// 在 PNG 中写入/替换 pHYs 块（物理像素密度，单位=米），紧跟 IHDR。
/// 非预期结构则静默跳过，不阻断导出。
fn embed_png_dpi(path: &Path, dpi: u32) -> Result<(), GlyphError> {
    const SIG: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
    let bytes = std::fs::read(path).map_err(e)?;
    // 签名(8) + IHDR 块固定 25 字节（4 长度 + 4 类型 + 13 数据 + 4 CRC）
    if bytes.len() < 33 || &bytes[0..8] != SIG {
        return Ok(());
    }
    let ppm = (dpi.max(1) as f64 * 39.3701).round().max(1.0) as u32; // 像素/米
    let mut data = Vec::with_capacity(9);
    data.extend_from_slice(&ppm.to_be_bytes());
    data.extend_from_slice(&ppm.to_be_bytes());
    data.push(1); // 1 = 以米为单位
    let mut phys: Vec<u8> = Vec::with_capacity(21);
    phys.extend_from_slice(&(data.len() as u32).to_be_bytes());
    phys.extend_from_slice(b"pHYs");
    phys.extend_from_slice(&data);
    let mut crc_input = Vec::with_capacity(13);
    crc_input.extend_from_slice(b"pHYs");
    crc_input.extend_from_slice(&data);
    phys.extend_from_slice(&crc32(&crc_input).to_be_bytes());

    let mut out: Vec<u8> = Vec::with_capacity(bytes.len() + phys.len());
    out.extend_from_slice(&bytes[0..33]); // 签名 + IHDR
    out.extend_from_slice(&phys);
    // 其余块，跳过已存在的 pHYs（避免重复）
    let mut pos = 33;
    while pos + 8 <= bytes.len() {
        let len = u32::from_be_bytes(bytes[pos..pos + 4].try_into().unwrap()) as usize;
        let ctype = &bytes[pos + 4..pos + 8];
        let end = pos + 8 + len + 4;
        if end > bytes.len() {
            break;
        }
        if ctype != b"pHYs" {
            out.extend_from_slice(&bytes[pos..end]);
        }
        pos = end;
    }
    std::fs::write(path, out).map_err(e)
}

/// 在 JPEG 中写入/替换 JFIF APP0 密度（units=1=dpi）。找不到 JFIF 则插入一个。
fn embed_jpeg_dpi(path: &Path, dpi: u32) -> Result<(), GlyphError> {
    let bytes = std::fs::read(path).map_err(e)?;
    if bytes.len() < 4 || bytes[0] != 0xFF || bytes[1] != 0xD8 {
        return Ok(());
    }
    let d = (dpi.clamp(1, 65535)) as u16;
    let mut pos = 2;
    while pos + 4 <= bytes.len() {
        if bytes[pos] != 0xFF {
            break;
        }
        let marker = bytes[pos + 1];
        if marker == 0xDA {
            break; // SOS，之后为压缩数据
        }
        let seglen = u16::from_be_bytes(bytes[pos + 2..pos + 4].try_into().unwrap()) as usize;
        // APP0 + "JFIF\0"：标识(5) 后跟 版本(2) 单位(1) Xdensity(2) Ydensity(2)
        if marker == 0xE0 && pos + 9 <= bytes.len() && &bytes[pos + 4..pos + 9] == b"JFIF\0" {
            let base = pos + 4 + 5 + 2; // 跳过 "JFIF\0" + 版本 2 字节 → 单位字节
            if base + 5 <= bytes.len() {
                let mut out = bytes.clone();
                out[base] = 1; // units = dpi
                out[base + 1..base + 3].copy_from_slice(&d.to_be_bytes());
                out[base + 3..base + 5].copy_from_slice(&d.to_be_bytes());
                return std::fs::write(path, out).map_err(e);
            }
            return Ok(());
        }
        if seglen < 2 {
            break;
        }
        pos += 2 + seglen;
    }
    // 未找到 JFIF APP0 → 在 SOI 后插入一个最小 JFIF
    let mut app0: Vec<u8> = Vec::with_capacity(20);
    app0.extend_from_slice(&[0xFF, 0xE0]);
    app0.extend_from_slice(&16u16.to_be_bytes()); // 段长度
    app0.extend_from_slice(b"JFIF\0");
    app0.extend_from_slice(&[1, 1]); // version 1.1
    app0.push(1); // units = dpi
    app0.extend_from_slice(&d.to_be_bytes());
    app0.extend_from_slice(&d.to_be_bytes());
    app0.extend_from_slice(&[0, 0]); // 无缩略图
    let mut out = Vec::with_capacity(bytes.len() + app0.len());
    out.extend_from_slice(&bytes[0..2]); // SOI
    out.extend_from_slice(&app0);
    out.extend_from_slice(&bytes[2..]);
    std::fs::write(path, out).map_err(e)
}
