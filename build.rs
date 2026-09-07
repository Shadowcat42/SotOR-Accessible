use core::{util::Game, GameData};
use std::{env::var, fs, io::Write, path::PathBuf};

fn main() {
    dotenv::dotenv().ok();
    let out_dir = var("OUT_DIR").unwrap();
    let output_path = PathBuf::from_iter([&out_dir, "assets.zip"]);

    // Release builds can reuse a previously generated asset bundle. This keeps
    // reproducible accessibility builds independent of a local game install.
    if let Ok(asset_bundle) = var("SOTOR_ASSETS_ZIP") {
        println!("cargo:rerun-if-env-changed=SOTOR_ASSETS_ZIP");
        println!("cargo:rerun-if-changed={asset_bundle}");
        fs::copy(asset_bundle, output_path).unwrap();
        return;
    }

    let out = &mut fs::File::options()
        .write(true)
        .create(true)
        .truncate(true)
        .open(output_path)
        .unwrap();
    let mut zip = zip::ZipWriter::new(out);
    let method = zip::CompressionMethod::Deflated;
    let options = zip::write::FileOptions::default().compression_method(method);

    zip.start_file("gamedata.bin", options).unwrap();
    let steam_dir: PathBuf = var("STEAM_APPS").unwrap().into();
    let mut common = steam_dir.clone();
    common.push("common");
    let game_dirs = Game::LIST.map(|game| {
        let mut dir = common.clone();
        dir.push(game.steam_dir());
        assert!(dir.exists(), "game directory missing at {dir:#?}");
        dir
    });
    let game_data = Game::LIST
        .map(|game| GameData::read(game, &game_dirs[game.idx()], Some(&steam_dir)).unwrap());
    bincode::serialize_into(&mut zip, &game_data).unwrap();

    zip.start_file("icons.ttf", options).unwrap();
    let icons = fs::read(PathBuf::from_iter(["assets", "fa-solid-900.ttf"])).unwrap();
    zip.write_all(&icons).unwrap();

    zip.start_file("font.ttf", options).unwrap();
    let font = fs::read(PathBuf::from_iter(["assets", "Roboto-Medium.ttf"])).unwrap();
    zip.write_all(&font).unwrap();

    zip.finish().unwrap();
}
