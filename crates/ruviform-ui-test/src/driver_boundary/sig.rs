//! public signature 漏出検査（Issue #390 driver boundary guard の
//! signature 系統）。
//!
//! 全 workspace member の lib target を走査し、公開 API に
//! forbidden path root（`gpui`/`wry`/`tao` 等）が現れることを検出
//! する。allowlisted adapter も対象 — adapter の *公開 API* に
//! backend 型が出ること自体が漏出であり、private 利用は `pub`
//! item でないため検出しない。
//!
//! 公開到達性（exposure）は pub-in 意味論を近似する:
//! - crate root scope は全 `pub` item が公開（`Exposure::Full`）。
//! - `pub mod m` は m の全 `pub` item を公開。
//! - `pub use m::{X, Y}` / `pub use m::X` は private mod m の
//!   指定 item のみを公開（`Exposure::Names`）。`pub use m::*` と
//!   `pub use m::sub::Y`（2 段以上）は m を glob 公開と近似。
//! - `self`/`crate`/`super` 先頭 segment は strip して判定。
//! - `mod` decl は exposure に関わらず全て辿る — `impl` block は
//!   self/trait type 名が crate 全体の公開 type 名集合（pass 1 で
//!   収集）に含まれる場合のみ走査し、private mod 内の
//!   `impl PubType { pub fn … }` 漏出も捕捉する。
//!
//! 限界: macro 内の型・生成コード・`#[path]` の複雑解決・
//! forbidden root と同名 local mod（誤検出 → rename 要求）・
//! private type alias 経由の間接露出は検出しない。

use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use cargo_metadata::Metadata;
use syn::visit::Visit;

use super::Finding;

const CHECK: &str = "signature";

/// scope の公開到達性。
enum Exposure {
    /// 全 `pub` item が公開（crate root / `pub mod` / glob re-export）。
    Full,
    /// `pub use m::{X, Y}` で一部 item のみ公開される private mod。
    Names(Rc<HashSet<String>>),
    /// 公開到達しない scope（`impl` block のみ pub type 名で走査）。
    Hidden,
}

/// 全 workspace member の lib target を走査する。
pub fn check(metadata: &Metadata, forbidden: &[String]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for member_id in &metadata.workspace_members {
        let Some(pkg) = metadata.packages.iter().find(|p| &p.id == member_id) else {
            continue;
        };
        for target in &pkg.targets {
            if !target
                .kind
                .iter()
                .any(|k| k == &cargo_metadata::TargetKind::Lib)
            {
                continue;
            }
            let root = PathBuf::from(target.src_path.as_str());
            // pass 1: 公開 type 名収集（impl gating 用）。
            let mut pub_types = HashSet::new();
            let mut seen = HashMap::new();
            let mut mode = Mode::CollectTypes(&mut pub_types);
            walk_file(
                &root,
                pkg.name.as_str(),
                &Exposure::Full,
                &mut seen,
                &mut mode,
                forbidden,
                &mut findings,
            );
            // pass 2: 漏出検査。
            let mut seen = HashMap::new();
            let mut mode = Mode::Scan(&pub_types);
            walk_file(
                &root,
                pkg.name.as_str(),
                &Exposure::Full,
                &mut seen,
                &mut mode,
                forbidden,
                &mut findings,
            );
        }
    }
    findings
}

