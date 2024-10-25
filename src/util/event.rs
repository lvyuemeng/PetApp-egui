use std::sync::mpsc::{Receiver, Sender};

use anyhow::{anyhow, Result};
use eframe::egui::{self};

use super::{
    item::{CatJSON, DogJSON, Pet, PetKind},
    model::{db_delete_pet, db_get_pet, db_get_pets, db_insert_pet, SqlCon},
};

const DOG_API: &str = "https://dog.ceo/api/breeds/image/random";
const CAT_API: &str = "https://api.thecatapi.com/v1/images/search";

impl eframe::App for PetApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.render_handler.handle_stream();

        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_sidebar(ui, ctx);
            egui::CentralPanel::default().show_inside(ui, |ui| {
                self.render_detail(ui, ctx);
            })
        });
    }
}

pub struct PetApp {
    render_handler: Handler<BackendEvent, RenderEvent, AppState>,
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
    show: bool,
    name: String,
    age: String,
    kind: String,
}

impl PetApp {
    pub fn new(
        backend_sender: Sender<BackendEvent>,
        render_receiver: Receiver<RenderEvent>,
    ) -> Box<PetApp> {
        let app_state = AppState::default();
        let handler = Handler::new(backend_sender, render_receiver, app_state);
        Box::new(PetApp {
            render_handler: handler,
        })
    }

    fn state(&mut self) -> &mut AppState {
        &mut self.render_handler.state
    }

    fn send(&self, event: BackendEvent) {
        let _ = self.render_handler.sender.send(event);
    }

    fn ui_label_column(ui: &mut egui::Ui, labels: &[&str]) {
        ui.vertical(|ui| {
            for label in labels {
                ui.label(*label);
            }
        });
    }

    fn ui_label_info_column(
        ui: &mut egui::Ui,
        labels: &[&str],
        content: impl FnOnce(&mut egui::Ui),
    ) {
        ui.horizontal(|ui| {
            Self::ui_label_column(ui, labels);
            ui.end_row();
            ui.vertical(|ui| {
                content(ui);
            });
        });
    }

    // Sidebar
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

    // Sidebar -> Header
    fn render_header(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.vertical_centered(|ui| {
            ui.heading("Pets");
            ui.separator();
            if ui.button("Add new Pet").clicked() {
                self.state().add_form.show = !self.state().add_form.show;
            }
            if self.state().add_form.show {
                ui.separator();
                self.render_add_form(ui, ctx);
            }
        });
    }

    // Sidebar -> AddForm
    fn render_add_form(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.vertical_centered(|ui| {
            let add_form = &mut self.state().add_form;
            Self::ui_label_info_column(ui, &["name:", "age:", "kind:"], |ui| {
                ui.text_edit_singleline(&mut add_form.name);
                ui.text_edit_singleline(&mut add_form.age);
                ui.text_edit_singleline(&mut add_form.kind);
            });
            if ui.button("Submit").clicked() {
                self.handle_add_pet_submission(ctx);
            }
        });
    }

    fn handle_add_pet_submission(&mut self, ctx: &egui::Context) {
        let add_form = &mut self.state().add_form;
        if let Ok(pet) = add_form.to_pet() {
            let pet_clone = pet.clone().inner();
            let _ = self.send(BackendEvent::InsertPetDB(ctx.clone(), pet));
            let _ = self.send(BackendEvent::FetchPetImage(ctx.clone(), pet_clone.3));
            self.state().clear_add_form();
        }
    }

    // Sidebar -> PetList
    fn render_pet_list(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let state = &mut self.render_handler.state;
        let sender = &self.render_handler.sender;

        state.pets.iter().for_each(|pet| {
            let pet_clone = pet.clone().inner();
            if ui
                .selectable_value(&mut state.selected_pet, Some(pet.to_owned()), pet_clone.1)
                .changed()
            {
                let _ = sender.send(BackendEvent::GetPetDB(ctx.clone(), pet.id()));
                let _ = sender.send(BackendEvent::FetchPetImage(ctx.clone(), pet_clone.3));
            }
        });
    }

