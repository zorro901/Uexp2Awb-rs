use std::fs::{self, File};
use std::io::{Read, Write, Seek, SeekFrom};
use std::path::{Path};
use std::env;
use std::ffi::OsStr;

// コピー処理: バッファを使って指定された部分をコピー
fn copypart(src: &Path, dest: &Path, start: u64, length: u64, bufsize: usize) -> std::io::Result<()> {
    let mut f1 = File::open(src)?;
    f1.seek(SeekFrom::Start(start))?;

    let mut f2 = File::create(dest)?;

    let mut remaining = length;
    let mut buffer = vec![0u8; bufsize];

    while remaining > 0 {
        let chunk = std::cmp::min(bufsize as u64, remaining) as usize;
        let bytes_read = f1.read(&mut buffer[..chunk])?;
        if bytes_read == 0 {
            break;
        }
        f2.write_all(&buffer[..bytes_read])?;
        remaining -= bytes_read as u64;
    }

    Ok(())
}

// マークを見つけてファイルを分割
fn split_uexp(root: &Path, file: &str, target_dir: &Path, output_root: &Path) -> std::io::Result<()> {
    let in_path = root.join(file);
    let mut f = File::open(&in_path)?;

    let mut s = Vec::new();
    f.read_to_end(&mut s)?;

    // AFS2
    let afs2pos = match s.windows(4).position(|window| window == b"AFS2") {
        Some(pos) => pos as u64,
        None => {
            return Ok(()); // AFS2が見つからなければ終了
        }
    };

    // @UTF
    let end_pos = match s.windows(4).position(|window| window == b"@UTF") {
        Some(pos) => pos as u64,
        None => {
            return Ok(()); // 終端が見つからなければ終了
        }
    };

    // target_dir の部分を出力パスから除去
    let relative_path = root.strip_prefix(target_dir).unwrap_or(root); // target_dir から相対パスを作成
    let out_dir = output_root.join(relative_path);
    fs::create_dir_all(&out_dir)?;

    // 拡張子を除去したファイル名を作成
    let file_stem = Path::new(file)
        .file_stem() // "Zitome.uexp" -> "Zitome"
        .and_then(OsStr::to_str)
        .unwrap_or(file);

    let out_file = out_dir.join(format!("{}.awb", file_stem));
    copypart(&in_path, &out_file, afs2pos, end_pos - afs2pos, 1024 * 1024)?;

    println!("created {}", out_file.display()); // ログに出力先ファイル名を出力

    Ok(())
}

// 対象ディレクトリのすべてのuexpファイルを検索し、同一構造で出力
fn process_directory(target_dir: &Path, output_root: &Path) -> std::io::Result<()> {
    for entry in fs::read_dir(&target_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // ディレクトリ内を再帰的に探索
            process_directory(&path, &output_root)?;
        } else if path.extension() == Some(OsStr::new("uexp")) {
            if let Some(file_name) = path.file_name() {
                if let Some(file_str) = file_name.to_str() {
                    split_uexp(&path.parent().unwrap_or_else(|| Path::new("")), file_str, &target_dir, &output_root)?;
                }
            }
        }
    }

    Ok(())
}

fn main() -> std::io::Result<()> {
    // コマンドライン引数から対象ディレクトリを取得
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <target_directory>", args[0]);
        return Ok(());
    }
    let target_dir = Path::new(&args[1]);

    // 出力ディレクトリ
    let output_root = env::current_dir()?.join("output_awb");

    // ディレクトリの処理
    process_directory(&target_dir, &output_root)?;

    println!("All files processed successfully.");
    Ok(())
}