enum Mode<'a> {
    /// 公開 type 名を収集する pass。
    CollectTypes(&'a mut HashSet<String>),
    /// findings を出す pass（`pub_types` は収集済み集合）。
    Scan(&'a HashSet<String>),
}

/// file 単位の到達済み exposure。Names は union merge し、
/// exposure が広がった場合のみ再 scan する。
enum Seen {
    Full,
    Names(HashSet<String>),
    Hidden,
}

fn needs_rescan(seen: &mut HashMap<PathBuf, Seen>, path: &Path, exp: &Exposure) -> bool {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    match (seen.entry(canonical), exp) {
        (Entry::Vacant(e), Exposure::Full) => {
            e.insert(Seen::Full);
            true
        }
        (Entry::Vacant(e), Exposure::Names(s)) => {
            e.insert(Seen::Names(s.as_ref().clone()));
            true
        }
        (Entry::Vacant(e), Exposure::Hidden) => {
            e.insert(Seen::Hidden);
            true
        }
        // exposure が狭い→広い場合のみ再 scan。
        (Entry::Occupied(mut e), Exposure::Full) => {
            if matches!(e.get(), Seen::Full) {
                false
            } else {
                e.insert(Seen::Full);
                true
            }
        }
        (Entry::Occupied(_), Exposure::Hidden) => false,
        (Entry::Occupied(mut e), Exposure::Names(s)) => {
            let next: Option<Seen> = match e.get() {
                Seen::Full => None,
                Seen::Hidden => Some(Seen::Names(s.as_ref().clone())),
                Seen::Names(old) => {
                    if s.iter().all(|n| old.contains(n)) {
                        None
                    } else {
                        let mut merged = old.clone();
                        merged.extend(s.iter().cloned());
                        Some(Seen::Names(merged))
                    }
                }
            };
            match next {
                Some(v) => {
                    e.insert(v);
                    true
                }
                None => false,
            }
        }
    }
}

fn walk_file(
    path: &Path,
    package: &str,
    exposure: &Exposure,
    seen: &mut HashMap<PathBuf, Seen>,
    mode: &mut Mode<'_>,
    forbidden: &[String],
    findings: &mut Vec<Finding>,
) {
    if !needs_rescan(seen, path, exposure) {
        return;
    }
    let exposed = matches!(exposure, Exposure::Full | Exposure::Names(_));
    let is_scan = matches!(*mode, Mode::Scan(_));
    let Ok(text) = std::fs::read_to_string(path) else {
        if is_scan && exposed {
            findings.push(Finding {
                check: CHECK,
                code: "signature-src-unreadable",
                package: package.to_string(),
                file: Some(path.display().to_string()),
                line: None,
                detail: "lib source を読めない（検査不能のため fail-closed）".to_string(),
            });
        }
        return;
    };
    let file = match syn::parse_file(&text) {
        Ok(f) => f,
        Err(e) => {
            if is_scan && exposed {
                findings.push(Finding {
                    check: CHECK,
                    code: "signature-src-unparseable",
                    package: package.to_string(),
                    file: Some(path.display().to_string()),
                    line: None,
                    detail: format!("syn parse 失敗（fail-closed）: {e}"),
                });
            }
            return;
        }
    };
    let children = process_scope(
        &file.items,
        path,
        package,
        exposure,
        mode,
        forbidden,
        findings,
    );
    for (name, path_attr, child_exp) in children {
        for candidate in resolve_mod_file(path, &name, path_attr.as_deref()) {
            if candidate.exists() {
                walk_file(
                    &candidate, package, &child_exp, seen, mode, forbidden, findings,
                );
                break;
            }
        }
    }
}

/// `mod foo;` の解決先候補。`foo.rs`/`foo/mod.rs`/`#[path]`。
fn resolve_mod_file(current: &Path, name: &str, path_attr: Option<&str>) -> Vec<PathBuf> {
    let dir = current.parent().unwrap_or_else(|| Path::new("."));
    if let Some(rel) = path_attr {
        return vec![dir.join(rel)];
    }
    let stem_is_mod = current
        .file_stem()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s == "mod" || s == "lib" || s == "main");
    let base = if stem_is_mod {
        dir.to_path_buf()
    } else {
        dir.join(current.file_stem().unwrap_or_default())
    };
    vec![
        base.join(format!("{name}.rs")),
        base.join(name).join("mod.rs"),
    ]
}

