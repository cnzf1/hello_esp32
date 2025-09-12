use std::{env, fs, path::PathBuf};

fn main() {
    embuild::espidf::sysenv::output();

    copy_srmodels();
}

fn copy_srmodels() {
    // 如果 build 过程中有  srmodels.bin 生成，则将其拷贝到 对应的 target 目录下
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let target_triple = env::var("TARGET").unwrap();
    let profile = env::var("PROFILE").unwrap();

    let target_dir = manifest_dir
        .join("target")
        .join(&target_triple)
        .join(&profile);
    eprintln!("target_dir: {target_dir:?}");

    let target_build_dir = target_dir.join("build");
    eprintln!("target_build_dir: {target_build_dir:?}");

    // 查找名字以 esp-idf-sys- 开头的目录
    if let Some(srmodels_src) = fs::read_dir(target_build_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find_map(|p| {
            if p.file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("esp-idf-sys-")
            {
                let candidate = p
                    .join("out")
                    .join("build")
                    .join("srmodels")
                    .join("srmodels.bin");
                eprintln!("esp-idf-sys srmodels path: {candidate:?}");
                if candidate.exists() {
                    return Some(candidate);
                }
            }
            None
        })
    {
        let srmodels_dest = target_dir.join("srmodels.bin");
        fs::create_dir_all(srmodels_dest.parent().unwrap()).unwrap();
        fs::copy(&srmodels_src, &srmodels_dest).unwrap();
        eprintln!("esp-idf-sys srmodels dest: {srmodels_dest:?}");
    }
}
