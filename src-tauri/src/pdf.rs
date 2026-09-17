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