/// scope 内 item を処理し、子 mod 宣言（名前・`#[path]`・exposure）
/// を返す。
fn process_scope(
    items: &[syn::Item],
    path: &Path,
    package: &str,
    exposure: &Exposure,
    mode: &mut Mode<'_>,
    forbidden: &[String],
    findings: &mut Vec<Finding>,
) -> Vec<(String, Option<String>, Exposure)> {
    // `pub use` による mod 露出 map（Full scope のみ意味を持つ）。
    let use_map = if matches!(exposure, Exposure::Full) {
        build_use_map(items)
    } else {
        HashMap::new()
    };
    // この scope の `use` alias map（`use wry::X as W` → `W`→`wry`）。
    // signature 内の裸 ident を forbidden root へ解決する。
    let aliases = build_alias_map(items, forbidden);
    let mut children = Vec::new();
    for item in items {
        match item {
            syn::Item::Mod(m) => {
                let child_exp = child_exposure(m, exposure, &use_map);
                match (&m.content, child_exp) {
                    (Some((_, inner)), exp) => {
                        // inline mod: 未公開でも impl 捕捉のため
                        // Hidden で中身を辿る。
                        let inner_exp = exp.unwrap_or(Exposure::Hidden);
                        children.extend(process_scope(
                            inner, path, package, &inner_exp, mode, forbidden, findings,
                        ));
                    }
                    (None, exp) => {
                        let attr = path_attr(&m.attrs);
                        children.push((m.ident.to_string(), attr, exp.unwrap_or(Exposure::Hidden)));
                    }
                }
            }
            syn::Item::Impl(i) => {
                if let Mode::Scan(pub_types) = mode {
                    // impl header（self_ty / trait path）は公開性に
                    // 関わらず走査 — `impl wry::Trait for T` 自体が
                    // 型の public surface に backend 型を露出する。
                    scan_impl_header(i, path, package, &aliases, forbidden, findings);
                    if impl_is_public(i, pub_types) {
                        scan_impl(i, path, package, &aliases, forbidden, findings);
                    }
                }
            }
            _ => {
                if item_visible(item, exposure) {
                    match mode {
                        Mode::CollectTypes(out) => collect_type_names(item, out),
                        Mode::Scan(_) => {
                            scan_item(item, path, package, &aliases, forbidden, findings);
                        }
                    }
                }
            }
        }
    }
    children
}

fn path_attr(attrs: &[syn::Attribute]) -> Option<String> {
    attrs
        .iter()
        .find(|a| a.path().is_ident("path"))
        .and_then(|a| a.parse_args::<syn::LitStr>().ok().map(|l| l.value()))
}

fn child_exposure(
    m: &syn::ItemMod,
    scope_exp: &Exposure,
    use_map: &HashMap<String, UseExp>,
) -> Option<Exposure> {
    match scope_exp {
        Exposure::Full => {
            if is_pub(&m.vis) {
                Some(Exposure::Full)
            } else {
                match use_map.get(&m.ident.to_string()) {
                    Some(UseExp::Glob) => Some(Exposure::Full),
                    Some(UseExp::Names(s)) => Some(Exposure::Names(Rc::new(s.clone()))),
                    None => None,
                }
            }
        }
        // Names scope: `pub use` 経由の再露出は近似せず、set 内の
        // 名前（re-export された mod 自体）のみ Full で公開。
        Exposure::Names(set) => {
            if set.contains(&m.ident.to_string()) {
                Some(Exposure::Full)
            } else {
                None
            }
        }
        Exposure::Hidden => None,
    }
}

enum UseExp {
    Glob,
    Names(HashSet<String>),
}

/// scope 内の `use` 宣言から ident → forbidden root の alias map を
/// 構築する（`use wry::X as W` / `use wry::X` の裸 ident 回避を捕捉）。
/// glob import（`use wry::*`）は解決不能なため限界として残す。
fn build_alias_map(items: &[syn::Item], forbidden: &[String]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for item in items {
        let syn::Item::Use(u) = item else {
            continue;
        };
        collect_use_aliases(&u.tree, &mut Vec::new(), forbidden, &mut map);
    }
    map
}

fn collect_use_aliases(
    tree: &syn::UseTree,
    prefix: &mut Vec<String>,
    forbidden: &[String],
    map: &mut HashMap<String, String>,
) {
    fn strip_prefix(segs: &[String]) -> Vec<&String> {
        segs.iter()
            .skip_while(|s| s.as_str() == "self" || s.as_str() == "crate" || s.as_str() == "super")
            .collect()
    }
    let strip = strip_prefix;
    match tree {
        syn::UseTree::Path(p) => {
            prefix.push(p.ident.to_string());
            collect_use_aliases(&p.tree, prefix, forbidden, map);
            prefix.pop();
        }
        syn::UseTree::Name(n) => {
            let mut full = prefix.clone();
            full.push(n.ident.to_string());
            let segs = strip(&full);
            if let (Some(first), Some(last)) = (segs.first(), segs.last())
                && forbidden.iter().any(|f| f == *first)
            {
                map.insert((*last).clone(), (*first).clone());
            }
        }
        syn::UseTree::Rename(r) => {
            let mut full = prefix.clone();
            full.push(r.ident.to_string());
            let segs = strip(&full);
            if let Some(first) = segs.first()
                && forbidden.iter().any(|f| f == *first)
            {
                map.insert(r.rename.to_string(), (*first).clone());
            }
        }
        // glob は ident 解決不能 — 限界として限界節に記録。
        syn::UseTree::Glob(_) => {}
        syn::UseTree::Group(g) => {
            for t in &g.items {
                collect_use_aliases(t, prefix, forbidden, map);
            }
        }
    }
}

