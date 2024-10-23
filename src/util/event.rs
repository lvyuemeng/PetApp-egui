use std::sync::{
    mpsc::{Receiver, Sender},
    Arc,
};

use anyhow::{anyhow, Result};
use eframe::egui::{self, mutex::Mutex};

use super::{
    item::{CatJSON, DogJSON, Pet, PetKind},
    model::{db_delete_pet, db_get_pet, db_get_pets, db_insert_pet, SqlCon},
};

const DOG_API: &str = "https://dog.ceo/api/breeds/image/random";
const CAT_API: &str = "https://api.thecatapi.com/v1/images/search";

impl eframe::App for PetApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.handle_egui();

        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_sidebar(ui, ctx);
        });
    }
}

pub struct PetApp {
    app_state: AppState,
    backend_event_sender: Sender<Event>,
    event_receiver: Receiver<Event>,
    db_con: SqlCon,
}
#[derive(Debug, Clone, Default)]
struct AppState {
    selected_pet: Option<Pet>,
    pets: Vec<Pet>,
    pet_image: Option<String>,
    add_form: AddForm,
}
#[derive(Debug, Clone, Default)]
struct AddForm {
    pub show: bool,
    name: String,
    age: String,
    kind: String,
}

impl PetApp {
    pub fn new(
        backend_event_sender: Sender<Event>,
        event_receiver: Receiver<Event>,
        db_con: sqlite::Connection,
    ) -> Result<Box<PetApp>> {
        let db_con = Arc::new(Mutex::new(db_con));
        let pets = db_get_pets(db_con.clone())?;
        Ok(Box::new(Self {
            app_state: AppState::new(pets),
            backend_event_sender,
            event_receiver,
            db_con,
        }))
    }

    fn render_sidebar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        egui::SidePanel::left("left panel")
            .resizable(false)
            .default_width(200.0)
            .show_inside(ui, |ui| {
                self.render_header(ui, ctx);
                ui.separator();
                self.render_pet_list(ui, ctx);
            });
    }

    fn render_header(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.vertical_centered(|ui| {
            ui.heading("Pets");
            ui.separator();
            if ui.button("Add new Pet").clicked() {
                self.app_state.add_form.show = !self.app_state.add_form.show;
            }
            if self.app_state.add_form.show {
                ui.separator();
                self.render_add_form(ui, ctx);
            }
        });
    }

    fn render_add_form(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.vertical_centered(|ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label("name:");
                    ui.label("age:");
                    ui.label("kind:");
                });
                ui.end_row();
                ui.vertical(|ui| {
                    ui.text_edit_singleline(&mut self.app_state.add_form.name);
                    ui.text_edit_singleline(&mut self.app_state.add_form.age);
                    ui.text_edit_singleline(&mut self.app_state.add_form.kind);
                });
            });
            if ui.button("Submit").clicked() {
                self.handle_add_pet_submission(ctx);
            }
        });
    }

    fn handle_add_pet_submission(&mut self, ctx: &egui::Context) {
        let add_form = &mut self.app_state.add_form;
        if let Ok(pet) = add_form.to_pet() {
            let kind = pet.clone().inner().3;
            let _ = self.backend_event_sender.send(Event::DBInsertPet(
                ctx.clone(),
                self.db_con.clone(),
                pet,
            ));
            let _ = self
                .backend_event_sender
                .send(Event::GetPetImage(ctx.clone(), kind));
            self.app_state.clear_add_form();
        }
    }

    fn render_pet_list(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        self.app_state.pets.iter().for_each(|pet| {
            let pet_clone = pet.clone().inner();
            if ui
                .selectable_value(
                    &mut self.app_state.selected_pet,
                    Some(pet.to_owned()),
                    pet_clone.1,
                )
                .changed()
            {
                let _ = self.backend_event_sender.send(Event::DBGetPet(
                    ctx.clone(),
                    self.db_con.clone(),
                    pet.id(),
                ));
                let _ = self
                    .backend_event_sender
                    .send(Event::GetPetImage(ctx.clone(), pet_clone.3));
            }
        });
    }

    pub fn handle_egui(&mut self) {
        while let Ok(event) = self.event_receiver.try_recv() {
            match event {
                Event::SetPetImage(pet_image) => self.app_state.pet_image = pet_image,
                Event::SetSelectedPet(pet) => self.app_state.selected_pet = pet,
                Event::SetPets(pets) => {
                    if let Some(ref selected_pet) = self.app_state.selected_pet {
                        if pets.iter().all(|p| p.id() != selected_pet.id()) {
                            self.app_state.selected_pet = None;
                        }
                    }
                    self.app_state.pets = pets
                }
                _ => (),
            }
        }
    }
}

