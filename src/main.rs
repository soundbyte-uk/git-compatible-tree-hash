use std::{
    env::args,
    ffi::OsString,
    fmt::Write,
    fs,
    io,
    path::{Path, PathBuf},
    os::unix::fs::PermissionsExt,
};

use sha1::{Digest, Sha1};

const OBJECT_ID_BYTES: usize = 20;
type ObjectId = [u8; OBJECT_ID_BYTES];

static MODE_NORMAL: &str = "100644";
static MODE_EXECUTABLE: &str = "100755";
static MODE_DIR: &str = "40000";
static MODE_SYMLINK: &str = "120000";

fn file_hash(file_path: &Path) -> io::Result<ObjectId> {
    let mut hasher = Sha1::new();
    hasher.update(format!("blob {}", fs::metadata(file_path)?.len()));
    hasher.update([0]);
    hasher.update(fs::read(file_path)?);
    Ok(hasher.finalize().into())
}

enum TreeResult {
    Hash(ObjectId),
    Empty,
}

fn tree_hash(dir_path: &Path) -> io::Result<TreeResult> {
    let mut entries: Vec<(bool, Vec<u8>, &'static str, ObjectId)> = Vec::new();
    for entry in fs::read_dir(dir_path)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let md = entry.metadata()?;

        if file_name == ".git" || file_name == ".jj" || file_name == "target" {
            continue;
        }

        let (is_dir, mode_string) = if md.is_symlink() {
            (false, MODE_SYMLINK)
        } else if md.is_dir() {
            (true, MODE_DIR)
        } else if md.permissions().mode() & 0x01 > 0 {
            (false, MODE_EXECUTABLE)
        } else {
            (false, MODE_NORMAL)
        };

        let hash = if md.is_dir() {
            let mut subdir_path = dir_path.to_path_buf();
            subdir_path.push(file_name.clone());
            match tree_hash(&subdir_path)? {
                TreeResult::Empty => { continue; }
                TreeResult::Hash(hash) => hash
            }
        } else if md.is_file() {
            let mut subdir_path = dir_path.to_path_buf();
            subdir_path.push(file_name.clone());
            file_hash(&subdir_path)?
        } else if md.is_symlink() {
            let mut subdir_path = dir_path.to_path_buf();
            subdir_path.push(file_name.clone());
            let link_text = fs::read_link(subdir_path)?;
            let mut hasher = Sha1::new();
            hasher.update(format!("blob {}\0", link_text.clone().into_os_string().len()));
            hasher.update(link_text.into_os_string().as_encoded_bytes());
            hasher.finalize().into()
        } else {
            panic!("wtf is \"{:?}\"?", file_name);
        };

        entries.push((is_dir, normalized_bytes(file_name), mode_string, hash));
    }

    if entries.is_empty() {
        return Ok(TreeResult::Empty);
    }

    entries.sort_by_key(|(is_dir, name, mode_string, hash)|{
        if *is_dir {
            let mut name_slashed = name.clone();
            name_slashed.push(b'/');
            (name_slashed, *mode_string, hash.clone())
        } else {
            (name.clone(), *mode_string, hash.clone())
        }
    });

    let mut tree_len = 0;
    for (_is_dir, name, mode, _hash) in entries.iter() {
        //eprintln!("{:?} : {:?} {} {:x?}", &dir_path, &name, &mode, &hash);
        tree_len += mode.len() + 1 + name.len() + 1 + OBJECT_ID_BYTES;
    }
    //eprintln!("{:?} : len {:?}", &dir_path, tree_len);

    let mut hasher = Sha1::new();
    hasher.update(format!("tree {}", tree_len));
    hasher.update([0]);
    for (_is_dir, name, mode, hash) in entries {
        hasher.update(mode);
        hasher.update(" ");
        hasher.update(name);
        hasher.update([0]);
        hasher.update(hash);
    }

    Ok(TreeResult::Hash(hasher.finalize().into()))
}

fn normalized_bytes(str: OsString) -> Vec<u8> {
    static NFC: icu_normalizer::ComposingNormalizerBorrowed<'_>
        = icu_normalizer::ComposingNormalizerBorrowed::new_nfc();
    NFC.normalize_utf8(str.as_encoded_bytes()).as_bytes().to_vec()
}

fn main() -> io::Result<()> {
    let mut args = args();
    let _ = args.next();
    let dir_path = args.next().expect("path arg required");
    let dir_path: PathBuf = dir_path.into();

    let hash = match tree_hash(&dir_path)? {
        TreeResult::Empty => { panic!("Can't hash an empty tree"); }
        TreeResult::Hash(hash) => hash
    };
    let mut hex = String::with_capacity(40);
    for byte in hash {
        write!(hex, "{:02x}", byte).unwrap();
    }
    println!("{}", hex);
    Ok(())
}