/// scope 直下の `pub use` を展開し、mod 名 → 露出度 map を作る。
fn build_use_map(items: &[syn::Item]) -> HashMap<String, UseExp> {
    let mut map: HashMap<String, UseExp> = HashMap::new();
    for item in items {
        let syn::Item::Use(u) = item else {
            continue;
        };
        if !is_pub(&u.vis) {
            continue;
        }
        let mut paths = Vec::new();
        flatten_use(&u.tree, &mut Vec::new(), &mut paths);
        for path in paths {
            // 先頭の self/crate/super を strip。
            let segs: Vec<&String> = path
                .iter()
                .skip_while(|s| {
                    s.as_str() == "self" || s.as_str() == "crate" || s.as_str() == "super"
                })
                .collect();
            match segs.len() {
                0 => {}
                1 => {
                    // `pub use m;` — mod 自体の再 export の可能性。
                    map.insert(segs[0].clone(), UseExp::Glob);
                }
                _ => {
                    let entry = map
                        .entry(segs[0].clone())
                        .or_insert_with(|| UseExp::Names(HashSet::new()));
                    if matches!(entry, UseExp::Names(_)) {
                        if segs[1].as_str() == "*" || segs.len() > 2 {
                            *entry = UseExp::Glob;
                        } else if let UseExp::Names(s) = entry {
                            s.insert(segs[1].clone());
                        }
                    }
                }
            }
        }
    }
    map
}

/// use tree を path（segment 列）へ展開。
fn flatten_use(tree: &syn::UseTree, prefix: &mut Vec<String>, out: &mut Vec<Vec<String>>) {
    match tree {
        syn::UseTree::Path(p) => {
            prefix.push(p.ident.to_string());
            flatten_use(&p.tree, prefix, out);
            prefix.pop();
        }
        syn::UseTree::Name(n) => {
            let mut v = prefix.clone();
            v.push(n.ident.to_string());
            out.push(v);
        }
        syn::UseTree::Rename(r) => {
            let mut v = prefix.clone();
            v.push(r.ident.to_string());
            out.push(v);
        }
        syn::UseTree::Glob(_) => {
            let mut v = prefix.clone();
            v.push("*".to_string());
            out.push(v);
        }
        syn::UseTree::Group(g) => {
            for t in &g.items {
                flatten_use(t, prefix, out);
            }
        }
    }
}

fn item_ident(item: &syn::Item) -> Option<String> {
    match item {
        syn::Item::Fn(i) => Some(i.sig.ident.to_string()),
        syn::Item::Struct(i) => Some(i.ident.to_string()),
        syn::Item::Enum(i) => Some(i.ident.to_string()),
        syn::Item::Union(i) => Some(i.ident.to_string()),
        syn::Item::Type(i) => Some(i.ident.to_string()),
        syn::Item::Trait(i) => Some(i.ident.to_string()),
        syn::Item::Static(i) => Some(i.ident.to_string()),
        syn::Item::Const(i) => Some(i.ident.to_string()),
        syn::Item::ExternCrate(i) => Some(i.ident.to_string()),
        _ => None,
    }
}

// variant ごとに binding 型が違うため同一 body の arm は merge できない。
#[allow(clippy::match_same_arms)]
fn item_vis(item: &syn::Item) -> Option<&syn::Visibility> {
    match item {
        syn::Item::Fn(i) => Some(&i.vis),
        syn::Item::Struct(i) => Some(&i.vis),
        syn::Item::Enum(i) => Some(&i.vis),
        syn::Item::Union(i) => Some(&i.vis),
        syn::Item::Type(i) => Some(&i.vis),
        syn::Item::Trait(i) => Some(&i.vis),
        syn::Item::Static(i) => Some(&i.vis),
        syn::Item::Const(i) => Some(&i.vis),
        syn::Item::Use(i) => Some(&i.vis),
        syn::Item::ExternCrate(i) => Some(&i.vis),
        syn::Item::ForeignMod(_) => None, // block 自体に vis はない
        _ => None,
    }
}

