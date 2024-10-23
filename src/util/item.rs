use serde::Deserialize;

#[derive(Debug, PartialEq, Clone)]
pub struct PetKind(String);

impl PetKind {
    pub fn new(kind: String) -> Self {
        PetKind(kind)
    }

    pub fn inner(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Pet {
    id: i64,
    name: String,
    age: i64,
    kind: PetKind,
}

impl Pet {
    pub fn new(id: i64, name: String, age: i64, kind: PetKind) -> Self {
        Pet {
            id,
            name,
            age,
            kind,
        }
    }

    pub fn inner(self) -> (i64, String, i64, PetKind) {
        (self.id, self.name, self.age, self.kind)
    }
    
    pub fn id(&self) -> i64 {
        self.id
    }
}
#[derive(Debug, Deserialize)]
pub struct CatJSON {
    #[serde(alias = "0")]
    item: CatJSONInner,
}

impl CatJSON {
    pub fn inner(self) -> CatJSONInner {
        self.item
    }
}
#[derive(Debug, Deserialize)]
pub struct CatJSONInner {
    pub url: String,
}
#[derive(Debug, Deserialize)]
pub struct DogJSON {
    message: String,
}

impl DogJSON {
    pub fn inner(self) -> String {
        self.message
    }
}