    // Details
    fn render_detail(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let selected_pet = &self.render_handler.state.selected_pet;
        let pet_img = &self.render_handler.state.pet_image;
        ui.vertical_centered(|ui| {
            ui.heading("Details");
            if let Some(pet) = selected_pet {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        if ui.button("Delete").clicked() {
                            let _ = self.send(BackendEvent::DeletePetDB(ctx.clone(), pet.id()));
                        }
                    })
                });
                ui.separator();
                let pet_clone = pet.clone().inner();
                ui.vertical(|ui| {
                    Self::ui_label_info_column(ui, &["name:", "age:", "kind"], |ui| {
                        ui.label(pet_clone.1);
                        ui.label(pet_clone.2.to_string());
                        ui.label(pet_clone.3.inner());
                    });
                    ui.separator();
                    if let Some(ref pet_img) = pet_img {
                        ui.add(egui::Image::from_uri(pet_img).max_width(200.0));
                    }
                });
            } else {
                ui.label("No pet selected.");
            }
        });
    }
}

impl AppState {
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

// Handle backend, send request of render side
pub enum BackendEvent {
    // ctx for repaint
    FetchPetImage(egui::Context, PetKind),
    GetPetDB(egui::Context, i64),
    InsertPetDB(egui::Context, Pet),
    DeletePetDB(egui::Context, i64),
}

// Handle render side, send request to backend
pub enum RenderEvent {
    SetPets(Vec<Pet>),
    SetPetImage(Option<String>),
    SetSelectedPet(Option<Pet>),
}

// T for Sender, U for receiver
pub trait EventHandle {
    type RecvEvent;
    fn handle(&mut self, recv_event: Self::RecvEvent);
    fn handle_stream(&mut self);
}

pub struct Handler<T, U, C> {
    sender: Sender<T>,
    receiver: Receiver<U>,
    state: C,
}

impl<T, U, C> Handler<T, U, C> {
    pub fn new(sender: Sender<T>, receiver: Receiver<U>, state: C) -> Handler<T, U, C> {
        Handler {
            sender,
            receiver,
            state,
        }
    }
}

// Render Handler
impl EventHandle for Handler<BackendEvent, RenderEvent, AppState> {
    type RecvEvent = RenderEvent;
    fn handle(&mut self, event: RenderEvent) {
        match event {
            RenderEvent::SetPets(pets) => {
                // Update the app state with the new pet list
                self.state.update_pets(pets);
            }
            RenderEvent::SetPetImage(pet_image) => {
                // Update the displayed pet image
                self.state.pet_image = pet_image;
            }
            RenderEvent::SetSelectedPet(pet) => {
                // Update the selected pet details
                self.state.selected_pet = pet;
            }
        }
    }

    fn handle_stream(&mut self) {
        while let Ok(event) = self.receiver.try_recv() {
            self.handle(event);
        }
    }
}

// Backend Handler
impl EventHandle for Handler<RenderEvent, BackendEvent, SqlCon> {
    type RecvEvent = BackendEvent;
    fn handle(&mut self, event: BackendEvent) {
        match event {
            BackendEvent::FetchPetImage(ctx, pet_kind) => {
                fetch_pet_image(ctx, pet_kind, self.sender.clone());
            }
            BackendEvent::GetPetDB(ctx, pet_id) => {
                if let Ok(Some(pet)) = db_get_pet(self.state.clone(), pet_id) {
                    let _ = self.sender.send(RenderEvent::SetSelectedPet(Some(pet)));
                    ctx.request_repaint();
                }
            }
            BackendEvent::DeletePetDB(ctx, pet_id) => {
                db_delete_pet(self.state.clone(), pet_id)
                    .and_then(|_| {
                        if let Ok(pets) = db_get_pets(self.state.clone()) {
                            let _ = self.sender.send(RenderEvent::SetPets(pets));
                            ctx.request_repaint();
                        }
                        Ok(())
                    })
                    .unwrap_or_else(|_| ());
            }
            BackendEvent::InsertPetDB(ctx, pet) => {
                db_insert_pet(self.state.clone(), pet)
                    .and_then(|new_pet| {
                        if let Ok(pets) = db_get_pets(self.state.clone()) {
                            let _ = self.sender.send(RenderEvent::SetPets(pets));
                            let _ = self.sender.send(RenderEvent::SetSelectedPet(Some(new_pet)));
                            ctx.request_repaint();
                        }
                        Ok(())
                    })
                    .unwrap_or_else(|_| ());
            }
        }
    }
    fn handle_stream(&mut self) {
        if let Ok(pets) = db_get_pets(self.state.clone()) {
            let _ = self.sender.send(RenderEvent::SetPets(pets));
        }

        while let Ok(event) = self.receiver.recv() {
            self.handle(event);
        }
    }
}

fn fetch_pet_image(ctx: egui::Context, pet_kind: PetKind, sender: Sender<RenderEvent>) {
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

            let _ = sender.send(RenderEvent::SetPetImage(img_url));
            ctx.request_repaint();
        }
    });
}
