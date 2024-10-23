use anyhow::{anyhow, Ok, Result};
use eframe::egui::mutex::Mutex;
use std::{fs, sync::Arc};

use super::item::{Pet, PetKind};
const GET_PET_BY_ID: &str = "SELECT id, name, age, kind FROM pets where id = ?";
const DELETE_PET_BY_ID: &str = "DELETE FROM pets WHERE id = ?";
const INSERT_PET: &str =
    "INSERT INTO pets (name, age, kind) VALUES (?, ?, ?) RETURNING id, name, age, kind";
const GET_PETS: &str = "SELECT id, name, age, kind FROM pets";

pub type SqlCon = Arc<Mutex<sqlite::Connection>>;
pub fn init_sql() -> std::io::Result<String> {
    fs::read_to_string("./init.sql")
}
pub fn db_insert_pet(db_con: SqlCon, pet: Pet) -> Result<Pet> {
    let con = db_con.lock();
    let mut stmt = con.prepare(INSERT_PET)?;

    let (_, name, age, kind) = pet.inner();
    stmt.bind((1, name.as_str()))?;
    stmt.bind((2, age))?;
    stmt.bind((3, kind.inner()))?;

    match stmt.next()? {
        sqlite::State::Row => Ok(Pet::new(
            stmt.read::<i64, _>(0)?,
            stmt.read::<String, _>(1)?,
            stmt.read::<i64, _>(2)?,
            PetKind::new(stmt.read::<String, _>(3)?),
        )),
        _ => Err(anyhow!("error while inserting pet")),
    }
}

pub fn db_delete_pet(db_con: SqlCon, pet_id: i64) -> Result<()> {
    let con = db_con.lock();
    let mut stmt = con.prepare(DELETE_PET_BY_ID)?;
    stmt.bind((1, pet_id))?;

    match stmt.next()? {
        sqlite::State::Done => Ok(()),
        _ => Err(anyhow!("error while delete pet by id {}", pet_id)),
    }
}

pub fn db_get_pet(db_con: SqlCon, pet_id: i64) -> Result<Option<Pet>> {
    let con = db_con.lock();
    let mut stmt = con.prepare(GET_PET_BY_ID)?;

    stmt.bind((1, pet_id))?;

    match stmt.next()? {
        sqlite::State::Row => {
            let pet = Pet::new(
                stmt.read::<i64, _>(0)?,
                stmt.read::<String, _>(1)?,
                stmt.read::<i64, _>(2)?,
                PetKind::new(stmt.read::<String, _>(3)?),
            );
            Ok(Some(pet))
        }
        _ => Err(anyhow!("error while get pet by id {}", pet_id)),
    }
}

pub fn db_get_pets(db_con: SqlCon) -> Result<Vec<Pet>> {
    let con = db_con.lock();
    let mut stmt = con.prepare(GET_PETS)?;

    stmt.iter()
        .map(|row| {
            let row = row?;
            let pet = Pet::new(
                row.read::<i64, _>(0),
                row.read::<&str, _>(1).to_owned(),
                row.read::<i64, _>(2),
                PetKind::new(row.read::<&str, _>(3).to_owned()),
            );
            Ok(pet)
        })
        .collect()
}
