//! 目标解析：把「命令行传了什么」变成「要浏览哪一串文件、从哪个开始」。
//! 纯逻辑：只吃 paths + cwd，不碰终端。

use std::cmp::Ordering;
use std::path::{Path, PathBuf};

use color_eyre::eyre::bail;

/// 认的扩展名（大小写不敏感）。
const EXT: [&str; 2] = ["md", "markdown"];

fn is_md(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| EXT.iter().any(|x| e.eq_ignore_ascii_case(x)))
}

/// 目录下所有 md 文件，自然序；读不动的条目跳过，目录本身读不了就返回空。
pub fn list_md(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = match std::fs::read_dir(dir) {
        Ok(rd) => rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_file() && is_md(p))
            .map(|p| absolute(&p, Path::new(".")))
            .collect(),
        Err(_) => return Vec::new(),
    };
    out.sort_by(|a, b| natural_cmp(&file_key(a), &file_key(b)));
    out
}

/// 起始文件 + 要浏览的列表：无参数 = 当前目录；目录 = 该目录的 md，从第一个开始；
/// 文件 = 它所在目录的兄弟列表，起始就是它自己（非 .md 也插进列表，照旧能看）。
pub fn resolve_targets(paths: &[PathBuf], cwd: &Path) -> color_eyre::Result<(Vec<PathBuf>, usize)> {
    let arg = paths
        .first()
        .filter(|p| !p.as_os_str().is_empty())
        .cloned()
        .unwrap_or_else(|| PathBuf::from("."));
    let target = absolute(&arg, cwd);
    let dir = if target.is_dir() {
        target.clone()
    } else {
        target
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| cwd.to_path_buf())
    };
    let mut files = list_md(&dir);
    // 当前文件即使不是 .md（或被并排列表漏掉）也要能看、能切回来。
    if target.is_file() {
        insert_sorted(&mut files, target.clone());
    }
    if files.is_empty() {
        bail!("no .md file in {}", dir.display());
    }
    let start = files
        .iter()
        .position(|p| *p == target)
        .unwrap_or(0)
        .min(files.len() - 1);
    Ok((files, start))
}

/// 插进自然序里该在的位置；已在列表里就不动，返回它在列表中的下标。
pub fn insert_sorted(files: &mut Vec<PathBuf>, path: PathBuf) -> usize {
    let key = file_key(&path);
    match files.binary_search_by(|p| natural_cmp(&file_key(p), &key)) {
        Ok(i) => i,
        Err(i) => {
            files.insert(i, path);
            i
        }
    }
}

/// 绝对 + canonical：比较和去重都要走同一个形式（canonical 失败就用拼出来的路径）。
pub fn absolute(path: &Path, cwd: &Path) -> PathBuf {
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    };
    joined.canonicalize().unwrap_or(joined)
}

/// 排序键用文件名，父目录不同的文件才有可比性。
fn file_key(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_string()
}

/// 自然序：数字段按数值比（part2 < part10），其余按字节不区分大小写比。
fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (ab, bb) = (a.as_bytes(), b.as_bytes());
    let (mut i, mut j) = (0, 0);
    while i < ab.len() && j < bb.len() {
        if ab[i].is_ascii_digit() && bb[j].is_ascii_digit() {
            let si = i;
            while i < ab.len() && ab[i].is_ascii_digit() {
                i += 1;
            }
            let sj = j;
            while j < bb.len() && bb[j].is_ascii_digit() {
                j += 1;
            }
            // 先剔前导零再比：位数多的就是大数，省掉转 u64 溢出的麻烦。
            let (x, y) = (&a[si..i], &b[sj..j]);
            let (xn, yn) = (x.trim_start_matches('0'), y.trim_start_matches('0'));
            match xn.len().cmp(&yn.len()).then_with(|| xn.cmp(yn)) {
                Ordering::Equal => {}
                ord => return ord,
            }
        } else {
            let (ca, cb) = (ab[i].to_ascii_lowercase(), bb[j].to_ascii_lowercase());
            if ca != cb {
                return ca.cmp(&cb);
            }
            i += 1;
            j += 1;
        }
    }
    // 谁先走完谁短。
    (ab.len() - i).cmp(&(bb.len() - j))
}
