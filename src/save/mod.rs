use crate::{
    save::update::Updater,
    util::{load_tga, Game, SResult},
};
#[cfg(target_arch = "wasm32")]
use ahash::HashMap;
use core::{
    erf::{self, Erf},
    gff::{self, Field, Gff, Struct},
    Data, DataDescr, GameDataMapped, Item as DItem, ReadResourceNoArg as _, ResourceKey,
};
use egui::{Context, TextureHandle, TextureOptions};
use macros::{EnumFromInt, EnumList, EnumToInt, EnumToString};
use std::{collections::VecDeque, fmt};
#[cfg(not(target_arch = "wasm32"))]
use std::{fs, path::PathBuf};

mod read;
mod update;

const GLOBALS_TYPES: &[&str] = &["Number", "Boolean"];
const NPC_RESOURCE_PREFIX: &str = "availnpc";

#[derive(Debug, Clone, PartialEq)]
pub struct PartyMember {
    pub idx: usize,
    pub leader: bool,
}
#[derive(Debug, Clone, PartialEq)]
pub struct AvailablePartyMember {
    pub available: bool,
    pub selectable: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JournalEntry {
    pub id: String,
    pub stage: i32,
    pub date: u32,
    pub time: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PartyTable {
    pub journal: Vec<JournalEntry>,
    pub cheat_used: bool,
    pub credits: u32,
    pub members: Vec<PartyMember>,
    pub available_members: Vec<AvailablePartyMember>,
    pub influence: Option<Vec<i32>>,
    pub party_xp: i32,
    pub components: Option<u32>,
    pub chemicals: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GlobalValue {
    Boolean(bool),
    Number(u8),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Global {
    pub name: String,
    pub value: GlobalValue,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Nfo {
    pub save_name: String,
    pub pc_name: Option<String>,
    pub area_name: String,
    pub last_module: String,
    pub cheat_used: bool,
    pub time_played: u32,
}

#[derive(EnumFromInt, EnumToInt, EnumToString, EnumList, Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum Gender {
    Male = 0,
    Female = 1,
    Both = 2,
    Other = 3,
    None = 4,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Class {
    pub id: i32,
    pub level: i16,
    pub powers: Option<Vec<u16>>, // IDs
}

const EQUIPMENT_SLOT_IDS: [u32; 12] = [
    0x00200, // Implant
    0x00001, // Head
    0x00008, // Gloves
    0x00080, // Arm Left
    0x00002, // Armor
    0x00100, // Arm Right
    0x00400, // Belt
    0x00010, // Main Hand
    0x00020, // Offhand
    0x40000, // Main Hand 2
    0x80000, // Offhand 2
    0x20000, // idk, some hidden slot
];

#[derive(Debug, Clone, PartialEq)]
pub struct Character {
    pub idx: usize,
    pub name: String,
    pub name_ref: u32,
    pub tag: String,
    pub hp: i16,
    pub hp_max: i16,
    pub fp: i16,
    pub fp_max: i16,
    pub invulnerable: bool,
    pub min_1_hp: bool,
    pub good_evil: u8,
    pub experience: u32,
    pub attributes: [u8; 6], // STR, DEX, CON, INT, WIS, CHA
    pub skills: [u8; 8],     // ranks in order of the skill menu
    pub feats: Vec<u16>,     // IDs
    pub classes: Vec<Class>,
    pub gender: Gender,
    pub portrait: u16,
    pub appearance: u16,
    pub soundset: u16,
    pub equipment: Box<[Option<Item>; 12]>,

    raw: Struct,
}

fn read_character_invulnerable(raw: &Struct) -> SResult<bool> {
    for field_name in ["Invulnerable", "Plot"] {
        let Some(field) = raw.fields.get(field_name) else {
            continue;
        };
        return Field::byte(field)
            .copied()
            .map(|value| value != 0)
            .ok_or_else(|| format!("invalid field {field_name}"));
    }

    Ok(false)
}

fn write_character_invulnerable(raw: &mut Struct, invulnerable: bool) {
    let value = Field::Byte(invulnerable as u8);
    raw.insert("Plot", value.clone());
    if raw.fields.contains_key("Invulnerable") {
        raw.insert("Invulnerable", value);
    }
}

impl Character {
    pub fn get_name(&self) -> &str {
        if self.name.is_empty() {
            &self.tag
        } else {
            &self.name
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub tag: String,
    pub base_item: i32,
    pub name: Option<String>,
    pub description: Option<String>,
    pub stack_size: u16,
    pub max_charges: u8,
    pub charges: u8,
    pub new: bool,
    pub upgrades: u32,                   // in K1
    pub upgrade_slots: Option<[i32; 6]>, // in K2

    pub raw: Struct,
}

impl Data<String> for Item {
    fn get_id(&self) -> &String {
        &self.tag
    }

    fn get_name(&self) -> &str {
        if let Some(name) = &self.name {
            name
        } else {
            &self.tag
        }
    }
}

impl From<&DItem> for Item {
    fn from(item: &DItem) -> Self {
        Self {
            tag: item.tag.clone(),
            base_item: item.base_item,
            name: item.name.clone(),
            description: item.description.clone(),
            stack_size: item.stack_size,
            max_charges: item.charges,
            charges: item.charges,
            new: true,
            upgrades: 0,
            upgrade_slots: item.upgrade_level.map(|_| [-1; 6]),

            raw: item.raw.clone(),
        }
    }
}

impl DataDescr for Item {
    fn get_description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Door {
    pub tag: String,
    pub locked: bool,
    pub open_state: u8,

    raw: Struct,
}

#[derive(Debug, Clone, PartialEq)]
struct SaveInternals {
    nfo: Gff,
    globals: Gff,
    party_table: Gff,
    erf: Erf,
    pifo: Option<Gff>,

    git_key: Option<ResourceKey>,
    use_pifo: bool,
}

#[derive(Clone, PartialEq)]
pub struct Save {
    pub id: u64,
    pub game: Game,
    pub globals: Vec<Global>,
    pub nfo: Nfo,
    pub party_table: PartyTable,
    pub image: Option<TextureHandle>,
    pub characters: Vec<Character>,
    pub inventory: Vec<Item>,
    pub doors: Option<Vec<Door>>,

    inner: SaveInternals,
}

// TextureHandle doesn't implement Debug and there isn't much point to printing internals
impl fmt::Debug for Save {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Save")
            .field("game", &self.game)
            .field("globals", &self.globals)
            .field("nfo", &self.nfo)
            .field("party_table", &self.party_table)
            .finish_non_exhaustive()
    }
}

const GFFS: &[(bool, &str)] = &[
    (true, "savenfo.res"),
    (true, "globalvars.res"),
    (true, "partytable.res"),
    (false, "pifo.ifo"),
];
const ERF_NAME: &str = "savegame.sav";
const IMAGE_NAME: &str = "screen.tga";

impl Save {
    fn read(mut gffs: VecDeque<Gff>, erf: Erf, image: Option<TextureHandle>) -> SResult<Save> {
        let reader = read::Reader::new(
            gffs.pop_front().unwrap(),
            gffs.pop_front().unwrap(),
            gffs.pop_front().unwrap(),
            erf,
            gffs.pop_front(),
            image,
        );

        reader
            .into_save()
            .map_err(|err| format!("Save::read| {err}"))
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::util::load_default_game_data;
    use core::GameData;
    use std::{env, fs, path::Path};

    fn copy_save(source: &Path, destination: &Path) {
        fs::create_dir_all(destination).unwrap();
        for entry in fs::read_dir(source).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_file() {
                fs::copy(entry.path(), destination.join(entry.file_name())).unwrap();
            }
        }
    }

    fn changed_u8(value: u8) -> u8 {
        if value == u8::MAX {
            value - 1
        } else {
            value + 1
        }
    }

    #[test]
    fn character_invulnerability_uses_engine_field_precedence() {
        let plot_only = Struct::new(vec![("Plot", Field::Byte(1))]);
        assert!(read_character_invulnerable(&plot_only).unwrap());

        let both = Struct::new(vec![
            ("Plot", Field::Byte(1)),
            ("Invulnerable", Field::Byte(0)),
        ]);
        assert!(!read_character_invulnerable(&both).unwrap());

        assert!(!read_character_invulnerable(&Struct::new(vec![])).unwrap());
    }

    #[test]
    fn character_invulnerability_writes_plot_and_syncs_existing_alias() {
        let mut plot_only = Struct::new(vec![("Plot", Field::Byte(1))]);
        write_character_invulnerable(&mut plot_only, false);
        assert_eq!(plot_only.fields.get("Plot"), Some(&Field::Byte(0)));
        assert!(!plot_only.fields.contains_key("Invulnerable"));

        let mut both = Struct::new(vec![
            ("Plot", Field::Byte(0)),
            ("Invulnerable", Field::Byte(0)),
        ]);
        write_character_invulnerable(&mut both, true);
        assert_eq!(both.fields.get("Plot"), Some(&Field::Byte(1)));
        assert_eq!(both.fields.get("Invulnerable"), Some(&Field::Byte(1)));
    }

    #[test]
    fn supplied_save_round_trip_applies_only_requested_model_edits() {
        let Ok(source) = env::var("SOTOR_TEST_SAVE") else {
            eprintln!("SOTOR_TEST_SAVE is not set; skipping supplied-save round-trip test");
            return;
        };

        let test_dir = env::temp_dir().join(format!("sotor-roundtrip-{}", fastrand::u64(..)));
        copy_save(Path::new(&source), &test_dir);
        let test_path = test_dir.to_string_lossy().into_owned();
        let ctx = Context::default();
        let mut save = Save::read_from_directory(&test_path, &ctx).unwrap();
        assert_eq!(save.game, Game::One);
        assert!(!save.characters.is_empty());

        let baseline_area = save.nfo.area_name.clone();
        let baseline_module = save.nfo.last_module.clone();
        let baseline_character_count = save.characters.len();
        let baseline_inventory_count = save.inventory.len();

        save.nfo.save_name.push_str(" [accessibility round trip]");
        let expected_name = save.nfo.save_name.clone();
        save.party_table.credits = save.party_table.credits.saturating_add(1);
        let expected_credits = save.party_table.credits;
        save.party_table.party_xp = save.party_table.party_xp.saturating_add(1);
        let expected_xp = save.party_table.party_xp;

        let bool_global = save.globals.iter_mut().find_map(|global| {
            if let GlobalValue::Boolean(value) = &mut global.value {
                *value = !*value;
                Some((global.name.clone(), *value))
            } else {
                None
            }
        });
        let number_global = save.globals.iter_mut().find_map(|global| {
            if let GlobalValue::Number(value) = &mut global.value {
                *value = changed_u8(*value);
                Some((global.name.clone(), *value))
            } else {
                None
            }
        });

        let character = &mut save.characters[0];
        character.experience = character.experience.saturating_add(1);
        let expected_experience = character.experience;
        character.attributes[0] = changed_u8(character.attributes[0]);
        let expected_strength = character.attributes[0];
        character.skills[0] = changed_u8(character.skills[0]);
        let expected_computer_use = character.skills[0];

        let game_data: [GameDataMapped; Game::COUNT] = load_default_game_data().map(GameData::into);
        let data = &game_data[save.game.idx()];
        let added_feat = data
            .inner
            .feats
            .iter()
            .find(|feat| !character.feats.contains(&feat.id))
            .map(|feat| feat.id)
            .unwrap();
        character.feats.push(added_feat);

        let inventory_edit = save.inventory.first_mut().map(|item| {
            item.stack_size = if item.stack_size >= 99 {
                item.stack_size - 1
            } else {
                item.stack_size + 1
            };
            (item.tag.clone(), item.stack_size)
        });
        let door_edit = save.doors.as_mut().and_then(|doors| {
            doors.first_mut().map(|door| {
                door.locked = !door.locked;
                (door.tag.clone(), door.locked)
            })
        });

        Save::save_to_directory(&test_path, &mut save, data).unwrap();
        assert!(test_dir.join("backup.zip").is_file());

        let reloaded = Save::read_from_directory(&test_path, &ctx).unwrap();
        assert_eq!(reloaded.nfo.save_name, expected_name);
        assert_eq!(reloaded.nfo.area_name, baseline_area);
        assert_eq!(reloaded.nfo.last_module, baseline_module);
        assert_eq!(reloaded.party_table.credits, expected_credits);
        assert_eq!(reloaded.party_table.party_xp, expected_xp);
        assert_eq!(reloaded.characters.len(), baseline_character_count);
        assert_eq!(reloaded.inventory.len(), baseline_inventory_count);
        assert_eq!(reloaded.characters[0].experience, expected_experience);
        assert_eq!(reloaded.characters[0].attributes[0], expected_strength);
        assert_eq!(reloaded.characters[0].skills[0], expected_computer_use);
        assert!(reloaded.characters[0].feats.contains(&added_feat));

        if let Some((name, expected)) = bool_global {
            assert_eq!(
                reloaded
                    .globals
                    .iter()
                    .find(|g| g.name == name)
                    .unwrap()
                    .value,
                GlobalValue::Boolean(expected)
            );
        }
        if let Some((name, expected)) = number_global {
            assert_eq!(
                reloaded
                    .globals
                    .iter()
                    .find(|g| g.name == name)
                    .unwrap()
                    .value,
                GlobalValue::Number(expected)
            );
        }
        if let Some((tag, expected)) = inventory_edit {
            assert_eq!(
                reloaded
                    .inventory
                    .iter()
                    .find(|i| i.tag == tag)
                    .unwrap()
                    .stack_size,
                expected
            );
        }
        if let Some((tag, expected)) = door_edit {
            assert_eq!(
                reloaded
                    .doors
                    .as_ref()
                    .unwrap()
                    .iter()
                    .find(|d| d.tag == tag)
                    .unwrap()
                    .locked,
                expected
            );
        }

        fs::remove_dir_all(test_dir).unwrap();
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Save {
    pub fn read_from_directory(path: &str, ctx: &Context) -> SResult<Self> {
        use crate::util::read_file;
        // ERF
        let erf_bytes = read_file(path, ERF_NAME)
            .map_err(|err| format!("couldn't read ERF file {ERF_NAME}: {err}"))?;
        let erf = Erf::read(&erf_bytes)?;

        // GFFs
        let mut gffs = VecDeque::with_capacity(GFFS.len());
        for (required, name) in GFFS {
            match read_file(path, name) {
                Ok(file) => {
                    gffs.push_back(
                        Gff::read(&file)
                            .map_err(|err| format!("couldn't read GFF {name}: {err}"))?,
                    );
                }
                Err(e) => {
                    if *required {
                        return Err(format!("couldn't read GFF file {name}: {e}"));
                    }
                }
            }
        }
        // autosaves don't have screenshots
        let image = read_file(path, IMAGE_NAME)
            .ok()
            .and_then(|file| load_tga(&file).ok())
            .map(|tga| ctx.load_texture("save_image", tga, TextureOptions::NEAREST));

        Self::read(gffs, erf, image)
    }

    pub fn save_to_directory(
        path: &str,
        save: &mut Save,
        data: &GameDataMapped,
    ) -> crate::util::ESResult {
        use crate::util::{add_zip_file, read_dir_filemap};

        Updater::new(save, data).update();
        let file_names = read_dir_filemap(&path.into())
            .map_err(|err| format!("couldn't read dir {path}: {err}"))?;
        let backup_path = PathBuf::from_iter([path, "backup.zip"]);
        let backup_handle = &mut fs::File::options()
            .write(true)
            .create(true)
            .truncate(true)
            .open(backup_path)
            .unwrap();
        let mut backup = zip::ZipWriter::new(backup_handle);

        for ((_, name), gff) in GFFS.iter().zip([
            Some(&save.inner.nfo),
            Some(&save.inner.globals),
            Some(&save.inner.party_table),
            save.inner.pifo.as_ref(),
        ]) {
            let Some(gff) = gff else {
                continue;
            };
            let gff_name = file_names.get(*name).map_or(*name, |n| n.as_str());
            let full_path = PathBuf::from_iter([path, gff_name]);
            add_zip_file(&full_path, &mut backup).ok();

            let bytes = gff::write(gff.clone());
            fs::write(full_path, &bytes)
                .map_err(|err| format!("Couldn't write GFF file {name}: {err}"))?;
        }

        let erf_name = file_names.get(ERF_NAME).map_or(ERF_NAME, |n| n.as_str());
        let full_path = PathBuf::from_iter([path, erf_name]);
        add_zip_file(&full_path, &mut backup).ok();
        let bytes = erf::write(save.inner.erf.clone());

        fs::write(full_path, bytes).map_err(|err| format!("couldn't write ERF file: {err}"))?;

        Ok(())
    }
}

#[cfg(target_arch = "wasm32")]
impl Save {
    pub fn read_from_files(files: &HashMap<String, Vec<u8>>, ctx: &Context) -> SResult<Save> {
        // ERF
        let erf_bytes = files
            .get(ERF_NAME)
            .ok_or(format!("Couldn't find ERF file {ERF_NAME}"))?;
        let erf = Erf::read(erf_bytes)?;

        // GFFs
        let mut gffs = VecDeque::with_capacity(GFFS.len());
        for (required, name) in GFFS {
            let Some(bytes) = files.get(*name) else {
                if *required {
                    return Err(format!("couldn't find GFF file {name}"));
                }
                continue;
            };

            gffs.push_back(Gff::read(bytes)?);
        }
        let image = files
            .get(IMAGE_NAME)
            .and_then(|bytes| load_tga(bytes).ok())
            .map(|tga| ctx.load_texture("save_image", tga, TextureOptions::NEAREST));

        Self::read(gffs, erf, image)
    }

    pub fn save_to_zip(save: &mut Self, data: &GameDataMapped) -> Vec<u8> {
        use std::io::{Cursor, Write};

        Updater::new(save, data).update();
        let buf = vec![];
        let mut zip = zip::ZipWriter::new(Cursor::new(buf));
        let options =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        for ((_, name), gff) in GFFS.iter().zip([
            Some(&save.inner.nfo),
            Some(&save.inner.globals),
            Some(&save.inner.party_table),
            save.inner.pifo.as_ref(),
        ]) {
            let Some(gff) = gff else {
                continue;
            };
            let bytes = gff::write(gff.clone());

            zip.start_file(*name, options).unwrap();
            zip.write_all(&bytes).unwrap();
        }
        let erf_bytes = erf::write(save.inner.erf.clone());
        zip.start_file(ERF_NAME, options).unwrap();
        zip.write_all(&erf_bytes).unwrap();

        zip.finish().unwrap().into_inner()
    }

    pub fn reload(self) -> Self {
        let mut gffs =
            VecDeque::from_iter([self.inner.nfo, self.inner.globals, self.inner.party_table]);
        if let Some(pifo) = self.inner.pifo {
            gffs.push_back(pifo);
        };
        Self::read(gffs, self.inner.erf, self.image).unwrap()
    }
}