fn item_visible(item: &syn::Item, exposure: &Exposure) -> bool {
    // ForeignMod は block に vis が無い — 内部の pub item を scope
    // 露出で判断するため常に「見える」扱いにする。
    let vis = item_vis(item);
    let is_pub_item = vis.is_some_and(is_pub) || matches!(item, syn::Item::ForeignMod(_));
    match exposure {
        Exposure::Full => is_pub_item,
        Exposure::Names(set) => {
            // `pub use` item は ident を持たない — Names scope 内の
            // re-export 連鎖（`pub use wry::Y`）も漏出し得るため
            // 保守的に全走査する。
            if matches!(item, syn::Item::Use(_)) {
                return is_pub_item;
            }
            is_pub_item && item_ident(item).is_some_and(|n| set.contains(&n))
        }
        Exposure::Hidden => false,
    }
}

/// pass 1: 公開 type 名を収集（impl gating 用）。struct/enum/union/
/// trait/type alias + `pub use` の leaf 名。
fn collect_type_names(item: &syn::Item, out: &mut HashSet<String>) {
    match item {
        syn::Item::Struct(_)
        | syn::Item::Enum(_)
        | syn::Item::Union(_)
        | syn::Item::Trait(_)
        | syn::Item::Type(_) => {
            if let Some(n) = item_ident(item) {
                out.insert(n);
            }
        }
        syn::Item::Use(u) => {
            let mut paths = Vec::new();
            flatten_use(&u.tree, &mut Vec::new(), &mut paths);
            for path in &paths {
                if let Some(last) = path.iter().rev().find(|s| s.as_str() != "*") {
                    out.insert(last.clone());
                }
            }
        }
        _ => {}
    }
}

/// impl block が公開 API に関わるか: inherent impl は self type が
/// 公開 type、trait impl は trait または self type が公開 type の
/// 場合に走査する。
fn impl_is_public(i: &syn::ItemImpl, pub_types: &HashSet<String>) -> bool {
    let self_name = first_seg(&i.self_ty);
    if let Some((_, t, _)) = &i.trait_ {
        let trait_name = t.segments.first().map(|s| s.ident.to_string());
        trait_name.is_some_and(|n| pub_types.contains(&n))
            || self_name.is_some_and(|n| pub_types.contains(&n))
    } else {
        self_name.is_some_and(|n| pub_types.contains(&n))
    }
}

fn first_seg(t: &syn::Type) -> Option<String> {
    if let syn::Type::Path(tp) = t {
        tp.path.segments.first().map(|s| s.ident.to_string())
    } else {
        None
    }
}

// --- finding 生成 ---

struct Ctx<'a> {
    file: String,
    package: &'a str,
}

fn ctx<'a>(path: &Path, package: &'a str) -> Ctx<'a> {
    Ctx {
        file: path.display().to_string(),
        package,
    }
}

fn leak(c: &Ctx<'_>, kind: &str, name: &str, root: &str, path_text: &str, line: u32) -> Finding {
    Finding {
        check: CHECK,
        code: "signature-leak",
        package: c.package.to_string(),
        file: Some(c.file.clone()),
        line: Some(line),
        detail: format!(
            "pub {kind} `{name}` の signature が forbidden path root `{root}` を参照（{path_text}）"
        ),
    }
}

struct PathScan<'a, 'b> {
    c: &'a Ctx<'a>,
    kind: &'a str,
    name: &'a str,
    aliases: &'a HashMap<String, String>,
    forbidden: &'a [String],
    findings: &'b mut Vec<Finding>,
}

impl<'ast> Visit<'ast> for PathScan<'_, '_> {
    fn visit_path(&mut self, p: &'ast syn::Path) {
        if let Some(seg) = p.segments.first() {
            let root = seg.ident.to_string();
            // `use wry::X as W` 経由の alias も解決する。
            let resolved = if self.forbidden.iter().any(|f| f == &root) {
                Some(root.clone())
            } else {
                self.aliases.get(&root).cloned()
            };
            if let Some(root) = resolved {
                self.findings.push(leak(
                    self.c,
                    self.kind,
                    self.name,
                    &root,
                    &path_text(p),
                    u32::try_from(seg.ident.span().start().line).unwrap_or(u32::MAX),
                ));
            }
        }
        syn::visit::visit_path(self, p);
    }
}