impl AppState {
    fn new(pets: Vec<Pet>) -> Self {
        Self {
            pets,
            ..Default::default()
        }
    }

    fn update_pets(&mut self, pets: Vec<Pet>) {
        if let Some(ref selected_pet) = self.selected_pet {
            if pets.iter().all(|p| p.id() != selected_pet.id()) {
                self.selected_pet = None;
            }
        }
        self.pets = pets;
    }

    fn clear_add_form(&mut self) {
        self.add_form = AddForm::default();
    }
}

impl AddForm {
    fn inner(self) -> (String, String, String) {
        (self.name, self.age, self.kind)
    }
    fn to_pet(&self) -> Result<Pet> {
        let name = self.name.clone();
        let age = self
            .age
            .parse::<i64>()
            .map_err(|_| anyhow!("Invalid age"))?;
        let kind = match self.kind.as_str() {
            "cat" => "cat",
            _ => "dog",
        };
        Ok(Pet::new(-1, name, age, PetKind::new(String::from(kind))))
    }
}

pub struct EventHandler {
    event_sender: Sender<Event>,
    event_receiber: Receiver<Event>,
    db_con: SqlCon,
}

pub enum Event {
    SetPets(Vec<Pet>),
    GetPetImage(egui::Context, PetKind),
    SetPetImage(Option<String>),
    DBGetPet(egui::Context, SqlCon, i64), // Sql Connection; pet_id
    SetSelectedPet(Option<Pet>),
    DBInsertPet(egui::Context, SqlCon, Pet),
    DBDeletePet(egui::Context, SqlCon, i64), // Sql Connection, pet_id
}

impl Event {
    pub fn handle_op(self, sender: Sender<Event>) {
        match self {
            Event::GetPetImage(ctx, pet_kind) => {
                fetch_pet_image(ctx, pet_kind, sender);
            }
            Event::DBGetPet(ctx, db_con, pet_id) => {
                if let Ok(Some(pet)) = db_get_pet(db_con, pet_id) {
                    let _ = sender.send(Event::SetSelectedPet(Some(pet)));
                    ctx.request_repaint();
                }
            }
            Event::DBDeletePet(ctx, db_con, pet_id) => db_delete_pet(db_con.clone(), pet_id)
                .and_then(|_| {
                    if let Ok(pets) = db_get_pets(db_con) {
                        let _ = sender.send(Event::SetPets(pets));
                        ctx.request_repaint();
                    }
                    Ok(())
                })
                .unwrap_or_else(|_| ()),
            Event::DBInsertPet(ctx, db_con, pet) => db_insert_pet(db_con.clone(), pet)
                .and_then(|new_pet| {
                    if let Ok(pets) = db_get_pets(db_con) {
                        let _ = sender.send(Event::SetPets(pets));
                        let _ = sender.send(Event::SetSelectedPet(Some(new_pet)));
                        ctx.request_repaint();
                    }
                    Ok(())
                })
                .unwrap_or_else(|_| ()),
            _ => (),
        }
    }
}

pub fn fetch_pet_image(ctx: egui::Context, pet_kind: PetKind, sender: Sender<Event>) {
    let url = if pet_kind.inner() == "dog" {
        DOG_API
    } else {
        CAT_API
    };

    ehttp::fetch(ehttp::Request::get(url), move |res| {
        if let Ok(res) = res {
            let img_url = match pet_kind.inner() {
                inner if inner == "dog" => match res.json::<DogJSON>() {
                    Ok(json) => Some(json.inner()),
                    _ => None,
                },
                _ => match res.json::<CatJSON>() {
                    Ok(json) => Some(json.inner().url),
                    _ => None,
                },
            };

            let _ = sender.send(Event::SetPetImage(img_url));
            ctx.request_repaint();
        }
    });
}