fn path_text(p: &syn::Path) -> String {
    p.segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

fn is_pub(vis: &syn::Visibility) -> bool {
    matches!(vis, syn::Visibility::Public(_))
}

// --- item 走査 ---

// syn item variant ごとの signature 走査を一本の match に集約する
// — 分割すると item 種別網羅が見通しにくくなるため大きさを許容する。
#[allow(clippy::too_many_lines)]
fn scan_item(
    item: &syn::Item,
    path: &Path,
    package: &str,
    aliases: &HashMap<String, String>,
    forbidden: &[String],
    findings: &mut Vec<Finding>,
) {
    let c = ctx(path, package);
    match item {
        syn::Item::Fn(i) => scan_sig(
            &i.sig,
            &c,
            "fn",
            &i.sig.ident.to_string(),
            aliases,
            forbidden,
            findings,
        ),
        syn::Item::Struct(i) => {
            let name = i.ident.to_string();
            scan_gen(
                &i.generics,
                &c,
                "struct",
                &name,
                aliases,
                forbidden,
                findings,
            );
            for f in &i.fields {
                if is_pub(&f.vis) {
                    scan_ty(&f.ty, &c, "struct", &name, aliases, forbidden, findings);
                }
            }
        }
        syn::Item::Enum(i) => {
            let name = i.ident.to_string();
            scan_gen(&i.generics, &c, "enum", &name, aliases, forbidden, findings);
            for v in &i.variants {
                for f in &v.fields {
                    scan_ty(&f.ty, &c, "enum", &name, aliases, forbidden, findings);
                }
            }
        }
        syn::Item::Union(i) => {
            let name = i.ident.to_string();
            scan_gen(
                &i.generics,
                &c,
                "union",
                &name,
                aliases,
                forbidden,
                findings,
            );
            for f in &i.fields.named {
                if is_pub(&f.vis) {
                    scan_ty(&f.ty, &c, "union", &name, aliases, forbidden, findings);
                }
            }
        }
        syn::Item::Type(i) => {
            let name = i.ident.to_string();
            scan_gen(&i.generics, &c, "type", &name, aliases, forbidden, findings);
            scan_ty(&i.ty, &c, "type", &name, aliases, forbidden, findings);
        }
        syn::Item::Trait(i) => {
            let name = i.ident.to_string();
            scan_gen(
                &i.generics,
                &c,
                "trait",
                &name,
                aliases,
                forbidden,
                findings,
            );
            for b in &i.supertraits {
                scan_bound(b, &c, "trait", &name, aliases, forbidden, findings);
            }
            for ti in &i.items {
                match ti {
                    syn::TraitItem::Fn(f) => {
                        scan_sig(&f.sig, &c, "trait", &name, aliases, forbidden, findings);
                    }
                    syn::TraitItem::Type(t) => {
                        scan_gen(
                            &t.generics,
                            &c,
                            "trait",
                            &name,
                            aliases,
                            forbidden,
                            findings,
                        );
                        for b in &t.bounds {
                            scan_bound(b, &c, "trait", &name, aliases, forbidden, findings);
                        }
                        if let Some((_, ty)) = &t.default {
                            scan_ty(ty, &c, "trait", &name, aliases, forbidden, findings);
                        }
                    }
                    syn::TraitItem::Const(co) => {
                        scan_ty(&co.ty, &c, "trait", &name, aliases, forbidden, findings);
                    }
                    _ => {}
                }
            }
        }
        syn::Item::Static(i) => scan_ty(
            &i.ty,
            &c,
            "static",
            &i.ident.to_string(),
            aliases,
            forbidden,
            findings,
        ),
        syn::Item::Const(i) => scan_ty(
            &i.ty,
            &c,
            "const",
            &i.ident.to_string(),
            aliases,
            forbidden,
            findings,
        ),
        syn::Item::Use(i) => {
            let name = "pub use".to_string();
            // `pub use` 自体の path root を検査（`pub use wry::X`）。
            let mut paths = Vec::new();
            flatten_use(&i.tree, &mut Vec::new(), &mut paths);
            for segs in paths {
                if let Some(first) = segs.iter().find(|s| {
                    s.as_str() != "self" && s.as_str() != "crate" && s.as_str() != "super"
                }) && forbidden.iter().any(|f| f == first)
                {
                    findings.push(leak(&c, "use", &name, first.as_str(), &segs.join("::"), 0));
                }
            }
        }
        syn::Item::ExternCrate(i) => {
            let name = i.ident.to_string();
            if forbidden.iter().any(|f| f == &name) {
                findings.push(leak(
                    &c,
                    "extern crate",
                    &name,
                    &name,
                    &name,
                    u32::try_from(i.ident.span().start().line).unwrap_or(u32::MAX),
                ));
            }
        }
        syn::Item::ForeignMod(fm) => {
            for fi in &fm.items {
                if let syn::ForeignItem::Fn(f) = fi
                    && is_pub(&f.vis)
                {
                    scan_sig(
                        &f.sig,
                        &c,
                        "extern fn",
                        &f.sig.ident.to_string(),
                        aliases,
                        forbidden,
                        findings,
                    );
                }
            }
        }
        _ => {}
    }
}

/// impl header の `self_ty` / trait path を走査する（公開性不問 —
/// `impl wry::Trait for T` は T の公開 surface に wry を露出する）。
fn scan_impl_header(
    i: &syn::ItemImpl,
    path: &Path,
    package: &str,
    aliases: &HashMap<String, String>,
    forbidden: &[String],
    findings: &mut Vec<Finding>,
) {
    let c = ctx(path, package);
    let name = format!(
        "impl {}",
        first_seg(&i.self_ty).unwrap_or_else(|| "?".into())
    );
    scan_ty(&i.self_ty, &c, "impl", &name, aliases, forbidden, findings);
    if let Some((_, t, _)) = &i.trait_ {
        PathScan {
            c: &c,
            kind: "impl",
            name: name.as_str(),
            aliases,
            forbidden,
            findings,
        }
        .visit_path(t);
    }
}

fn scan_impl(
    i: &syn::ItemImpl,
    path: &Path,
    package: &str,
    aliases: &HashMap<String, String>,
    forbidden: &[String],
    findings: &mut Vec<Finding>,
) {
    let c = ctx(path, package);
    let is_trait_impl = i.trait_.is_some();
    let name = format!(
        "impl {}",
        first_seg(&i.self_ty).unwrap_or_else(|| "?".into())
    );
    scan_gen(&i.generics, &c, "impl", &name, aliases, forbidden, findings);
    for ii in &i.items {
        match ii {
            syn::ImplItem::Fn(f) => {
                // inherent impl は pub のみ。trait impl は trait 側公開性に
                // 従うため全走査。
                if is_pub(&f.vis) || is_trait_impl {
                    scan_sig(&f.sig, &c, "impl", &name, aliases, forbidden, findings);
                }
            }
            syn::ImplItem::Type(t) => {
                scan_ty(&t.ty, &c, "impl", &name, aliases, forbidden, findings);
            }
            syn::ImplItem::Const(co) => {
                scan_ty(&co.ty, &c, "impl", &name, aliases, forbidden, findings);
            }
            _ => {}
        }
    }
}

fn scan_sig(
    sig: &syn::Signature,
    c: &Ctx<'_>,
    kind: &str,
    name: &str,
    aliases: &HashMap<String, String>,
    forbidden: &[String],
    findings: &mut Vec<Finding>,
) {
    PathScan {
        c,
        kind,
        name,
        aliases,
        forbidden,
        findings,
    }
    .visit_signature(sig);
}

fn scan_gen(
    g: &syn::Generics,
    c: &Ctx<'_>,
    kind: &str,
    name: &str,
    aliases: &HashMap<String, String>,
    forbidden: &[String],
    findings: &mut Vec<Finding>,
) {
    PathScan {
        c,
        kind,
        name,
        aliases,
        forbidden,
        findings,
    }
    .visit_generics(g);
}

fn scan_ty(
    t: &syn::Type,
    c: &Ctx<'_>,
    kind: &str,
    name: &str,
    aliases: &HashMap<String, String>,
    forbidden: &[String],
    findings: &mut Vec<Finding>,
) {
    PathScan {
        c,
        kind,
        name,
        aliases,
        forbidden,
        findings,
    }
    .visit_type(t);
}

fn scan_bound(
    b: &syn::TypeParamBound,
    c: &Ctx<'_>,
    kind: &str,
    name: &str,
    aliases: &HashMap<String, String>,
    forbidden: &[String],
    findings: &mut Vec<Finding>,
) {
    PathScan {
        c,
        kind,
        name,
        aliases,
        forbidden,
        findings,
    }
    .visit_type_param_bound(b);
}
